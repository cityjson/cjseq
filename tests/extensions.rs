//! The four ways a CityJSON Extension extends the core model (§ 8): new root
//! properties, new attributes, new semantic surfaces, new City Objects. Each
//! must parse, validate, and survive slicing into a CityJSONSeq and
//! collecting back.

use cjseq::{CityJSON, CityObjectType, Collector, SemanticSurfaceType, SortingStrategy};
use serde_json::{json, Value};
use std::str::FromStr;

fn extended() -> Value {
    json!({
        "type": "CityJSON",
        "version": "2.0",
        "transform": {"scale": [1.0, 1.0, 1.0], "translate": [0.0, 0.0, 0.0]},
        "extensions": {
            "Noise": {"url": "https://someurl.org/noise.ext.json", "version": "2.0"},
            "Census": {"url": "https://someurl.org/census.ext.json", "version": "0.7"}
        },
        "CityObjects": {
            //-- case 2: extra attributes on a core City Object
            "b": {
                "type": "Building",
                "attributes": {
                    "storeysAboveGround": 2,
                    "+colour": {"rgba": [255, 255, 255, 1]},
                    "+area-parcel": {"value": 437, "uom": "m2"}
                },
                "geometry": [{
                    "type": "MultiSurface", "lod": "2",
                    "boundaries": [[[0, 1, 2]], [[0, 2, 3]]],
                    //-- case 3: a new semantic surface, with its own member
                    "semantics": {
                        "surfaces": [
                            {"type": "RoofSurface"},
                            {"type": "+ThermalSurface", "azimuth": 30.5}
                        ],
                        "values": [0, 1]
                    }
                }]
            },
            //-- case 4: a new 1st-level City Object, with a new 2nd-level child
            "n": {
                "type": "+NoiseBuilding",
                "attributes": {"buildingLDenMin": 55.0},
                "children": ["np"],
                "+noiseSource": {"kind": "road"},
                "geometry": [{"type": "MultiPoint", "lod": "1", "boundaries": [0, 1]}]
            },
            "np": {
                "type": "+NoiseBuildingPart",
                "parents": ["n"],
                "geometry": [{"type": "MultiPoint", "lod": "1", "boundaries": [2, 3]}]
            }
        },
        "vertices": [[0, 0, 0], [1, 0, 0], [1, 1, 0], [0, 1, 0]],
        //-- case 1: a new root property
        "+census": {"percent_men": 49.5, "percent_women": 51.5}
    })
}

#[test]
fn every_kind_of_extension_parses_validates_and_round_trips() {
    let input = extended();
    let cj = CityJSON::from_str(&input.to_string()).unwrap();
    cj.validate().unwrap();
    assert_eq!(serde_json::to_value(&cj).unwrap(), input);

    assert_eq!(cj.other["+census"]["percent_men"], json!(49.5));
    assert_eq!(cj.extensions.as_ref().unwrap()["Noise"].version, "2.0");
    let b = &cj.city_objects["b"];
    assert_eq!(
        b.attributes.as_ref().unwrap()["+colour"]["rgba"][0],
        json!(255)
    );
    let surfaces = b.geometry.as_ref().unwrap()[0].surfaces().unwrap();
    assert_eq!(
        surfaces[1].thetype,
        SemanticSurfaceType::Extension("+ThermalSurface".into())
    );
    assert_eq!(surfaces[1].other["azimuth"], json!(30.5));
    let n = &cj.city_objects["n"];
    assert_eq!(
        n.thetype,
        CityObjectType::Extension("+NoiseBuilding".into())
    );
    assert_eq!(n.other["+noiseSource"], json!({"kind": "road"}));
}

#[test]
fn every_kind_of_extension_survives_cat_and_collect() {
    let cj = CityJSON::from_str(&extended().to_string()).unwrap();
    let header = cj.header().unwrap();
    //-- root properties and the extension declarations travel in the header
    assert_eq!(header.other["+census"], cj.other["+census"]);
    assert_eq!(header.extensions, cj.extensions);

    let features: Vec<_> = cj
        .features(SortingStrategy::AsIs)
        .map(|f| f.unwrap())
        .collect();
    //-- the Extension child goes with its Extension parent
    assert_eq!(features.len(), 2);
    let noise = features.iter().find(|f| f.id == "n").unwrap();
    assert!(noise.city_objects.contains_key("np"));
    for f in &features {
        f.validate().unwrap();
    }

    let mut c = Collector::new(header);
    for f in features {
        c.add(f).unwrap();
    }
    let back = c.finish();
    back.validate().unwrap();
    assert_eq!(back, cj);
}

#[test]
fn filtering_by_an_extension_type_works() {
    let t: CityObjectType = serde_json::from_value(json!("+NoiseBuilding")).unwrap();
    let cj = CityJSON::from_str(&extended().to_string()).unwrap();
    let ids: Vec<String> = cj
        .features(SortingStrategy::AsIs)
        .map(|f| f.unwrap())
        .filter(|f| f.city_objects[&f.id].thetype == t)
        .map(|f| f.id)
        .collect();
    assert_eq!(ids, ["n"]);
}
