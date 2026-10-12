use cjseq::Geometry;

#[test]
fn multisurface_accepts_three_levels() {
    let g: Geometry = serde_json::from_value(serde_json::json!({
        "type": "MultiSurface", "lod": "2",
        "boundaries": [[[0, 3, 2, 1]], [[4, 5, 6, 7]]]
    }))
    .expect("valid MultiSurface must deserialize");
    match g {
        Geometry::MultiSurface { ref boundaries, .. } => assert_eq!(boundaries.len(), 2),
        other => panic!("wrong variant: {other:?}"),
    }
}

#[test]
fn multisurface_rejects_solid_depth() {
    //-- one level too deep for a MultiSurface
    let r: Result<Geometry, _> = serde_json::from_value(serde_json::json!({
        "type": "MultiSurface", "lod": "2",
        "boundaries": [[[[0, 3, 2, 1]]]]
    }));
    assert!(r.is_err(), "wrong-depth boundaries must not deserialize");
}

#[test]
fn solid_roundtrips_through_json() {
    let input = serde_json::json!({
        "type": "Solid", "lod": "2",
        "boundaries": [[[[0, 3, 2, 1]], [[4, 5, 6, 7]]]]
    });
    let g: Geometry = serde_json::from_value(input.clone()).unwrap();
    assert_eq!(serde_json::to_value(&g).unwrap(), input);
}
