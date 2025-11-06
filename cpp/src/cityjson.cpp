#include "cjseq/cityjson.hpp"

#include <algorithm>
#include <array>
#include <limits>
#include <stdexcept>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

namespace cjseq {
namespace {

using IndexMap = std::unordered_map<std::size_t, std::size_t>;

constexpr std::array<const char *, 9> kCityJsonKnownKeys = {
    "type",     "version",    "transform",          "CityObjects", "vertices",
    "metadata", "appearance", "geometry-templates", "extensions"};

constexpr std::array<const char *, 6> kCityObjectKnownKeys = {
    "type",     "geographicalExtent", "attributes",
    "geometry", "children",           "parents"};

GeometryType geometry_type_from_string(const std::string &value) {
  if (value == "MultiPoint") {
    return GeometryType::MultiPoint;
  }
  if (value == "MultiLineString") {
    return GeometryType::MultiLineString;
  }
  if (value == "MultiSurface") {
    return GeometryType::MultiSurface;
  }
  if (value == "CompositeSurface") {
    return GeometryType::CompositeSurface;
  }
  if (value == "Solid") {
    return GeometryType::Solid;
  }
  if (value == "MultiSolid") {
    return GeometryType::MultiSolid;
  }
  if (value == "CompositeSolid") {
    return GeometryType::CompositeSolid;
  }
  if (value == "GeometryInstance") {
    return GeometryType::GeometryInstance;
  }
  throw std::runtime_error("Unsupported geometry type: " + value);
}

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

std::size_t ensure_index(IndexMap &map, std::size_t original) {
  const auto it = map.find(original);
  if (it != map.end()) {
    return it->second;
  }
  const std::size_t next = map.size();
  map.emplace(original, next);
  return next;
}

void remap_vertex_indices(JsonValue &value, IndexMap &map) {
  if (value.is_null()) {
    return;
  }
  if (value.is_number_integer()) {
    const auto original = static_cast<std::size_t>(value.get<std::int64_t>());
    const auto mapped = ensure_index(map, original);
    value = static_cast<std::int64_t>(mapped);
    return;
  }
  if (value.is_array()) {
    for (auto &child : value) {
      remap_vertex_indices(child, map);
    }
  } else if (value.is_object()) {
    for (auto &item : value.items()) {
      remap_vertex_indices(item.value(), map);
    }
  }
}

void apply_vertex_offset(JsonValue &value, std::size_t offset) {
  if (value.is_null()) {
    return;
  }
  if (value.is_number_integer()) {
    const auto original = static_cast<std::size_t>(value.get<std::int64_t>());
    value = static_cast<std::int64_t>(original + offset);
    return;
  }
  if (value.is_array()) {
    for (auto &child : value) {
      apply_vertex_offset(child, offset);
    }
  } else if (value.is_object()) {
    for (auto &item : value.items()) {
      apply_vertex_offset(item.value(), offset);
    }
  }
}

void remap_optional_indices(JsonValue &value, IndexMap &map) {
  if (value.is_null()) {
    return;
  }
  if (value.is_number_integer()) {
    const auto original = static_cast<std::size_t>(value.get<std::int64_t>());
    const auto mapped = ensure_index(map, original);
    value = static_cast<std::int64_t>(mapped);
    return;
  }
  if (value.is_array()) {
    for (auto &child : value) {
      remap_optional_indices(child, map);
    }
  } else if (value.is_object()) {
    for (auto &item : value.items()) {
      remap_optional_indices(item.value(), map);
    }
  }
}

void remap_texture_values(JsonValue &value, IndexMap &tex_map,
                          IndexMap &vertex_tex_map, std::size_t offset) {
  if (value.is_null()) {
    return;
  }
  if (value.is_array()) {
    for (std::size_t i = 0; i < value.size(); ++i) {
      auto &child = value.at(i);
      if (child.is_array()) {
        remap_texture_values(child, tex_map, vertex_tex_map, offset);
      } else if (child.is_null()) {
        continue;
      } else if (child.is_number_integer()) {
        const auto raw = static_cast<std::size_t>(child.get<std::int64_t>());
        if (i == 0) {
          const auto mapped = ensure_index(tex_map, raw);
          child = static_cast<std::int64_t>(mapped);
        } else {
          const auto mapped = ensure_index(vertex_tex_map, raw);
          child = static_cast<std::int64_t>(mapped + offset);
        }
      }
    }
  } else if (value.is_object()) {
    for (auto &item : value.items()) {
      remap_texture_values(item.value(), tex_map, vertex_tex_map, offset);
    }
  }
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

Geometry parse_geometry(const JsonValue &value) {
  return Geometry::from_json(value);
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
    const auto &geometry_array = value.at("geometry");
    if (!geometry_array.is_array()) {
      throw std::runtime_error("CityObject.geometry must be an array");
    }
    std::vector<Geometry> geometries;
    geometries.reserve(geometry_array.size());
    for (const auto &geometry_value : geometry_array) {
      geometries.emplace_back(parse_geometry(geometry_value));
    }
    object.geometry = std::move(geometries);
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

Material::Material() = default;

Material Material::from_json(const JsonValue &value) {
  Material material;
  if (value.contains("value")) {
    material.value =
        static_cast<std::size_t>(value.at("value").get<std::int64_t>());
  }
  if (value.contains("values")) {
    material.values = value.at("values");
  }
  return material;
}

Texture::Texture() = default;

Texture Texture::from_json(const JsonValue &value) {
  Texture texture;
  if (value.contains("values")) {
    texture.values = value.at("values");
  }
  return texture;
}

Appearance::Appearance() = default;

Appearance Appearance::from_json(const JsonValue &value) {
  Appearance appearance;
  if (value.contains("materials")) {
    appearance.materials = value.at("materials").get<std::vector<JsonValue>>();
  }
  if (value.contains("textures")) {
    appearance.textures = value.at("textures").get<std::vector<JsonValue>>();
  }
  if (value.contains("vertices-texture")) {
    appearance.vertices_texture =
        value.at("vertices-texture").get<std::vector<std::vector<double>>>();
  }
  if (value.contains("default-theme-texture")) {
    appearance.default_theme_texture =
        value.at("default-theme-texture").get<std::string>();
  }
  if (value.contains("default-theme-material")) {
    appearance.default_theme_material =
        value.at("default-theme-material").get<std::string>();
  }
  return appearance;
}

std::size_t Appearance::add_material(JsonValue material) {
  if (!materials) {
    materials = std::vector<JsonValue>{};
  }
  auto &list = *materials;
  const auto it = std::find(list.begin(), list.end(), material);
  if (it != list.end()) {
    return static_cast<std::size_t>(std::distance(list.begin(), it));
  }
  list.push_back(material);
  return list.size() - 1;
}

std::size_t Appearance::add_texture(JsonValue texture) {
  if (!textures) {
    textures = std::vector<JsonValue>{};
  }
  auto &list = *textures;
  const auto it = std::find(list.begin(), list.end(), texture);
  if (it != list.end()) {
    return static_cast<std::size_t>(std::distance(list.begin(), it));
  }
  list.push_back(texture);
  return list.size() - 1;
}

void Appearance::add_vertices_texture(
    std::vector<std::vector<double>> vertices) {
  if (!vertices_texture) {
    vertices_texture = std::vector<std::vector<double>>{};
  }
  auto &list = *vertices_texture;
  list.insert(list.end(), vertices.begin(), vertices.end());
}

Geometry::Geometry() = default;

Geometry Geometry::from_json(const JsonValue &value) {
  Geometry geometry;
  geometry.type =
      geometry_type_from_string(value.at("type").get<std::string>());
  geometry.boundaries = value.at("boundaries");

  if (value.contains("lod")) {
    geometry.lod = value.at("lod").get<std::string>();
  }
  if (value.contains("semantics")) {
    geometry.semantics = value.at("semantics");
  }
  if (value.contains("material")) {
    std::unordered_map<std::string, Material> map;
    for (const auto &[key, material_value] : value.at("material").items()) {
      map.emplace(key, Material::from_json(material_value));
    }
    geometry.material = std::move(map);
  }
  if (value.contains("texture")) {
    std::unordered_map<std::string, Texture> map;
    for (const auto &[key, texture_value] : value.at("texture").items()) {
      map.emplace(key, Texture::from_json(texture_value));
    }
    geometry.texture = std::move(map);
  }
  if (value.contains("template")) {
    geometry.template_index =
        static_cast<std::size_t>(value.at("template").get<std::int64_t>());
  }
  if (value.contains("transformationMatrix")) {
    geometry.transformation_matrix = value.at("transformationMatrix");
  }

  return geometry;
}

void Geometry::update_geometry_boundaries(IndexMap &vi_oldnew) {
  JsonValue updated = boundaries;
  remap_vertex_indices(updated, vi_oldnew);
  boundaries = std::move(updated);
}

void Geometry::offset_geometry_boundaries(std::size_t offset) {
  JsonValue updated = boundaries;
  apply_vertex_offset(updated, offset);
  boundaries = std::move(updated);
}

void Geometry::update_material(IndexMap &m_oldnew) {
  if (!material) {
    return;
  }
  for (auto &[_, material_value] : *material) {
    if (material_value.value) {
      const auto mapped = ensure_index(m_oldnew, *material_value.value);
      material_value.value = mapped;
    }
    if (material_value.values) {
      JsonValue updated = *material_value.values;
      remap_optional_indices(updated, m_oldnew);
      material_value.values = std::move(updated);
    }
  }
}

void Geometry::update_texture(IndexMap &t_oldnew, IndexMap &t_v_oldnew,
                              std::size_t offset) {
  if (!texture) {
    return;
  }
  for (auto &[_, texture_value] : *texture) {
    if (texture_value.values) {
      JsonValue updated = *texture_value.values;
      remap_texture_values(updated, t_oldnew, t_v_oldnew, offset);
      texture_value.values = std::move(updated);
    }
  }
}

Address Address::from_json(const JsonValue &value) {
  Address address;
  address.thoroughfare_number = static_cast<std::int64_t>(
      value.at("thoroughfareNumber").get<std::int64_t>());
  address.thoroughfare_name = value.at("thoroughfareName").get<std::string>();
  address.locality = value.at("locality").get<std::string>();
  address.postal_code = value.at("postalCode").get<std::string>();
  address.country = value.at("country").get<std::string>();
  return address;
}

PointOfContact::PointOfContact() = default;

PointOfContact PointOfContact::from_json(const JsonValue &value) {
  PointOfContact contact;
  contact.contact_name = value.at("contactName").get<std::string>();
  if (value.contains("contactType")) {
    contact.contact_type = value.at("contactType").get<std::string>();
  }
  if (value.contains("role")) {
    contact.role = value.at("role").get<std::string>();
  }
  if (value.contains("phone")) {
    contact.phone = value.at("phone").get<std::string>();
  }
  contact.email_address = value.at("emailAddress").get<std::string>();
  if (value.contains("website")) {
    contact.website = value.at("website").get<std::string>();
  }
  if (value.contains("address")) {
    contact.address = Address::from_json(value.at("address"));
  }
  return contact;
}

ReferenceSystem ReferenceSystem::from_url(const std::string &url) {
  const std::string needle = "/crs/";
  const auto pos = url.find(needle);
  if (pos == std::string::npos) {
    throw std::runtime_error("Invalid reference system URL: " + url);
  }

  ReferenceSystem ref;
  ref.base_url = url.substr(0, pos + needle.size() - 1);
  std::string remainder = url.substr(pos + needle.size());

  std::vector<std::string> parts;
  std::size_t start = 0;
  while (true) {
    const auto slash = remainder.find('/', start);
    if (slash == std::string::npos) {
      parts.emplace_back(remainder.substr(start));
      break;
    }
    parts.emplace_back(remainder.substr(start, slash - start));
    start = slash + 1;
  }

  parts.erase(std::remove_if(
                  parts.begin(), parts.end(),
                  [](const std::string &segment) { return segment.empty(); }),
              parts.end());

  if (parts.size() != 3) {
    throw std::runtime_error("Invalid reference system URL: " + url);
  }

  ref.authority = parts[0];
  ref.version = parts[1];
  ref.code = parts[2];
  return ref;
}

ReferenceSystem ReferenceSystem::from_json(const JsonValue &value) {
  if (value.is_string()) {
    return from_url(value.get<std::string>());
  }
  ReferenceSystem ref;
  ref.base_url = value.value("baseUrl", "https://www.opengis.net/def/crs");
  ref.authority = value.value("authority", "");
  ref.version = value.value("version", "");
  ref.code = value.value("code", "");
  return ref;
}

JsonValue ReferenceSystem::to_json(const ReferenceSystem &ref) {
  return JsonValue(to_url(ref));
}

std::string ReferenceSystem::to_url(const ReferenceSystem &ref) {
  return ref.base_url + "/" + ref.authority + "/" + ref.version + "/" +
         ref.code;
}

Metadata::Metadata() = default;

Metadata Metadata::from_json(const JsonValue &value) {
  Metadata metadata;
  if (value.contains("geographicalExtent")) {
    const auto extent =
        value.at("geographicalExtent").get<std::vector<double>>();
    if (extent.size() == 6) {
      metadata.geographical_extent = {extent[0], extent[1], extent[2],
                                      extent[3], extent[4], extent[5]};
    }
  }
  if (value.contains("identifier")) {
    metadata.identifier = value.at("identifier").get<std::string>();
  }
  if (value.contains("pointOfContact")) {
    metadata.point_of_contact =
        PointOfContact::from_json(value.at("pointOfContact"));
  }
  if (value.contains("referenceDate")) {
    metadata.reference_date = value.at("referenceDate").get<std::string>();
  }
  if (value.contains("referenceSystem")) {
    metadata.reference_system =
        ReferenceSystem::from_json(value.at("referenceSystem"));
  }
  if (value.contains("title")) {
    metadata.title = value.at("title").get<std::string>();
  }
  return metadata;
}

GeometryTemplates::GeometryTemplates() = default;

GeometryTemplates GeometryTemplates::from_json(const JsonValue &value) {
  GeometryTemplates templates;
  if (value.contains("templates")) {
    const auto &templates_array = value.at("templates");
    if (templates_array.is_array()) {
      templates.templates.reserve(templates_array.size());
      for (const auto &geometry_value : templates_array) {
        templates.templates.emplace_back(Geometry::from_json(geometry_value));
      }
    }
  }
  if (value.contains("vertices-templates")) {
    templates.vertices_templates = value.at("vertices-templates");
  } else {
    templates.vertices_templates = JsonValue::array();
  }
  return templates;
}

CityObject::CityObject() : type(""), other(JsonValue::object()) {}

std::string CityObject::get_type() const { return type; }

bool CityObject::is_toplevel() const { return parents.empty(); }

std::vector<std::string> CityObject::get_children_keys() const {
  return children;
}

CityJSONFeature::CityJSONFeature() : type_("CityJSONFeature"), id_("") {}

CityJSONFeature CityJSONFeature::from_json(const JsonValue &value) {
  CityJSONFeature feature;
  if (value.contains("type")) {
    feature.type_ = value.at("type").get<std::string>();
  }
  if (value.contains("id")) {
    feature.id_ = value.at("id").get<std::string>();
  }

  if (value.contains("CityObjects")) {
    const auto &objects = value.at("CityObjects");
    for (const auto &[key, entry] : objects.items()) {
      feature.city_objects_.emplace(key, parse_city_object(entry));
    }
  }

  if (value.contains("vertices")) {
    feature.vertices_ =
        value.at("vertices").get<std::vector<std::vector<int64_t>>>();
  }

  if (value.contains("appearance")) {
    feature.appearance_ = Appearance::from_json(value.at("appearance"));
  }

  return feature;
}

CityJSONFeature CityJSONFeature::parse(const std::string &json_text) {
  return from_json(JsonValue::parse(json_text));
}

void CityJSONFeature::add_city_object(const std::string &id,
                                      CityObject object) {
  city_objects_.insert_or_assign(id, std::move(object));
}

const std::string &CityJSONFeature::type() const noexcept { return type_; }

const std::string &CityJSONFeature::id() const noexcept { return id_; }

void CityJSONFeature::set_id(std::string id) { id_ = std::move(id); }

const std::unordered_map<std::string, CityObject> &
CityJSONFeature::city_objects() const noexcept {
  return city_objects_;
}

std::unordered_map<std::string, CityObject> &CityJSONFeature::city_objects() {
  return city_objects_;
}

const std::vector<std::vector<int64_t>> &
CityJSONFeature::vertices() const noexcept {
  return vertices_;
}

std::vector<std::vector<int64_t>> &CityJSONFeature::vertices() {
  return vertices_;
}

const std::optional<Appearance> &CityJSONFeature::appearance() const noexcept {
  return appearance_;
}

void CityJSONFeature::set_appearance(std::optional<Appearance> appearance) {
  appearance_ = std::move(appearance);
}

std::vector<double> CityJSONFeature::centroid() const {
  if (vertices_.empty()) {
    return {0.0, 0.0, 0.0};
  }
  std::array<double, 3> totals{0.0, 0.0, 0.0};
  for (const auto &vertex : vertices_) {
    for (std::size_t i = 0; i < std::min<std::size_t>(3, vertex.size()); ++i) {
      totals[i] += static_cast<double>(vertex[i]);
    }
  }
  const double count = static_cast<double>(vertices_.size());
  for (double &value : totals) {
    value /= count;
  }
  return {totals[0], totals[1], totals[2]};
}

CityJSON::CityJSON()
    : type_("CityJSON"), version_("2.0"), transform_(), metadata_(std::nullopt),
      appearance_(std::nullopt), geometry_templates_(std::nullopt),
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
    document.city_objects_.emplace(id, parse_city_object(entry));
  }

  if (value.contains("vertices")) {
    document.vertices_ =
        value.at("vertices").get<std::vector<std::vector<int64_t>>>();
  }

  if (value.contains("metadata")) {
    document.metadata_ = Metadata::from_json(value.at("metadata"));
  }

  if (value.contains("appearance")) {
    document.appearance_ = Appearance::from_json(value.at("appearance"));
  }

  if (value.contains("geometry-templates")) {
    document.geometry_templates_ =
        GeometryTemplates::from_json(value.at("geometry-templates"));
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

const std::optional<Metadata> &CityJSON::metadata() const noexcept {
  return metadata_;
}

const std::optional<Appearance> &CityJSON::appearance() const noexcept {
  return appearance_;
}

const std::optional<GeometryTemplates> &
CityJSON::geometry_templates() const noexcept {
  return geometry_templates_;
}

const std::optional<JsonValue> &CityJSON::extensions() const noexcept {
  return extensions_;
}

const JsonValue &CityJSON::other() const noexcept { return other_; }

std::size_t CityJSON::number_of_city_objects() const {
  return sorted_ids_.size();
}

void CityJSON::sort_cjfeatures(SortingStrategy strategy) {
  populate_sorted_ids();
  switch (strategy) {
  case SortingStrategy::Random:
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

void CityJSON::ensure_sorted_ids_initialized() {
  if (!sorted_ids_.empty()) {
    return;
  }
  populate_sorted_ids();
}

void CityJSON::append_vertices(
    const std::vector<std::vector<int64_t>> &vertices) {
  vertices_.insert(vertices_.end(), vertices.begin(), vertices.end());
}

void CityJSON::refresh_geographical_extent_bounds(
    const std::array<double, 6> &bounds) {
  if (!metadata_) {
    metadata_ = Metadata();
  }
  if (!metadata_->geographical_extent) {
    metadata_->geographical_extent = bounds;
    return;
  }
  auto &extent = metadata_->geographical_extent.value();
  extent[0] = std::min(extent[0], bounds[0]);
  extent[1] = std::min(extent[1], bounds[1]);
  extent[2] = std::min(extent[2], bounds[2]);
  extent[3] = std::max(extent[3], bounds[3]);
  extent[4] = std::max(extent[4], bounds[4]);
  extent[5] = std::max(extent[5], bounds[5]);
}

std::size_t CityJSON::add_material(const JsonValue &material) {
  if (!appearance_) {
    appearance_ = Appearance();
  }
  return appearance_->add_material(material);
}

std::size_t CityJSON::add_texture(const JsonValue &texture) {
  if (!appearance_) {
    appearance_ = Appearance();
  }
  return appearance_->add_texture(texture);
}

std::size_t CityJSON::add_vertices_texture(
    const std::vector<std::vector<double>> &vertices) {
  if (!appearance_) {
    appearance_ = Appearance();
  }
  if (!appearance_->vertices_texture) {
    appearance_->vertices_texture = std::vector<std::vector<double>>{};
  }
  auto &list = *appearance_->vertices_texture;
  const std::size_t offset = list.size();
  list.insert(list.end(), vertices.begin(), vertices.end());
  return offset;
}

CityJSON CityJSON::get_metadata() const {
  CityJSON metadata_doc;
  metadata_doc.type_ = type_;
  metadata_doc.version_ = version_;
  metadata_doc.transform_ = transform_;
  metadata_doc.metadata_ = metadata_;
  metadata_doc.geometry_templates_ = geometry_templates_;
  metadata_doc.extensions_ = extensions_;
  metadata_doc.other_ = other_;
  return metadata_doc;
}

std::optional<CityJSONFeature>
CityJSON::get_cjfeature(std::size_t index) const {
  if (city_objects_.empty()) {
    return std::nullopt;
  }

  const_cast<CityJSON *>(this)->ensure_sorted_ids_initialized();

  if (index >= sorted_ids_.size()) {
    return std::nullopt;
  }

  const std::string &top_id = sorted_ids_.at(index);
  const auto object_it = city_objects_.find(top_id);
  if (object_it == city_objects_.end()) {
    return std::nullopt;
  }

  CityJSONFeature feature;
  feature.set_id(top_id);

  IndexMap vertex_map;
  IndexMap material_map;
  IndexMap texture_map;
  IndexMap texture_vertex_map;

  const auto append_object = [&](const std::string &id,
                                 const CityObject &object) {
    CityObject copy = object;
    if (copy.geometry) {
      for (auto &geometry : *copy.geometry) {
        geometry.update_geometry_boundaries(vertex_map);
        geometry.update_material(material_map);
        geometry.update_texture(texture_map, texture_vertex_map, 0);
      }
    }
    feature.add_city_object(id, std::move(copy));
  };

  append_object(top_id, object_it->second);

  const auto children_keys = object_it->second.get_children_keys();
  for (const auto &child_id : children_keys) {
    const auto child_it = city_objects_.find(child_id);
    if (child_it == city_objects_.end()) {
      continue;
    }
    append_object(child_id, child_it->second);
  }

  std::vector<std::vector<int64_t>> collected_vertices(vertex_map.size());
  for (const auto &[old_index, new_index] : vertex_map) {
    if (old_index < vertices_.size()) {
      collected_vertices.at(new_index) = vertices_.at(old_index);
    }
  }
  feature.vertices() = std::move(collected_vertices);

  if (appearance_) {
    Appearance feature_appearance;
    bool has_data = false;
    if (appearance_->default_theme_material) {
      feature_appearance.default_theme_material =
          appearance_->default_theme_material;
      has_data = true;
    }
    if (appearance_->default_theme_texture) {
      feature_appearance.default_theme_texture =
          appearance_->default_theme_texture;
      has_data = true;
    }
    if (appearance_->materials) {
      std::vector<JsonValue> materials(material_map.size(), JsonValue());
      for (const auto &[old_index, new_index] : material_map) {
        if (old_index < appearance_->materials->size()) {
          materials.at(new_index) = appearance_->materials->at(old_index);
        }
      }
      feature_appearance.materials = std::move(materials);
      has_data = has_data || !material_map.empty();
    }
    if (appearance_->textures) {
      std::vector<JsonValue> textures(texture_map.size(), JsonValue());
      for (const auto &[old_index, new_index] : texture_map) {
        if (old_index < appearance_->textures->size()) {
          textures.at(new_index) = appearance_->textures->at(old_index);
        }
      }
      feature_appearance.textures = std::move(textures);
      has_data = has_data || !texture_map.empty();
    }
    if (appearance_->vertices_texture) {
      std::vector<std::vector<double>> vertices_texture(
          texture_vertex_map.size(), std::vector<double>());
      for (const auto &[old_index, new_index] : texture_vertex_map) {
        if (old_index < appearance_->vertices_texture->size()) {
          vertices_texture.at(new_index) =
              appearance_->vertices_texture->at(old_index);
        }
      }
      feature_appearance.vertices_texture = std::move(vertices_texture);
      has_data = has_data || !texture_vertex_map.empty();
    }

    if (has_data) {
      feature.set_appearance(std::move(feature_appearance));
    }
  }

  return feature;
}

void CityJSON::add_cjfeature(CityJSONFeature &feature) {
  IndexMap material_map;
  IndexMap texture_map;
  IndexMap vertex_texture_map;

  const std::size_t vertex_offset = vertices_.size();
  std::size_t vertex_texture_offset = 0;
  if (appearance_ && appearance_->vertices_texture) {
    vertex_texture_offset = appearance_->vertices_texture->size();
  }

  if (const auto feature_appearance_opt = feature.appearance()) {
    const auto &feature_appearance = *feature_appearance_opt;

    if (feature_appearance.materials) {
      for (std::size_t i = 0; i < feature_appearance.materials->size(); ++i) {
        const auto &material_value = feature_appearance.materials->at(i);
        const std::size_t mapped_index = add_material(material_value);
        material_map.emplace(i, mapped_index);
      }
    }
    if (feature_appearance.textures) {
      for (std::size_t i = 0; i < feature_appearance.textures->size(); ++i) {
        const auto &texture_value = feature_appearance.textures->at(i);
        const std::size_t mapped_index = add_texture(texture_value);
        texture_map.emplace(i, mapped_index);
      }
    }
    if (feature_appearance.vertices_texture) {
      vertex_texture_offset =
          add_vertices_texture(*feature_appearance.vertices_texture);
    }
    if (feature_appearance.default_theme_material) {
      if (!appearance_) {
        appearance_ = Appearance();
      }
      appearance_->default_theme_material =
          feature_appearance.default_theme_material;
    }
    if (feature_appearance.default_theme_texture) {
      if (!appearance_) {
        appearance_ = Appearance();
      }
      appearance_->default_theme_texture =
          feature_appearance.default_theme_texture;
    }
  }

  for (auto &[id, object] : feature.city_objects()) {
    if (object.geometry) {
      for (auto &geometry : *object.geometry) {
        geometry.offset_geometry_boundaries(vertex_offset);
        geometry.update_material(material_map);
        geometry.update_texture(texture_map, vertex_texture_map,
                                vertex_texture_offset);
      }
    }
    city_objects_.insert_or_assign(id, object);
  }

  append_vertices(feature.vertices());
  sorted_ids_.push_back(feature.id());
}

void CityJSON::remove_duplicate_vertices() {
  IndexMap remap;
  std::vector<std::vector<int64_t>> unique_vertices;
  unique_vertices.reserve(vertices_.size());

  std::unordered_map<std::string, std::size_t> seen;
  seen.reserve(vertices_.size());

  const auto to_key = [](const std::vector<int64_t> &vertex) {
    return std::to_string(vertex[0]) + ":" + std::to_string(vertex[1]) + ":" +
           std::to_string(vertex[2]);
  };

  for (std::size_t i = 0; i < vertices_.size(); ++i) {
    const auto &vertex = vertices_[i];
    const std::string key = to_key(vertex);
    const auto it = seen.find(key);
    if (it == seen.end()) {
      const std::size_t new_index = unique_vertices.size();
      unique_vertices.push_back(vertex);
      seen.emplace(key, new_index);
      remap.emplace(i, new_index);
    } else {
      remap.emplace(i, it->second);
    }
  }

  for (auto &[_, object] : city_objects_) {
    if (object.geometry) {
      for (auto &geometry : *object.geometry) {
        geometry.update_geometry_boundaries(remap);
      }
    }
  }

  vertices_ = std::move(unique_vertices);
}

void CityJSON::update_geographical_extent() {
  if (!metadata_ || !metadata_->geographical_extent) {
    return;
  }
  if (vertices_.empty()) {
    metadata_->geographical_extent =
        std::array<double, 6>{0.0, 0.0, 0.0, 0.0, 0.0, 0.0};
    return;
  }

  std::array<int64_t, 3> mins{std::numeric_limits<int64_t>::max(),
                              std::numeric_limits<int64_t>::max(),
                              std::numeric_limits<int64_t>::max()};
  std::array<int64_t, 3> maxs{std::numeric_limits<int64_t>::min(),
                              std::numeric_limits<int64_t>::min(),
                              std::numeric_limits<int64_t>::min()};

  for (const auto &vertex : vertices_) {
    for (std::size_t i = 0; i < 3; ++i) {
      mins[i] = std::min(mins[i], vertex[i]);
      maxs[i] = std::max(maxs[i], vertex[i]);
    }
  }

  auto &extent = metadata_->geographical_extent.value();
  extent[0] = mins[0] * transform_.scale[0] + transform_.translate[0];
  extent[1] = mins[1] * transform_.scale[1] + transform_.translate[1];
  extent[2] = mins[2] * transform_.scale[2] + transform_.translate[2];
  extent[3] = maxs[0] * transform_.scale[0] + transform_.translate[0];
  extent[4] = maxs[1] * transform_.scale[1] + transform_.translate[1];
  extent[5] = maxs[2] * transform_.scale[2] + transform_.translate[2];
}

void CityJSON::update_transform() {
  if (vertices_.empty()) {
    return;
  }

  std::array<int64_t, 3> mins{std::numeric_limits<int64_t>::max(),
                              std::numeric_limits<int64_t>::max(),
                              std::numeric_limits<int64_t>::max()};

  for (const auto &vertex : vertices_) {
    for (std::size_t i = 0; i < 3; ++i) {
      mins[i] = std::min(mins[i], vertex[i]);
    }
  }

  for (auto &vertex : vertices_) {
    vertex[0] -= mins[0];
    vertex[1] -= mins[1];
    vertex[2] -= mins[2];
  }

  transform_.translate[0] += mins[0] * transform_.scale[0];
  transform_.translate[1] += mins[1] * transform_.scale[1];
  transform_.translate[2] += mins[2] * transform_.scale[2];
}

CityJSON parse_cityjson(const std::string &json_text) {
  return CityJSON::parse(json_text);
}

} // namespace cjseq
