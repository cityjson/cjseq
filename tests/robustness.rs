//! Malformed or unusual input is rejected with an error or handled, never a
//! panic, and output order is deterministic.

use cjseq::{CityJSON, CityJSONFeature, CjseqError, Geometry, SortingStrategy};
use serde_json::{json, Value};
use std::str::FromStr;

fn doc(city_objects: Value, vertices: Value) -> CityJSON {
    CityJSON::from_str(
        &json!({
            "type": "CityJSON", "version": "2.0",
            "transform": {"scale": [1.0, 1.0, 1.0], "translate": [10.0, 20.0, 30.0]},
            "CityObjects": city_objects,
            "vertices": vertices
        })
        .to_string(),
    )
    .unwrap()
}

fn features(cj: &mut CityJSON, order: SortingStrategy) -> Vec<CityJSONFeature> {
    cj.features(order).map(|f| f.unwrap()).collect()
}

#[test]
fn a_vertex_must_have_three_coordinates() {
    let s = json!({
        "type": "CityJSON", "version": "2.0",
        "transform": {"scale": [1.0, 1.0, 1.0], "translate": [0.0, 0.0, 0.0]},
        "CityObjects": {}, "vertices": [[1, 2]]
    })
    .to_string();
    assert!(CityJSON::from_str(&s).is_err());
    let f = json!({"type": "CityJSONFeature", "id": "a", "CityObjects": {}, "vertices": [[1, 2, 3, 4]]})
        .to_string();
    assert!(CityJSONFeature::from_str(&f).is_err());
}

#[test]
fn a_transform_must_have_three_components() {
    let s = json!({
        "type": "CityJSON", "version": "2.0",
        "transform": {"scale": [1.0, 1.0], "translate": [0.0, 0.0, 0.0]},
        "CityObjects": {}, "vertices": []
    })
    .to_string();
    assert!(CityJSON::from_str(&s).is_err());
}

#[test]
fn a_geometry_instance_refers_to_exactly_one_vertex() {
    let gi = |b: Value| {
        serde_json::from_value::<Geometry>(json!({
            "type": "GeometryInstance", "template": 0, "boundaries": b,
            "transformationMatrix": [1.0,0.0,0.0,0.0, 0.0,1.0,0.0,0.0, 0.0,0.0,1.0,0.0, 0.0,0.0,0.0,1.0]
        }))
    };
    assert!(gi(json!([3])).is_ok());
    assert!(gi(json!([1, 2])).is_err());
    assert!(gi(json!([])).is_err());
}

#[test]
fn vertices_templates_are_three_coordinate_points() {
    let s = json!({
        "type": "CityJSON", "version": "2.0",
        "transform": {"scale": [1.0, 1.0, 1.0], "translate": [0.0, 0.0, 0.0]},
        "CityObjects": {}, "vertices": [],
        "geometry-templates": {"templates": [], "vertices-templates": [[0.0, 1.0]]}
    })
    .to_string();
    assert!(CityJSON::from_str(&s).is_err());
}

#[test]
fn city_objects_keep_input_order_and_output_is_deterministic() {
    let ids = ["e", "b", "d", "a", "c"];
    let cos: Vec<String> = ids
        .iter()
        .map(|id| format!(r#""{id}":{{"type":"Building"}}"#))
        .collect();
    let input = format!(
        r#"{{"type":"CityJSON","version":"2.0","transform":{{"scale":[1.0,1.0,1.0],"translate":[0.0,0.0,0.0]}},"CityObjects":{{{}}},"vertices":[]}}"#,
        cos.join(",")
    );
    let mut cj = CityJSON::from_str(&input).unwrap();
    let got: Vec<String> = features(&mut cj, SortingStrategy::AsIs)
        .into_iter()
        .map(|f| f.id)
        .collect();
    assert_eq!(got, ids);
    //-- and serializing twice gives byte-identical output, in input order
    let a = serde_json::to_string(&cj).unwrap();
    let b = serde_json::to_string(&cj).unwrap();
    assert_eq!(a, b);
    let positions: Vec<usize> = ids
        .iter()
        .map(|id| a.find(&format!("\"{id}\":{{")).unwrap())
        .collect();
    assert!(positions.windows(2).all(|w| w[0] < w[1]), "{a}");
}

#[test]
fn a_feature_carries_every_descendant_not_just_children() {
    let mut cj = doc(
        json!({
            "b": {"type": "Building", "children": ["p"]},
            "p": {"type": "BuildingPart", "parents": ["b"], "children": ["i"]},
            "i": {"type": "BuildingInstallation", "parents": ["p"], "geometry": [
                {"type": "MultiPoint", "lod": "1", "boundaries": [0]}
            ]}
        }),
        json!([[1, 2, 3]]),
    );
    let fs = features(&mut cj, SortingStrategy::AsIs);
    assert_eq!(fs.len(), 1);
    let mut keys: Vec<&String> = fs[0].city_objects.keys().collect();
    keys.sort();
    assert_eq!(keys, ["b", "i", "p"]);
    assert_eq!(fs[0].vertices, vec![[1, 2, 3]]);
}

#[test]
fn a_parent_child_cycle_does_not_loop_forever() {
    let mut cj = doc(
        json!({
            "b": {"type": "Building", "children": ["p"]},
            "p": {"type": "BuildingPart", "parents": ["b"], "children": ["b"]}
        }),
        json!([]),
    );
    let fs = features(&mut cj, SortingStrategy::AsIs);
    assert_eq!(fs[0].city_objects.len(), 2);
}

#[test]
fn a_dangling_child_is_an_error_not_a_panic() {
    let cj = doc(
        json!({"b": {"type": "Building", "children": ["nope"]}}),
        json!([]),
    );
    match cj.feature("b") {
        Err(CjseqError::DanglingReference { from, to }) => {
            assert_eq!(from, "b");
            assert_eq!(to, "nope");
        }
        other => panic!("expected DanglingReference, got {other:?}"),
    }
}

#[test]
fn an_out_of_range_vertex_index_is_an_error_not_a_panic() {
    let cj = doc(
        json!({"b": {"type": "Building", "geometry": [{"type": "MultiPoint", "lod": "1", "boundaries": [0, 7]}]}}),
        json!([[1, 2, 3]]),
    );
    assert!(matches!(
        cj.feature("b"),
        Err(CjseqError::IndexOutOfRange { .. })
    ));
}

#[test]
fn an_index_past_the_last_feature_is_none() {
    let cj = doc(json!({"b": {"type": "Building"}}), json!([]));
    assert!(cj.feature("nope").unwrap().is_none());
}

#[test]
fn update_transform_on_no_vertices_leaves_the_transform_alone() {
    let mut cj = doc(json!({}), json!([]));
    cj.update_transform();
    assert_eq!(cj.transform.translate, [10.0, 20.0, 30.0]);
}

#[test]
fn update_geographicalextent_on_no_vertices_leaves_the_extent_alone() {
    let mut cj = CityJSON::from_str(
        &json!({
            "type": "CityJSON", "version": "2.0",
            "transform": {"scale": [1.0, 1.0, 1.0], "translate": [0.0, 0.0, 0.0]},
            "metadata": {"geographicalExtent": [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]},
            "CityObjects": {}, "vertices": []
        })
        .to_string(),
    )
    .unwrap();
    cj.update_geographicalextent();
    assert_eq!(
        cj.metadata.unwrap().geographical_extent,
        Some([1.0, 2.0, 3.0, 4.0, 5.0, 6.0])
    );
}

#[test]
fn the_centroid_of_no_vertices_is_none() {
    let f = CityJSONFeature::new();
    assert_eq!(f.centroid(), None);
    let f = CityJSONFeature::from_str(
        &json!({"type": "CityJSONFeature", "id": "a", "CityObjects": {}, "vertices": [[0, 0, 0], [2, 4, 6]]})
            .to_string(),
    )
    .unwrap();
    assert_eq!(f.centroid(), Some([1.0, 2.0, 3.0]));
}

/// Every City Object lands in exactly one feature: one that no feature can
/// reach (its parent is missing, or does not list it as a child) is an
/// error, not silently left out.
#[test]
fn a_city_object_in_no_feature_is_an_error_not_dropped() {
    let cj = doc(
        json!({
            "a": {"type": "Building"},
            "orphan": {"type": "BuildingPart", "parents": ["nope"]},
            "c": {"type": "BuildingPart", "parents": ["a"]}
        }),
        json!([]),
    );
    let results: Vec<_> = cj.features(SortingStrategy::AsIs).collect();
    assert_eq!(results.len(), 1, "one error, and nothing partial");
    match &results[0] {
        Err(CjseqError::NotInAnyFeature { ids }) => assert_eq!(ids, &["orphan", "c"]),
        other => panic!("expected NotInAnyFeature, got {other:?}"),
    }
}

/// A CityObjectGroup's members are not its parts (§ 2.5): each member is a
/// feature of its own, and the group's feature holds just the group, so a
/// member of two groups is written once.
#[test]
fn group_members_are_features_of_their_own() {
    let cj = doc(
        json!({
            "g1": {"type": "CityObjectGroup", "children": ["b"]},
            "g2": {"type": "CityObjectGroup", "children": ["b", "c"]},
            "b": {"type": "Building", "parents": ["g1", "g2"], "children": ["bp"]},
            "bp": {"type": "BuildingPart", "parents": ["b"]},
            "c": {"type": "Building", "parents": ["g2"]}
        }),
        json!([]),
    );
    cj.validate().unwrap();
    let fs = features(&mut cj.clone(), SortingStrategy::AsIs);
    let ids: Vec<Vec<&String>> = fs.iter().map(|f| f.city_objects.keys().collect()).collect();
    assert_eq!(ids, [vec!["g1"], vec!["g2"], vec!["b", "bp"], vec!["c"]]);
}
