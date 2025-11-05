#include "cjseq/cityjson.hpp"

#include <cassert>
#include <cmath>
#include <iostream>
#include <optional>
#include <unordered_map>

namespace {

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

  std::cout << "All cjseq tests passed." << std::endl;
  return 0;
}
