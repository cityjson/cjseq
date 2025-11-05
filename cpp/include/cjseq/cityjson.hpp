#pragma once

#include <cstdint>
#include <optional>
#include <string>
#include <unordered_map>
#include <vector>

#include <nlohmann/json.hpp>

namespace cjseq {

using JsonValue = nlohmann::json;

enum class SortingStrategy {
  Random,
  Lexicographical,
};

struct Transform {
  std::vector<double> scale;
  std::vector<double> translate;

  Transform();
};

struct CityObject {
  std::string type;
  std::optional<std::vector<double>> geographical_extent;
  std::optional<JsonValue> attributes;
  std::optional<std::vector<JsonValue>> geometry;
  std::vector<std::string> children;
  std::vector<std::string> parents;
  JsonValue other;

  CityObject();

  [[nodiscard]] bool is_toplevel() const;
  [[nodiscard]] std::vector<std::string> get_children_keys() const;
};

class CityJSON {
public:
  CityJSON();

  static CityJSON from_json(const JsonValue &value);
  static CityJSON parse(const std::string &json_text);

  [[nodiscard]] const std::string &type() const noexcept;
  [[nodiscard]] const std::string &version() const noexcept;
  [[nodiscard]] const Transform &transform() const noexcept;
  [[nodiscard]] const std::unordered_map<std::string, CityObject> &
  city_objects() const noexcept;
  [[nodiscard]] const std::vector<std::vector<int64_t>> &
  vertices() const noexcept;
  [[nodiscard]] const std::vector<std::string> &sorted_ids() const noexcept;
  [[nodiscard]] std::size_t number_of_city_objects() const;

  void sort_cjfeatures(SortingStrategy strategy);

private:
  std::string type_;
  std::string version_;
  Transform transform_;
  std::unordered_map<std::string, CityObject> city_objects_;
  std::vector<std::vector<int64_t>> vertices_;
  std::optional<JsonValue> metadata_;
  std::optional<JsonValue> appearance_;
  std::optional<JsonValue> geometry_templates_;
  std::optional<JsonValue> extensions_;
  JsonValue other_;
  std::vector<std::string> sorted_ids_;

  void populate_sorted_ids();
};

CityJSON parse_cityjson(const std::string &json_text);

} // namespace cjseq
