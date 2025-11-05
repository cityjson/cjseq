#include "cjseq/cityjson.hpp"

#include <cassert>
#include <iostream>

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
        "children": ["building-1-child"]
      },
      "building-1-child": {
        "type": "BuildingPart",
        "parents": ["building-1"]
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

  const auto children_keys = building.get_children_keys();
  assert(children_keys.size() == 1);
  assert(children_keys[0] == "building-1-child");

  assert(cityjson.number_of_city_objects() == 1);

  cityjson.sort_cjfeatures(cjseq::SortingStrategy::Lexicographical);
  const auto &sorted = cityjson.sorted_ids();
  assert(sorted.size() == 1);
  assert(sorted[0] == "building-1");

  std::cout << "All cjseq tests passed." << std::endl;
  return 0;
}
