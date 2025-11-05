#pragma once

#include <array>
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

enum class GeometryType {
  MultiPoint,
  MultiLineString,
  MultiSurface,
  CompositeSurface,
  Solid,
  MultiSolid,
  CompositeSolid,
  GeometryInstance,
};

struct Transform {
  std::vector<double> scale;
  std::vector<double> translate;

  Transform();
};

struct Material {
  std::optional<JsonValue> values;
  std::optional<std::size_t> value;

  Material();
  static Material from_json(const JsonValue &value);
};

struct Texture {
  std::optional<JsonValue> values;

  Texture();
  static Texture from_json(const JsonValue &value);
};

struct Appearance {
  std::optional<std::vector<JsonValue>> materials;
  std::optional<std::vector<JsonValue>> textures;
  std::optional<std::vector<std::vector<double>>> vertices_texture;
  std::optional<std::string> default_theme_texture;
  std::optional<std::string> default_theme_material;

  Appearance();

  static Appearance from_json(const JsonValue &value);

  std::size_t add_material(JsonValue material);
  std::size_t add_texture(JsonValue texture);
  void add_vertices_texture(std::vector<std::vector<double>> vertices);
};

struct Geometry {
  GeometryType type;
  std::optional<std::string> lod;
  JsonValue boundaries;
  std::optional<JsonValue> semantics;
  std::optional<std::unordered_map<std::string, Material>> material;
  std::optional<std::unordered_map<std::string, Texture>> texture;
  std::optional<std::size_t> template_index;
  std::optional<JsonValue> transformation_matrix;

  Geometry();

  static Geometry from_json(const JsonValue &value);

  void update_geometry_boundaries(
      std::unordered_map<std::size_t, std::size_t> &vi_oldnew);
  void offset_geometry_boundaries(std::size_t offset);
  void update_material(
      std::unordered_map<std::size_t, std::size_t> &m_oldnew);
  void update_texture(
      std::unordered_map<std::size_t, std::size_t> &t_oldnew,
      std::unordered_map<std::size_t, std::size_t> &t_v_oldnew,
      std::size_t offset);
};

struct Vertex {
  std::int64_t x{};
  std::int64_t y{};
  std::int64_t z{};
};

struct Address {
  std::int64_t thoroughfare_number{};
  std::string thoroughfare_name;
  std::string locality;
  std::string postal_code;
  std::string country;

  static Address from_json(const JsonValue &value);
};

struct PointOfContact {
  std::string contact_name;
  std::optional<std::string> contact_type;
  std::optional<std::string> role;
  std::optional<std::string> phone;
  std::string email_address;
  std::optional<std::string> website;
  std::optional<Address> address;

  PointOfContact();
  static PointOfContact from_json(const JsonValue &value);
};

struct ReferenceSystem {
  std::string base_url;
  std::string authority;
  std::string version;
  std::string code;

  static ReferenceSystem from_url(const std::string &url);
  static ReferenceSystem from_json(const JsonValue &value);
  static JsonValue to_json(const ReferenceSystem &ref);
  static std::string to_url(const ReferenceSystem &ref);
};

struct Metadata {
  std::optional<std::array<double, 6>> geographical_extent;
  std::optional<std::string> identifier;
  std::optional<PointOfContact> point_of_contact;
  std::optional<std::string> reference_date;
  std::optional<ReferenceSystem> reference_system;
  std::optional<std::string> title;

  Metadata();

  static Metadata from_json(const JsonValue &value);
};

struct GeometryTemplates {
  std::vector<Geometry> templates;
  JsonValue vertices_templates;

  GeometryTemplates();

  static GeometryTemplates from_json(const JsonValue &value);
};

struct CityObject {
  std::string type;
  std::optional<std::vector<double>> geographical_extent;
  std::optional<JsonValue> attributes;
  std::optional<std::vector<Geometry>> geometry;
  std::vector<std::string> children;
  std::vector<std::string> parents;
  JsonValue other;

  CityObject();

  [[nodiscard]] std::string get_type() const;
  [[nodiscard]] bool is_toplevel() const;
  [[nodiscard]] std::vector<std::string> get_children_keys() const;
};

class CityJSONFeature {
public:
  CityJSONFeature();

  static CityJSONFeature from_json(const JsonValue &value);
  static CityJSONFeature parse(const std::string &json_text);

  void add_city_object(const std::string &id, CityObject object);

  [[nodiscard]] const std::string &type() const noexcept;
  [[nodiscard]] const std::string &id() const noexcept;
  void set_id(std::string id);

  [[nodiscard]] const std::unordered_map<std::string, CityObject> &
  city_objects() const noexcept;
  [[nodiscard]] std::unordered_map<std::string, CityObject> &city_objects();

  [[nodiscard]] const std::vector<std::vector<int64_t>> &vertices() const noexcept;
  [[nodiscard]] std::vector<std::vector<int64_t>> &vertices();

  [[nodiscard]] const std::optional<Appearance> &appearance() const noexcept;
  void set_appearance(std::optional<Appearance> appearance);

  std::vector<double> centroid() const;

private:
  std::string type_;
  std::string id_;
  std::unordered_map<std::string, CityObject> city_objects_;
  std::vector<std::vector<int64_t>> vertices_;
  std::optional<Appearance> appearance_;
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
  [[nodiscard]] const std::optional<Metadata> &metadata() const noexcept;
  [[nodiscard]] const std::optional<Appearance> &appearance() const noexcept;
  [[nodiscard]] const std::optional<GeometryTemplates> &geometry_templates() const
      noexcept;
  [[nodiscard]] const std::optional<JsonValue> &extensions() const noexcept;
  [[nodiscard]] const JsonValue &other() const noexcept;
  [[nodiscard]] std::size_t number_of_city_objects() const;

  void sort_cjfeatures(SortingStrategy strategy);

private:
  std::string type_;
  std::string version_;
  Transform transform_;
  std::unordered_map<std::string, CityObject> city_objects_;
  std::vector<std::vector<int64_t>> vertices_;
  std::optional<Metadata> metadata_;
  std::optional<Appearance> appearance_;
  std::optional<GeometryTemplates> geometry_templates_;
  std::optional<JsonValue> extensions_;
  JsonValue other_;
  std::vector<std::string> sorted_ids_;

  void populate_sorted_ids();
};

CityJSON parse_cityjson(const std::string &json_text);

} // namespace cjseq
