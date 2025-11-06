#include "cjseq/cityjson.hpp"

#include <cassert>
#include <cmath>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <optional>
#include <sstream>
#include <string>
#include <unordered_map>
#include <vector>

namespace {

std::string read_file(const std::filesystem::path &path) {
  std::ifstream input(path, std::ios::binary);
  assert(input && "Failed to open file");
  std::ostringstream buffer;
  buffer << input.rdbuf();
  return buffer.str();
}

bool nearly_equal(double a, double b, double eps = 1e-9) {
  return std::fabs(a - b) <= eps;
}

bool compare_vectors(const std::vector<double> &lhs,
                     const std::vector<double> &rhs, double eps = 1e-9) {
  if (lhs.size() != rhs.size()) {
    return false;
  }
  for (std::size_t i = 0; i < lhs.size(); ++i) {
    if (!nearly_equal(lhs[i], rhs[i], eps)) {
      return false;
    }
  }
  return true;
}

bool compare_vertices(const std::vector<std::vector<int64_t>> &lhs,
                      const std::vector<std::vector<int64_t>> &rhs) {
  if (lhs.size() != rhs.size()) {
    return false;
  }

  auto to_key = [](const std::vector<int64_t> &vertex) {
    return std::to_string(vertex[0]) + ":" + std::to_string(vertex[1]) + ":" +
           std::to_string(vertex[2]);
  };

  std::unordered_map<std::string, std::size_t> counts;
  for (const auto &vertex : lhs) {
    counts[to_key(vertex)]++;
  }
  for (const auto &vertex : rhs) {
    const auto key = to_key(vertex);
    auto it = counts.find(key);
    if (it == counts.end() || it->second == 0) {
      return false;
    }
    --it->second;
  }
  for (const auto &[_, count] : counts) {
    if (count != 0) {
      return false;
    }
  }
  return true;
}

bool compare_geometry(const cjseq::Geometry &lhs, const cjseq::Geometry &rhs) {
  if (lhs.type != rhs.type) {
    std::cerr << "Geometry type mismatch" << std::endl;
    return false;
  }
  if (lhs.lod != rhs.lod) {
    std::cerr << "Geometry LOD mismatch" << std::endl;
    return false;
  }
  if (lhs.boundaries != rhs.boundaries) {
    std::cerr << "Geometry boundaries mismatch\nLHS: " << lhs.boundaries
              << "\nRHS: " << rhs.boundaries << std::endl;
    return false;
  }
  if (lhs.semantics != rhs.semantics) {
    std::cerr << "Geometry semantics mismatch" << std::endl;
    return false;
  }
  if (lhs.material.has_value() != rhs.material.has_value()) {
    std::cerr << "Geometry material presence mismatch" << std::endl;
    return false;
  }
  if (lhs.material) {
    if (lhs.material->size() != rhs.material->size()) {
      std::cerr << "Geometry material size mismatch" << std::endl;
      return false;
    }
    for (const auto &[key, value] : *lhs.material) {
      auto it = rhs.material->find(key);
      if (it == rhs.material->end()) {
        std::cerr << "Geometry material key missing: " << key << std::endl;
        return false;
      }
      if (value.value != it->second.value) {
        std::cerr << "Geometry material scalar mismatch for key " << key
                  << std::endl;
        return false;
      }
      if (value.values != it->second.values) {
        std::cerr << "Geometry material values mismatch for key " << key
                  << std::endl;
        return false;
      }
    }
  }
  if (lhs.texture.has_value() != rhs.texture.has_value()) {
    std::cerr << "Geometry texture presence mismatch" << std::endl;
    return false;
  }
  if (lhs.texture) {
    if (lhs.texture->size() != rhs.texture->size()) {
      std::cerr << "Geometry texture size mismatch" << std::endl;
      return false;
    }
    for (const auto &[key, value] : *lhs.texture) {
      auto it = rhs.texture->find(key);
      if (it == rhs.texture->end()) {
        std::cerr << "Geometry texture key missing: " << key << std::endl;
        return false;
      }
      if (value.values != it->second.values) {
        std::cerr << "Geometry texture values mismatch for key " << key
                  << std::endl;
        return false;
      }
    }
  }
  if (lhs.template_index != rhs.template_index) {
    std::cerr << "Geometry template index mismatch" << std::endl;
    return false;
  }
  if (lhs.transformation_matrix != rhs.transformation_matrix) {
    std::cerr << "Geometry transformation matrix mismatch" << std::endl;
    return false;
  }
  return true;
}

bool compare_city_objects(
    const std::unordered_map<std::string, cjseq::CityObject> &lhs,
    const std::unordered_map<std::string, cjseq::CityObject> &rhs) {
  if (lhs.size() != rhs.size()) {
    return false;
  }
  for (const auto &[key, lhs_object] : lhs) {
    auto it = rhs.find(key);
    if (it == rhs.end()) {
      return false;
    }
    const auto &rhs_object = it->second;
    if (lhs_object.type != rhs_object.type) {
      return false;
    }
    if (lhs_object.geographical_extent != rhs_object.geographical_extent) {
      return false;
    }
    if (lhs_object.attributes != rhs_object.attributes) {
      return false;
    }
    if (lhs_object.geometry.has_value() != rhs_object.geometry.has_value()) {
      return false;
    }
    if (lhs_object.geometry) {
      const auto &lhs_geom = *lhs_object.geometry;
      const auto &rhs_geom = *rhs_object.geometry;
      if (lhs_geom.size() != rhs_geom.size()) {
        return false;
      }
      for (std::size_t i = 0; i < lhs_geom.size(); ++i) {
        if (!compare_geometry(lhs_geom[i], rhs_geom[i])) {
          return false;
        }
      }
    }
    if (lhs_object.children != rhs_object.children) {
      return false;
    }
    if (lhs_object.parents != rhs_object.parents) {
      return false;
    }
    if (lhs_object.other != rhs_object.other) {
      return false;
    }
  }
  return true;
}

bool compare_vertices_texture(const std::vector<std::vector<double>> &lhs,
                              const std::vector<std::vector<double>> &rhs,
                              double eps = 1e-9) {
  if (lhs.size() != rhs.size()) {
    return false;
  }
  for (std::size_t i = 0; i < lhs.size(); ++i) {
    if (!compare_vectors(lhs[i], rhs[i], eps)) {
      return false;
    }
  }
  return true;
}

bool compare_appearance(const std::optional<cjseq::Appearance> &lhs,
                        const std::optional<cjseq::Appearance> &rhs) {
  if (lhs.has_value() != rhs.has_value()) {
    return false;
  }
  if (!lhs.has_value()) {
    return true;
  }
  const auto &la = lhs.value();
  const auto &ra = rhs.value();
  if (la.default_theme_material != ra.default_theme_material) {
    return false;
  }
  if (la.default_theme_texture != ra.default_theme_texture) {
    return false;
  }
  if (la.materials != ra.materials) {
    return false;
  }
  if (la.textures != ra.textures) {
    return false;
  }
  if (la.vertices_texture.has_value() != ra.vertices_texture.has_value()) {
    return false;
  }
  if (la.vertices_texture) {
    if (!compare_vertices_texture(la.vertices_texture.value(),
                                  ra.vertices_texture.value())) {
      return false;
    }
  }
  return true;
}

bool compare_feature(const cjseq::CityJSONFeature &lhs,
                     const cjseq::CityJSONFeature &rhs) {
  if (lhs.type() != rhs.type()) {
    return false;
  }
  if (lhs.city_objects().size() != rhs.city_objects().size()) {
    return false;
  }
  if (!compare_city_objects(lhs.city_objects(), rhs.city_objects())) {
    return false;
  }
  if (lhs.vertices() != rhs.vertices()) {
    return false;
  }
  if (!compare_appearance(lhs.appearance(), rhs.appearance())) {
    return false;
  }
  return true;
}

std::vector<cjseq::CityJSONFeature>
extract_features(const cjseq::CityJSON &cityjson) {
  std::vector<cjseq::CityJSONFeature> features;
  const std::size_t count = cityjson.number_of_city_objects();
  features.reserve(count);
  for (std::size_t i = 0; i < count; ++i) {
    auto feature_opt = cityjson.get_cjfeature(i);
    assert(feature_opt.has_value());
    features.push_back(feature_opt.value());
  }
  return features;
}

void normalize_cityjson(cjseq::CityJSON &cj) {
  cj.remove_duplicate_vertices();
  cj.update_transform();
  cj.update_geographical_extent();
  cj.sort_cjfeatures(cjseq::SortingStrategy::Lexicographical);
}

std::string sample_cityjson_document() {
  return R"JSON({
    "type": "CityJSON",
    "version": "2.0",
    "transform": {
      "scale": [1.0, 1.0, 1.0],
      "translate": [0.0, 0.0, 0.0]
    },
    "CityObjects": {
      "building-1": {
        "type": "Building",
        "children": ["building-1-child"],
        "geometry": [
          {
            "type": "MultiSurface",
            "lod": "2.0",
            "boundaries": [[[0, 1, 2, 3]]],
            "material": {
              "default": {
                "value": 0
              }
            },
            "texture": {
              "default": {
                "values": [
                  [
                    [
                      [0, 0],
                      [1, 2]
                    ]
                  ]
                ]
              }
            }
          }
        ]
      },
      "building-1-child": {
        "type": "BuildingPart",
        "parents": ["building-1"],
        "geometry": [
          {
            "type": "MultiPoint",
            "boundaries": [0, 1]
          }
        ]
      }
    },
    "vertices": [
      [0, 0, 0],
      [1, 0, 0],
      [1, 1, 0],
      [0, 1, 0]
    ]
  })JSON";
}

} // namespace

std::string sample_feature_document() {
  return R"JSON({
    "type": "CityJSONFeature",
    "id": "feature-1",
    "CityObjects": {
      "building-1": {
        "type": "Building",
        "geometry": [
          {
            "type": "MultiSurface",
            "boundaries": [[[0, 1, 2, 3]]]
          }
        ]
      }
    },
    "vertices": [
      [0, 0, 0],
      [1, 0, 0],
      [1, 1, 0],
      [0, 1, 0]
    ]
  })JSON";
}

int main() {
  namespace fs = std::filesystem;

  const auto json_text = sample_cityjson_document();
  auto cityjson = cjseq::CityJSON::parse(json_text);

  assert(cityjson.type() == "CityJSON");
  assert(cityjson.version() == "2.0");
  assert(cityjson.transform().scale == std::vector<double>({1.0, 1.0, 1.0}));
  assert(cityjson.transform().translate ==
         std::vector<double>({0.0, 0.0, 0.0}));

  const auto &objects = cityjson.city_objects();
  assert(objects.size() == 2);
  const auto &building = objects.at("building-1");
  const auto &child = objects.at("building-1-child");

  assert(building.is_toplevel());
  assert(!child.is_toplevel());

  assert(building.geometry.has_value());
  assert(!building.geometry->empty());
  assert(building.get_type() == "Building");

  auto geometry_copy = building.geometry.value().front();
  std::unordered_map<std::size_t, std::size_t> vertex_map;
  geometry_copy.update_geometry_boundaries(vertex_map);
  assert(vertex_map.size() == 4);

  geometry_copy.offset_geometry_boundaries(10);

  std::unordered_map<std::size_t, std::size_t> material_map;
  geometry_copy.update_material(material_map);
  assert(material_map.size() == 1);

  std::unordered_map<std::size_t, std::size_t> texture_map;
  std::unordered_map<std::size_t, std::size_t> texture_vertex_map;
  geometry_copy.update_texture(texture_map, texture_vertex_map, 5);
  assert(texture_map.size() == 2);
  assert(texture_vertex_map.size() == 2);

  const auto children_keys = building.get_children_keys();
  assert(children_keys.size() == 1);
  assert(children_keys[0] == "building-1-child");

  auto metadata_doc = cityjson.get_metadata();
  assert(metadata_doc.city_objects().empty());
  assert(metadata_doc.vertices().empty());

  auto feature_opt = cityjson.get_cjfeature(0);
  assert(feature_opt.has_value());
  auto feature_slice = feature_opt.value();
  assert(feature_slice.city_objects().size() == 2);
  assert(feature_slice.vertices().size() == 4);

  assert(cityjson.number_of_city_objects() == 1);

  cityjson.sort_cjfeatures(cjseq::SortingStrategy::Lexicographical);
  const auto &sorted = cityjson.sorted_ids();
  assert(sorted.size() == 1);
  assert(sorted[0] == "building-1");

  assert(!cityjson.metadata().has_value());
  assert(!cityjson.appearance().has_value());
  assert(!cityjson.geometry_templates().has_value());

  auto slice_again = cityjson.get_cjfeature(10);
  assert(!slice_again.has_value());

  auto feature = cjseq::CityJSONFeature::parse(sample_feature_document());
  assert(feature.type() == "CityJSONFeature");
  assert(feature.id() == "feature-1");
  const auto centroid = feature.centroid();
  assert(std::fabs(centroid[0] - 0.5) < 1e-9);
  assert(std::fabs(centroid[1] - 0.5) < 1e-9);
  assert(std::fabs(centroid[2]) < 1e-9);

  cjseq::CityJSONFeature feature2;
  feature2.set_id("feature-2");
  cjseq::CityObject bridge;
  bridge.type = "Bridge";
  feature2.add_city_object("bridge-1", bridge);
  assert(feature2.city_objects().size() == 1);
  assert(feature2.city_objects().at("bridge-1").get_type() == "Bridge");

  cjseq::Appearance custom_appearance;
  custom_appearance.default_theme_material = "default";
  feature2.set_appearance(custom_appearance);
  assert(feature2.appearance().has_value());

  cjseq::CityJSON collector = metadata_doc;
  collector.add_cjfeature(feature_slice);
  assert(collector.number_of_city_objects() == 1);
  assert(collector.city_objects().size() == 2);
  assert(collector.vertices().size() == 4);
  auto back_slice_opt = collector.get_cjfeature(0);
  assert(back_slice_opt.has_value());

  collector.remove_duplicate_vertices();
  collector.update_transform();
  collector.update_geographical_extent();
  assert(collector.vertices().size() == 4);
  assert(std::fabs(collector.transform().translate[0]) < 1e-9);

  cjseq::Appearance appearance;
  cjseq::JsonValue material = cjseq::JsonValue::object();
  material["effect"] = "matte";
  const std::size_t material_index = appearance.add_material(material);
  assert(material_index == 0);
  const std::size_t material_index_duplicate =
      appearance.add_material(material);
  assert(material_index_duplicate == material_index);

  cjseq::ReferenceSystem reference = cjseq::ReferenceSystem::from_url(
      "https://www.opengis.net/def/crs/EPSG/0/7415");
  assert(reference.authority == "EPSG");
  assert(reference.code == "7415");

  const fs::path test_root = fs::path(__FILE__).parent_path();
  const auto file_cityjson = read_file(test_root / "small.city.json");
  auto file_original = cjseq::CityJSON::parse(file_cityjson);
  file_original.sort_cjfeatures(cjseq::SortingStrategy::Lexicographical);

  std::vector<cjseq::CityJSONFeature> file_features;
  const std::size_t file_count = file_original.number_of_city_objects();
  for (std::size_t i = 0; i < file_count; ++i) {
    auto feature = file_original.get_cjfeature(i);
    assert(feature.has_value());
    file_features.push_back(feature.value());
  }

  auto reconstructed_file = file_original.get_metadata();
  for (auto &feature : file_features) {
    reconstructed_file.add_cjfeature(feature);
  }

  normalize_cityjson(file_original);
  normalize_cityjson(reconstructed_file);

  auto original_features = extract_features(file_original);
  auto reconstructed_features = extract_features(reconstructed_file);
  assert(original_features.size() == reconstructed_features.size());
  for (std::size_t i = 0; i < original_features.size(); ++i) {
    assert(compare_feature(original_features[i], reconstructed_features[i]));
  }

  assert(compare_vertices(file_original.vertices(),
                          reconstructed_file.vertices()));
  assert(file_original.sorted_ids() == reconstructed_file.sorted_ids());
  assert(compare_vectors(file_original.transform().scale,
                         reconstructed_file.transform().scale));
  assert(compare_vectors(file_original.transform().translate,
                         reconstructed_file.transform().translate));
  assert(file_original.other() == reconstructed_file.other());

  std::ifstream jsonl_input(test_root / "small.city.jsonl");
  assert(jsonl_input && "Failed to open small.city.jsonl");
  std::string line;
  std::getline(jsonl_input, line);
  auto jsonl_base = cjseq::CityJSON::parse(line);

  std::vector<cjseq::CityJSONFeature> jsonl_features;
  while (std::getline(jsonl_input, line)) {
    if (line.empty()) {
      continue;
    }
    jsonl_features.push_back(cjseq::CityJSONFeature::parse(line));
  }

  assert(jsonl_features.size() == file_count);

  for (auto &feature : jsonl_features) {
    jsonl_base.add_cjfeature(feature);
  }

  normalize_cityjson(jsonl_base);

  auto reconstructed_jsonl_features = extract_features(jsonl_base);
  assert(reconstructed_jsonl_features.size() == original_features.size());
  for (std::size_t i = 0; i < reconstructed_jsonl_features.size(); ++i) {
    assert(
        compare_feature(original_features[i], reconstructed_jsonl_features[i]));
  }

  assert(compare_vertices(file_original.vertices(), jsonl_base.vertices()));
  assert(file_original.sorted_ids() == jsonl_base.sorted_ids());
  assert(compare_vectors(file_original.transform().scale,
                         jsonl_base.transform().scale));
  assert(compare_vectors(file_original.transform().translate,
                         jsonl_base.transform().translate));
  assert(file_original.other() == jsonl_base.other());

  std::cout << "All cjseq tests passed." << std::endl;
  return 0;
}
