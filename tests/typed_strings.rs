//! Strings the schema constrains are typed, so a value the schema forbids
//! fails to parse.

use cjseq::{
    CityJSON, CityJSONVersion, CityObject, CityObjectType, ContactRole, ContactType, Geometry, Lod,
    PointOfContact, SemanticSurfaceType, TextureReference, TexturedRing,
};
use serde_json::json;
use std::str::FromStr;

fn doc(extra: serde_json::Value) -> serde_json::Value {
    let mut d = json!({
        "type": "CityJSON", "version": "2.0",
        "transform": {"scale": [1.0, 1.0, 1.0], "translate": [0.0, 0.0, 0.0]},
        "CityObjects": {}, "vertices": []
    });
    d.as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    d
}

#[test]
fn lod_is_one_of_the_schemas_values() {
    for ok in ["0", "1", "2", "3", "0.0", "1.2", "2.2", "3.3"] {
        let l: Lod = serde_json::from_value(json!(ok)).unwrap();
        assert_eq!(serde_json::to_value(l).unwrap(), json!(ok), "{ok}");
    }
    for bad in ["4", "2.4", "2.", "x", "", "1.2.3", "02"] {
        assert!(
            serde_json::from_value::<Lod>(json!(bad)).is_err(),
            "{bad:?} must be rejected"
        );
    }
    let l: Lod = serde_json::from_value(json!("2.2")).unwrap();
    assert_eq!((l.level(), l.refinement()), (2, Some(2)));
    let l: Lod = serde_json::from_value(json!("1")).unwrap();
    assert_eq!((l.level(), l.refinement()), (1, None));
}

#[test]
fn a_geometry_requires_a_lod() {
    assert!(serde_json::from_value::<Geometry>(json!({
        "type": "MultiSurface", "boundaries": [[[0, 1, 2]]]
    }))
    .is_err());
    let g: Geometry = serde_json::from_value(json!({
        "type": "MultiSurface", "lod": "2.2", "boundaries": [[[0, 1, 2]]]
    }))
    .unwrap();
    assert_eq!(g.lod().map(|l| l.to_string()), Some("2.2".to_string()));
}

#[test]
fn version_is_one_the_crate_supports() {
    let cj = CityJSON::from_str(&doc(json!({})).to_string()).unwrap();
    assert_eq!(cj.version, CityJSONVersion::V2_0);
    let cj = CityJSON::from_str(&doc(json!({"version": "1.1"})).to_string()).unwrap();
    assert_eq!(cj.version, CityJSONVersion::V1_1);
    assert!(CityJSON::from_str(&doc(json!({"version": "9.9"})).to_string()).is_err());
    assert!(CityJSON::from_str(&doc(json!({"version": 2.0})).to_string()).is_err());
}

#[test]
fn attributes_are_an_object() {
    let co: CityObject =
        serde_json::from_value(json!({"type": "Building", "attributes": {"h": 3}})).unwrap();
    assert_eq!(co.attributes.unwrap()["h"], json!(3));
    assert!(
        serde_json::from_value::<CityObject>(json!({"type": "Building", "attributes": [1]}))
            .is_err()
    );
}

#[test]
fn extensions_name_a_url_and_a_version() {
    let cj = CityJSON::from_str(
        &doc(json!({"extensions": {"Noise": {"url": "https://x.org/noise.ext.json", "version": "2.0"}}}))
            .to_string(),
    )
    .unwrap();
    let ext = &cj.extensions.as_ref().unwrap()["Noise"];
    assert_eq!(ext.url, "https://x.org/noise.ext.json");
    assert_eq!(ext.version, "2.0");
    assert!(CityJSON::from_str(
        &doc(json!({"extensions": {"Noise": {"url": "https://x.org/noise.ext.json"}}})).to_string()
    )
    .is_err());
}

#[test]
fn an_unknown_type_without_a_plus_is_rejected() {
    assert!(serde_json::from_value::<CityObjectType>(json!("Buildng")).is_err());
    assert!(serde_json::from_value::<CityObjectType>(json!("+")).is_err());
    assert_eq!(
        serde_json::from_value::<CityObjectType>(json!("+Noise")).unwrap(),
        CityObjectType::Extension("+Noise".into())
    );
    assert!(serde_json::from_value::<SemanticSurfaceType>(json!("Roof")).is_err());
    assert_eq!(
        serde_json::from_value::<SemanticSurfaceType>(json!("+Thermal")).unwrap(),
        SemanticSurfaceType::Extension("+Thermal".into())
    );
}

#[test]
fn contact_type_and_role_are_the_spec_enums() {
    let p: PointOfContact = serde_json::from_value(json!({
        "contactName": "a", "emailAddress": "b",
        "contactType": "organization", "role": "co-author"
    }))
    .unwrap();
    assert_eq!(p.contact_type, Some(ContactType::Organization));
    assert_eq!(p.role, Some(ContactRole::CoAuthor));
    assert_eq!(
        serde_json::to_value(&p).unwrap()["role"],
        json!("co-author")
    );
    let p: PointOfContact = serde_json::from_value(json!({
        "contactName": "a", "emailAddress": "b", "role": "pointOfContact"
    }))
    .unwrap();
    assert_eq!(p.role, Some(ContactRole::PointOfContact));
    assert!(serde_json::from_value::<PointOfContact>(json!({
        "contactName": "a", "emailAddress": "b", "contactType": "robot"
    }))
    .is_err());
}

#[test]
fn a_texture_ring_is_untextured_or_a_texture_with_uvs() {
    let t: TextureReference =
        serde_json::from_value(json!({"values": [[[0, 10, 11, 12]], [[null]]]})).unwrap();
    let v = serde_json::to_value(&t).unwrap();
    assert_eq!(v, json!({"values": [[[0, 10, 11, 12]], [[null]]]}));
    let ring: TexturedRing = serde_json::from_value(json!([0, 10, 11, 12])).unwrap();
    assert_eq!(
        ring,
        TexturedRing::Textured {
            texture: 0,
            uvs: vec![10, 11, 12]
        }
    );
    let ring: TexturedRing = serde_json::from_value(json!([null])).unwrap();
    assert_eq!(ring, TexturedRing::Untextured);
    for bad in [
        json!([0, null, 1]),
        json!([null, 1]),
        json!([]),
        json!([null, null]),
    ] {
        assert!(
            serde_json::from_value::<TexturedRing>(bad.clone()).is_err(),
            "{bad}"
        );
    }
}
