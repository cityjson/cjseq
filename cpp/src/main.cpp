#include "cjseq/cityjson.hpp"

#include <algorithm>
#include <array>
#include <chrono>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <optional>
#include <random>
#include <sstream>
#include <stdexcept>
#include <string>
#include <string_view>
#include <unordered_map>
#include <vector>

namespace {

using Json = cjseq::JsonValue;

std::string read_file(const std::filesystem::path &path) {
  std::ifstream input(path, std::ios::binary);
  if (!input.is_open()) {
    throw std::runtime_error("Failed to open file: " + path.string());
  }
  std::ostringstream buffer;
  buffer << input.rdbuf();
  return buffer.str();
}

std::string read_stdin() {
  std::ostringstream buffer;
  buffer << std::cin.rdbuf();
  return buffer.str();
}

std::vector<std::string> read_lines(std::istream &stream) {
  std::vector<std::string> lines;
  std::string line;
  while (std::getline(stream, line)) {
    if (!line.empty() && line.back() == '\r') {
      line.pop_back();
    }
    lines.push_back(line);
  }
  return lines;
}

std::string geometry_type_to_string(cjseq::GeometryType type) {
  switch (type) {
  case cjseq::GeometryType::MultiPoint:
    return "MultiPoint";
  case cjseq::GeometryType::MultiLineString:
    return "MultiLineString";
  case cjseq::GeometryType::MultiSurface:
    return "MultiSurface";
  case cjseq::GeometryType::CompositeSurface:
    return "CompositeSurface";
  case cjseq::GeometryType::Solid:
    return "Solid";
  case cjseq::GeometryType::MultiSolid:
    return "MultiSolid";
  case cjseq::GeometryType::CompositeSolid:
    return "CompositeSolid";
  case cjseq::GeometryType::GeometryInstance:
    return "GeometryInstance";
  }
  return "Unknown";
}

Json transform_to_json(const cjseq::Transform &transform) {
  Json j;
  j["scale"] = transform.scale;
  j["translate"] = transform.translate;
  return j;
}

Json material_to_json(const cjseq::Material &material) {
  Json j = Json::object();
  if (material.values) {
    j["values"] = *material.values;
  }
  if (material.value) {
    j["value"] = static_cast<std::uint64_t>(*material.value);
  }
  return j;
}

Json texture_to_json(const cjseq::Texture &texture) {
  Json j = Json::object();
  if (texture.values) {
    j["values"] = *texture.values;
  }
  return j;
}

Json appearance_to_json(const cjseq::Appearance &appearance) {
  Json j = Json::object();
  if (appearance.materials) {
    j["materials"] = *appearance.materials;
  }
  if (appearance.textures) {
    j["textures"] = *appearance.textures;
  }
  if (appearance.vertices_texture) {
    j["vertices-texture"] = *appearance.vertices_texture;
  }
  if (appearance.default_theme_texture) {
    j["default-theme-texture"] = *appearance.default_theme_texture;
  }
  if (appearance.default_theme_material) {
    j["default-theme-material"] = *appearance.default_theme_material;
  }
  return j;
}

Json geometry_to_json(const cjseq::Geometry &geometry) {
  Json j = Json::object();
  j["type"] = geometry_type_to_string(geometry.type);
  if (geometry.lod) {
    j["lod"] = *geometry.lod;
  }
  j["boundaries"] = geometry.boundaries;
  if (geometry.semantics) {
    j["semantics"] = *geometry.semantics;
  }
  if (geometry.material) {
    Json materials = Json::object();
    for (const auto &[key, value] : *geometry.material) {
      materials[key] = material_to_json(value);
    }
    j["material"] = std::move(materials);
  }
  if (geometry.texture) {
    Json textures = Json::object();
    for (const auto &[key, value] : *geometry.texture) {
      textures[key] = texture_to_json(value);
    }
    j["texture"] = std::move(textures);
  }
  if (geometry.template_index) {
    j["template"] = static_cast<std::uint64_t>(*geometry.template_index);
  }
  if (geometry.transformation_matrix) {
    j["transformationMatrix"] = *geometry.transformation_matrix;
  }
  return j;
}

Json point_of_contact_to_json(const cjseq::PointOfContact &poc) {
  Json j = Json::object();
  j["contactName"] = poc.contact_name;
  if (poc.contact_type) {
    j["contactType"] = *poc.contact_type;
  }
  if (poc.role) {
    j["role"] = *poc.role;
  }
  if (poc.phone) {
    j["phone"] = *poc.phone;
  }
  j["emailAddress"] = poc.email_address;
  if (poc.website) {
    j["website"] = *poc.website;
  }
  if (poc.address) {
    Json addr = Json::object();
    addr["thoroughfareNumber"] = poc.address->thoroughfare_number;
    addr["thoroughfareName"] = poc.address->thoroughfare_name;
    addr["locality"] = poc.address->locality;
    addr["postalCode"] = poc.address->postal_code;
    addr["country"] = poc.address->country;
    j["address"] = std::move(addr);
  }
  return j;
}

Json metadata_to_json(const cjseq::Metadata &metadata) {
  Json j = Json::object();
  if (metadata.geographical_extent) {
    j["geographicalExtent"] =
        std::vector<double>(metadata.geographical_extent->begin(),
                            metadata.geographical_extent->end());
  }
  if (metadata.identifier) {
    j["identifier"] = *metadata.identifier;
  }
  if (metadata.point_of_contact) {
    j["pointOfContact"] = point_of_contact_to_json(*metadata.point_of_contact);
  }
  if (metadata.reference_date) {
    j["referenceDate"] = *metadata.reference_date;
  }
  if (metadata.reference_system) {
    j["referenceSystem"] =
        cjseq::ReferenceSystem::to_json(*metadata.reference_system);
  }
  if (metadata.title) {
    j["title"] = *metadata.title;
  }
  return j;
}

Json geometry_templates_to_json(const cjseq::GeometryTemplates &templates) {
  Json j = Json::object();
  Json arr = Json::array();
  for (const auto &geometry : templates.templates) {
    arr.push_back(geometry_to_json(geometry));
  }
  j["templates"] = std::move(arr);
  j["vertices-templates"] = templates.vertices_templates;
  return j;
}

Json city_object_to_json(const cjseq::CityObject &object) {
  Json j = Json::object();
  j["type"] = object.type;
  if (object.geographical_extent) {
    j["geographicalExtent"] = *object.geographical_extent;
  }
  if (object.attributes) {
    j["attributes"] = *object.attributes;
  }
  if (object.geometry) {
    Json geoms = Json::array();
    for (const auto &geometry : *object.geometry) {
      geoms.push_back(geometry_to_json(geometry));
    }
    j["geometry"] = std::move(geoms);
  }
  if (!object.children.empty()) {
    j["children"] = object.children;
  }
  if (!object.parents.empty()) {
    j["parents"] = object.parents;
  }
  if (!object.other.is_null()) {
    for (auto &item : object.other.items()) {
      j[item.key()] = item.value();
    }
  }
  return j;
}

Json cityjson_to_json(const cjseq::CityJSON &cityjson) {
  Json j = Json::object();
  j["type"] = cityjson.type();
  j["version"] = cityjson.version();
  j["transform"] = transform_to_json(cityjson.transform());

  Json city_objects = Json::object();
  for (const auto &[key, value] : cityjson.city_objects()) {
    city_objects[key] = city_object_to_json(value);
  }
  j["CityObjects"] = std::move(city_objects);

  j["vertices"] = cityjson.vertices();

  if (cityjson.metadata()) {
    j["metadata"] = metadata_to_json(*cityjson.metadata());
  }
  if (cityjson.appearance()) {
    j["appearance"] = appearance_to_json(*cityjson.appearance());
  }
  if (cityjson.geometry_templates()) {
    j["geometry-templates"] =
        geometry_templates_to_json(*cityjson.geometry_templates());
  }
  if (cityjson.extensions()) {
    j["extensions"] = *cityjson.extensions();
  }

  const auto &other = cityjson.other();
  if (!other.is_null()) {
    for (auto &item : other.items()) {
      j[item.key()] = item.value();
    }
  }

  return j;
}

Json cityjsonfeature_to_json(const cjseq::CityJSONFeature &feature) {
  Json j = Json::object();
  j["type"] = feature.type();
  if (!feature.id().empty()) {
    j["id"] = feature.id();
  }

  Json city_objects = Json::object();
  for (const auto &[key, value] : feature.city_objects()) {
    city_objects[key] = city_object_to_json(value);
  }
  j["CityObjects"] = std::move(city_objects);
  j["vertices"] = feature.vertices();

  if (feature.appearance()) {
    j["appearance"] = appearance_to_json(*feature.appearance());
  }

  return j;
}

void print_usage(std::ostream &stream) {
  stream << "Usage: cjseq_cli <command> [options]\n\n"
         << "Commands:\n"
         << "  cat [--order <random|lexicographical>] [file]\n"
         << "  collect [files...]\n"
         << "  filter [--exclude] [--bbox minx miny maxx maxy]\n"
         << "         [--cotype type] [--radius x y r] [--random X]\n";
}

enum class FilterMode { None, BBox, CType, Radius, Random };

struct FilterOptions {
  FilterMode mode = FilterMode::None;
  bool exclude = false;
  std::array<double, 4> bbox{};
  std::string cotype;
  std::array<double, 3> radius{};
  std::uint32_t random_factor = 1;
};

bool parse_double(const std::string &value, double &out) {
  char *end = nullptr;
  const double parsed = std::strtod(value.c_str(), &end);
  if (end == value.c_str() || *end != '\0') {
    return false;
  }
  out = parsed;
  return true;
}

bool parse_uint32(const std::string &value, std::uint32_t &out) {
  char *end = nullptr;
  const unsigned long parsed = std::strtoul(value.c_str(), &end, 10);
  if (end == value.c_str() || *end != '\0') {
    return false;
  }
  if (parsed > std::numeric_limits<std::uint32_t>::max()) {
    return false;
  }
  out = static_cast<std::uint32_t>(parsed);
  return true;
}

int handle_cat(const std::vector<std::string> &args) {
  cjseq::SortingStrategy order = cjseq::SortingStrategy::Random;
  std::optional<std::filesystem::path> input_file;

  for (std::size_t i = 0; i < args.size(); ++i) {
    const std::string &arg = args[i];
    if (arg == "-h" || arg == "--help") {
      print_usage(std::cout);
      return 0;
    } else if (arg == "-o" || arg == "--order") {
      if (i + 1 >= args.size()) {
        throw std::runtime_error("Missing value for --order");
      }
      const std::string order_value = args[++i];
      if (order_value == "random") {
        order = cjseq::SortingStrategy::Random;
      } else if (order_value == "lexicographical") {
        order = cjseq::SortingStrategy::Lexicographical;
      } else {
        throw std::runtime_error("Unknown order: " + order_value);
      }
    } else if (arg.starts_with('-')) {
      throw std::runtime_error("Unknown option: " + arg);
    } else {
      if (input_file) {
        throw std::runtime_error("Multiple input files provided for cat");
      }
      input_file = std::filesystem::path(arg);
    }
  }

  std::string content = input_file ? read_file(*input_file) : read_stdin();
  if (content.empty()) {
    throw std::runtime_error("No input provided for cat");
  }

  auto cityjson = cjseq::CityJSON::parse(content);
  cityjson.sort_cjfeatures(order);

  Json metadata_json = cityjson_to_json(cityjson.get_metadata());
  std::cout << metadata_json.dump() << '\n';

  std::size_t index = 0;
  while (auto feature = cityjson.get_cjfeature(index++)) {
    std::cout << cityjsonfeature_to_json(*feature).dump() << '\n';
  }

  return 0;
}

int handle_collect(const std::vector<std::string> &files) {
  cjseq::CityJSON accumulator;
  bool initialized = false;

  auto process_line = [&](const std::string &line) {
    if (line.empty()) {
      return;
    }
    Json parsed = Json::parse(line);
    const std::string type = parsed.value("type", "");
    if (type == "CityJSON") {
      if (!initialized) {
        accumulator = cjseq::CityJSON::parse(line);
        initialized = true;
      } else {
        // Additional metadata lines are currently ignored.
      }
    } else {
      auto feature = cjseq::CityJSONFeature::parse(line);
      accumulator.add_cjfeature(feature);
    }
  };

  if (files.empty()) {
    auto lines = read_lines(std::cin);
    for (const auto &line : lines) {
      process_line(line);
    }
  } else {
    for (const auto &file : files) {
      std::ifstream input(file);
      if (!input.is_open()) {
        throw std::runtime_error("Failed to open file: " + file);
      }
      auto lines = read_lines(input);
      for (const auto &line : lines) {
        process_line(line);
      }
    }
  }

  if (!initialized) {
    throw std::runtime_error("No metadata (CityJSON) line encountered");
  }

  accumulator.remove_duplicate_vertices();
  accumulator.update_transform();
  accumulator.update_geographical_extent();

  std::cout << cityjson_to_json(accumulator).dump() << '\n';
  return 0;
}

bool apply_filter(const cjseq::CityJSONFeature &feature,
                  const cjseq::CityJSON &metadata, FilterOptions options,
                  std::mt19937 &rng) {
  bool keep = true;
  switch (options.mode) {
  case FilterMode::None:
    keep = true;
    break;
  case FilterMode::BBox: {
    if (metadata.metadata() && metadata.metadata()->geographical_extent) {
      // Use centroid in world coordinates.
      const auto centroid = feature.centroid();
      const auto &transform = metadata.transform();
      const double cx =
          centroid[0] * transform.scale[0] + transform.translate[0];
      const double cy =
          centroid[1] * transform.scale[1] + transform.translate[1];
      keep = (cx > options.bbox[0] && cx < options.bbox[2] &&
              cy > options.bbox[1] && cy < options.bbox[3]);
    } else {
      keep = true;
    }
    break;
  }
  case FilterMode::CType: {
    const auto &objects = feature.city_objects();
    auto it = objects.find(feature.id());
    if (it != objects.end()) {
      keep = (it->second.type == options.cotype);
    } else if (!objects.empty()) {
      keep = (objects.begin()->second.type == options.cotype);
    } else {
      keep = false;
    }
    break;
  }
  case FilterMode::Radius: {
    const auto centroid = feature.centroid();
    const auto &transform = metadata.transform();
    const double cx = centroid[0] * transform.scale[0] + transform.translate[0];
    const double cy = centroid[1] * transform.scale[1] + transform.translate[1];
    const double dx = cx - options.radius[0];
    const double dy = cy - options.radius[1];
    keep = (dx * dx + dy * dy) <= (options.radius[2] * options.radius[2]);
    break;
  }
  case FilterMode::Random: {
    std::uniform_int_distribution<std::uint32_t> dist(1, options.random_factor);
    keep = (dist(rng) == 1);
    break;
  }
  }

  if (options.exclude) {
    keep = !keep;
  }
  return keep;
}

int handle_filter(const std::vector<std::string> &args) {
  FilterOptions options;
  std::optional<FilterMode> selected_mode;

  for (std::size_t i = 0; i < args.size(); ++i) {
    const std::string &arg = args[i];
    if (arg == "--exclude") {
      options.exclude = true;
    } else if (arg == "--bbox") {
      if (selected_mode && *selected_mode != FilterMode::BBox) {
        throw std::runtime_error("Multiple filter options specified");
      }
      if (i + 4 >= args.size()) {
        throw std::runtime_error("--bbox requires four values");
      }
      for (int idx = 0; idx < 4; ++idx) {
        if (!parse_double(args[++i], options.bbox[idx])) {
          throw std::runtime_error("Invalid numeric value for --bbox");
        }
      }
      options.mode = FilterMode::BBox;
      selected_mode = FilterMode::BBox;
    } else if (arg == "--cotype") {
      if (selected_mode && *selected_mode != FilterMode::CType) {
        throw std::runtime_error("Multiple filter options specified");
      }
      if (i + 1 >= args.size()) {
        throw std::runtime_error("--cotype requires a value");
      }
      options.cotype = args[++i];
      options.mode = FilterMode::CType;
      selected_mode = FilterMode::CType;
    } else if (arg == "--radius") {
      if (selected_mode && *selected_mode != FilterMode::Radius) {
        throw std::runtime_error("Multiple filter options specified");
      }
      if (i + 3 >= args.size()) {
        throw std::runtime_error("--radius requires three values");
      }
      for (int idx = 0; idx < 3; ++idx) {
        if (!parse_double(args[++i], options.radius[idx])) {
          throw std::runtime_error("Invalid numeric value for --radius");
        }
      }
      options.mode = FilterMode::Radius;
      selected_mode = FilterMode::Radius;
    } else if (arg == "--random") {
      if (selected_mode && *selected_mode != FilterMode::Random) {
        throw std::runtime_error("Multiple filter options specified");
      }
      if (i + 1 >= args.size()) {
        throw std::runtime_error("--random requires a value");
      }
      if (!parse_uint32(args[++i], options.random_factor) ||
          options.random_factor == 0) {
        throw std::runtime_error("Invalid value for --random");
      }
      options.mode = FilterMode::Random;
      selected_mode = FilterMode::Random;
    } else if (arg == "-h" || arg == "--help") {
      print_usage(std::cout);
      return 0;
    } else {
      throw std::runtime_error("Unknown option for filter: " + arg);
    }
  }

  if (options.mode == FilterMode::None) {
    options.mode = FilterMode::Random;
    options.random_factor = 1;
  }

  cjseq::CityJSON metadata = cjseq::CityJSON::parse(
      "{\"type\":\"CityJSON\",\"version\":\"2.0\",\"transform\":{\"scale\":[1."
      "0,1.0,1.0],\"translate\":[0.0,0.0,0.0]},\"CityObjects\":{},\"vertices\":"
      "[]}");
  bool metadata_initialized = false;

  std::mt19937 rng(static_cast<std::mt19937::result_type>(
      std::chrono::steady_clock::now().time_since_epoch().count()));

  std::string line;
  while (std::getline(std::cin, line)) {
    if (!line.empty() && line.back() == '\r') {
      line.pop_back();
    }
    if (line.empty()) {
      continue;
    }
    Json parsed = Json::parse(line);
    const std::string type = parsed.value("type", "");
    if (type == "CityJSON") {
      metadata = cjseq::CityJSON::parse(line);
      metadata_initialized = true;
      metadata.sort_cjfeatures(cjseq::SortingStrategy::Lexicographical);
      std::cout << cityjson_to_json(metadata).dump() << '\n';
      continue;
    }
    auto feature = cjseq::CityJSONFeature::parse(line);
    if (!metadata_initialized) {
      metadata_initialized = true;
    }
    if (apply_filter(feature, metadata, options, rng)) {
      std::cout << cityjsonfeature_to_json(feature).dump() << '\n';
    }
  }

  return 0;
}

int dispatch_command(const std::vector<std::string> &args) {
  if (args.empty()) {
    print_usage(std::cerr);
    return 1;
  }
  const std::string &command = args[0];
  std::vector<std::string> command_args(args.begin() + 1, args.end());
  if (command == "cat") {
    return handle_cat(command_args);
  }
  if (command == "collect") {
    return handle_collect(command_args);
  }
  if (command == "filter") {
    return handle_filter(command_args);
  }
  if (command == "-h" || command == "--help") {
    print_usage(std::cout);
    return 0;
  }
  print_usage(std::cerr);
  throw std::runtime_error("Unknown command: " + command);
}

} // namespace

int main(int argc, char **argv) {
  try {
    std::vector<std::string> args;
    args.reserve(static_cast<std::size_t>(argc > 0 ? argc - 1 : 0));
    for (int i = 1; i < argc; ++i) {
      args.emplace_back(argv[i]);
    }
    return dispatch_command(args);
  } catch (const std::exception &ex) {
    std::cerr << "Error: " << ex.what() << std::endl;
    return 1;
  }
}
