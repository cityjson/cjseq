#include "cjseq/cityjson.hpp"

#include <algorithm>
#include <array>
#include <stdexcept>
#include <string_view>

namespace cjseq {
namespace {

constexpr std::array<const char *, 9> kCityJsonKnownKeys = {
    "type",     "version",    "transform",          "CityObjects", "vertices",
    "metadata", "appearance", "geometry-templates", "extensions"};

constexpr std::array<const char *, 6> kCityObjectKnownKeys = {
    "type",     "geographicalExtent", "attributes",
    "geometry", "children",           "parents"};

bool is_known_key(std::string_view key,
                  const std::array<const char *, 9> &known_keys) {
  return std::any_of(known_keys.begin(), known_keys.end(),
                     [&](const char *candidate) { return key == candidate; });
}

bool is_known_object_key(std::string_view key,
                         const std::array<const char *, 6> &known_keys) {
  return std::any_of(known_keys.begin(), known_keys.end(),
                     [&](const char *candidate) { return key == candidate; });
}

Transform parse_transform(const JsonValue &value) {
  Transform transform;
  if (value.contains("scale")) {
    transform.scale = value.at("scale").get<std::vector<double>>();
  }
  if (value.contains("translate")) {
    transform.translate = value.at("translate").get<std::vector<double>>();
  }
  return transform;
}

CityObject parse_city_object(const JsonValue &value) {
  if (!value.contains("type")) {
    throw std::runtime_error("CityObject is missing required field 'type'");
  }

  CityObject object;
  object.type = value.at("type").get<std::string>();

  if (value.contains("geographicalExtent")) {
    object.geographical_extent =
        value.at("geographicalExtent").get<std::vector<double>>();
  }

  if (value.contains("attributes")) {
    object.attributes = value.at("attributes");
  }

  if (value.contains("geometry")) {
    object.geometry = value.at("geometry").get<std::vector<JsonValue>>();
  }

  if (value.contains("children")) {
    object.children = value.at("children").get<std::vector<std::string>>();
  }

  if (value.contains("parents")) {
    object.parents = value.at("parents").get<std::vector<std::string>>();
  }

  object.other = JsonValue::object();
  for (const auto &[key, entry] : value.items()) {
    if (!is_known_object_key(key, kCityObjectKnownKeys)) {
      object.other[key] = entry;
    }
  }
  if (object.other.empty()) {
    object.other = JsonValue();
  }

  return object;
}

} // namespace

Transform::Transform() : scale({1.0, 1.0, 1.0}), translate({0.0, 0.0, 0.0}) {}

CityObject::CityObject() : type(""), other(JsonValue::object()) {}

bool CityObject::is_toplevel() const { return parents.empty(); }

std::vector<std::string> CityObject::get_children_keys() const {
  return children;
}

CityJSON::CityJSON()
    : type_("CityJSON"), version_("2.0"), transform_(),
      other_(JsonValue::object()) {}

CityJSON CityJSON::from_json(const JsonValue &value) {
  if (!value.contains("CityObjects")) {
    throw std::runtime_error("CityJSON document missing 'CityObjects'");
  }

  CityJSON document;
  if (value.contains("type")) {
    document.type_ = value.at("type").get<std::string>();
  }
  if (value.contains("version")) {
    document.version_ = value.at("version").get<std::string>();
  }

  if (value.contains("transform")) {
    document.transform_ = parse_transform(value.at("transform"));
  }

  const auto &city_objects_json = value.at("CityObjects");
  for (const auto &[id, entry] : city_objects_json.items()) {
    CityObject object = parse_city_object(entry);
    if (object.is_toplevel()) {
      document.sorted_ids_.push_back(id);
    }
    document.city_objects_.emplace(id, std::move(object));
  }

  if (value.contains("vertices")) {
    document.vertices_ =
        value.at("vertices").get<std::vector<std::vector<int64_t>>>();
  }

  if (value.contains("metadata")) {
    document.metadata_ = value.at("metadata");
  }

  if (value.contains("appearance")) {
    document.appearance_ = value.at("appearance");
  }

  if (value.contains("geometry-templates")) {
    document.geometry_templates_ = value.at("geometry-templates");
  }

  if (value.contains("extensions")) {
    document.extensions_ = value.at("extensions");
  }

  document.other_ = JsonValue::object();
  for (const auto &[key, entry] : value.items()) {
    if (!is_known_key(key, kCityJsonKnownKeys)) {
      document.other_[key] = entry;
    }
  }
  if (document.other_.empty()) {
    document.other_ = JsonValue();
  }

  document.populate_sorted_ids();
  return document;
}

CityJSON CityJSON::parse(const std::string &json_text) {
  return from_json(JsonValue::parse(json_text));
}

const std::string &CityJSON::type() const noexcept { return type_; }

const std::string &CityJSON::version() const noexcept { return version_; }

const Transform &CityJSON::transform() const noexcept { return transform_; }

const std::unordered_map<std::string, CityObject> &
CityJSON::city_objects() const noexcept {
  return city_objects_;
}

const std::vector<std::vector<int64_t>> &CityJSON::vertices() const noexcept {
  return vertices_;
}

const std::vector<std::string> &CityJSON::sorted_ids() const noexcept {
  return sorted_ids_;
}

std::size_t CityJSON::number_of_city_objects() const {
  return sorted_ids_.size();
}

void CityJSON::sort_cjfeatures(SortingStrategy strategy) {
  populate_sorted_ids();
  switch (strategy) {
  case SortingStrategy::Random:
    // Keep the existing insertion order that populate_sorted_ids produced.
    break;
  case SortingStrategy::Lexicographical:
    std::sort(sorted_ids_.begin(), sorted_ids_.end());
    break;
  }
}

void CityJSON::populate_sorted_ids() {
  sorted_ids_.clear();
  sorted_ids_.reserve(city_objects_.size());
  for (const auto &[id, object] : city_objects_) {
    if (object.is_toplevel()) {
      sorted_ids_.push_back(id);
    }
  }
}

CityJSON parse_cityjson(const std::string &json_text) {
  return CityJSON::parse(json_text);
}

} // namespace cjseq
