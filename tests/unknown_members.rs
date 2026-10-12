//! One policy for members the schema does not name: objects the schema closes
//! (`additionalProperties: false`) reject them, every other object keeps them
//! verbatim in an `other` map.

use cjseq::{
    Appearance, CityJSON, CityJSONFeature, CityObject, Geometry, Metadata, PointOfContact,
    Semantics, SemanticsSurface, Transform,
};
use serde_json::{json, Map, Value};
use std::str::FromStr;

#[test]
fn a_geometry_rejects_a_member_the_schema_does_not_name() {
    let r = serde_json::from_value::<Geometry>(json!({
        "type": "MultiSurface", "lod": "2", "boundaries": [[[0, 1, 2]]], "colour": "red"
    }));
    assert!(r.is_err());
}

#[test]
fn a_geometry_instance_rejects_a_lod() {
    let r = serde_json::from_value::<Geometry>(json!({
        "type": "GeometryInstance", "lod": "2", "template": 0, "boundaries": [0],
        "transformationMatrix": [1.0,0.0,0.0,0.0, 0.0,1.0,0.0,0.0, 0.0,0.0,1.0,0.0, 0.0,0.0,0.0,1.0]
    }));
    assert!(r.is_err());
}

#[test]
fn closed_objects_reject_unknown_members() {
    assert!(serde_json::from_value::<Transform>(json!({
        "scale": [1.0, 1.0, 1.0], "translate": [0.0, 0.0, 0.0], "rotate": 1
    }))
    .is_err());
    assert!(serde_json::from_value::<Appearance>(json!({"foo": 1})).is_err());
    let doc = json!({
        "type": "CityJSON", "version": "2.0",
        "transform": {"scale": [1.0, 1.0, 1.0], "translate": [0.0, 0.0, 0.0]},
        "CityObjects": {}, "vertices": [],
        "geometry-templates": {"templates": [], "vertices-templates": [], "x": 1}
    });
    assert!(CityJSON::from_str(&doc.to_string()).is_err());
}

#[test]
fn a_feature_keeps_extra_root_members() {
    let input = r#"{"type":"CityJSONFeature","id":"a","CityObjects":{},"vertices":[],"+provenance":{"run":7}}"#;
    let f = CityJSONFeature::from_str(input).unwrap();
    assert_eq!(f.other["+provenance"], json!({"run": 7}));
    assert_eq!(serde_json::to_string(&f).unwrap(), input);
}

#[test]
fn a_document_keeps_extra_root_members_in_order() {
    let input = r#"{"type":"CityJSON","version":"2.0","transform":{"scale":[1.0,1.0,1.0],"translate":[0.0,0.0,0.0]},"CityObjects":{},"vertices":[],"+b":1,"+a":2}"#;
    let cj = CityJSON::from_str(input).unwrap();
    assert_eq!(serde_json::to_string(&cj).unwrap(), input);
}

#[test]
fn a_new_document_equals_its_parsed_twin() {
    let built = CityJSON::new();
    let parsed = CityJSON::from_str(&serde_json::to_string(&built).unwrap()).unwrap();
    assert_eq!(built, parsed);
    let built = CityJSONFeature::new();
    let parsed = CityJSONFeature::from_str(&serde_json::to_string(&built).unwrap()).unwrap();
    assert_eq!(built, parsed);
}

/// Every `other` is the same type, so callers handle extras one way.
#[test]
fn every_catch_all_is_a_json_object_map() {
    fn is_map(_: &Map<String, Value>) {}
    let cj = CityJSON::new();
    is_map(&cj.other);
    is_map(&CityJSONFeature::new().other);
    let co: CityObject = serde_json::from_value(json!({"type": "Building"})).unwrap();
    is_map(&co.other);
    let m: Metadata = serde_json::from_value(json!({})).unwrap();
    is_map(&m.other);
    let p: PointOfContact =
        serde_json::from_value(json!({"contactName": "a", "emailAddress": "b"})).unwrap();
    is_map(&p.other);
    let s: Semantics =
        serde_json::from_value(json!({"surfaces": [{"type": "RoofSurface"}], "values": null}))
            .unwrap();
    is_map(&s.other);
    let ss: &SemanticsSurface = &s.surfaces[0];
    is_map(&ss.other);
}
