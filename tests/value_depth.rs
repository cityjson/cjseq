//! Semantics, material and texture values are nested exactly one level less
//! deeply than the geometry's boundaries (texture values: exactly as deeply),
//! and the geometry type decides which depth that is.

use cjseq::{
    Geometry, MaterialIndex, MultiSolidValues, PrimitiveValues, SolidValues, SurfaceIndex,
    TexturedRing,
};
use serde_json::{json, Value};

fn parse(v: Value) -> Result<Geometry, serde_json::Error> {
    serde_json::from_value(v)
}

fn round_trips(v: Value) {
    let g = parse(v.clone()).unwrap_or_else(|e| panic!("{v} must parse: {e}"));
    assert_eq!(serde_json::to_value(&g).unwrap(), v);
}

const SURFACES: [[[usize; 3]; 1]; 2] = [[[0, 1, 2]], [[2, 1, 3]]];

#[test]
fn semantics_values_must_have_the_depth_the_geometry_type_implies() {
    let sem = |values: Value| json!({"surfaces": [{"type": "RoofSurface"}], "values": values});
    //-- MultiSurface: one per surface
    assert!(parse(json!({"type": "MultiSurface", "lod": "2", "boundaries": SURFACES, "semantics": sem(json!([0, null]))})).is_ok());
    assert!(parse(json!({"type": "MultiSurface", "lod": "2", "boundaries": SURFACES, "semantics": sem(json!([[0, null]]))})).is_err());
    //-- Solid: per shell, per surface
    assert!(parse(json!({"type": "Solid", "lod": "2", "boundaries": [SURFACES], "semantics": sem(json!([[0, null]]))})).is_ok());
    assert!(parse(json!({"type": "Solid", "lod": "2", "boundaries": [SURFACES], "semantics": sem(json!([0, null]))})).is_err());
    assert!(parse(json!({"type": "Solid", "lod": "2", "boundaries": [SURFACES], "semantics": sem(json!([[[0, null]]]))})).is_err());
    //-- MultiSolid: per solid, per shell, per surface
    assert!(parse(json!({"type": "MultiSolid", "lod": "2", "boundaries": [[SURFACES]], "semantics": sem(json!([[[0, null]]]))})).is_ok());
    assert!(parse(json!({"type": "MultiSolid", "lod": "2", "boundaries": [[SURFACES]], "semantics": sem(json!([[0, null]]))})).is_err());
}

#[test]
fn material_values_must_have_the_depth_the_geometry_type_implies() {
    let mat = |values: Value| json!({"m": {"values": values}});
    assert!(parse(json!({"type": "CompositeSurface", "lod": "2", "boundaries": SURFACES, "material": mat(json!([0, 1]))})).is_ok());
    assert!(parse(json!({"type": "CompositeSurface", "lod": "2", "boundaries": SURFACES, "material": mat(json!([[0, 1]]))})).is_err());
    assert!(parse(json!({"type": "Solid", "lod": "2", "boundaries": [SURFACES], "material": mat(json!([0, 1]))})).is_err());
    assert!(parse(json!({"type": "CompositeSolid", "lod": "2", "boundaries": [[SURFACES]], "material": mat(json!([[0, 1]]))})).is_err());
}

#[test]
fn texture_values_must_have_the_depth_the_geometry_type_implies() {
    let tex = |values: Value| json!({"t": {"values": values}});
    let surface = json!([[0, 0, 1, 2]]);
    assert!(parse(json!({"type": "MultiSurface", "lod": "2", "boundaries": SURFACES, "texture": tex(json!([surface, surface]))})).is_ok());
    assert!(parse(json!({"type": "MultiSurface", "lod": "2", "boundaries": SURFACES, "texture": tex(json!([[surface, surface]]))})).is_err());
    assert!(parse(json!({"type": "Solid", "lod": "2", "boundaries": [SURFACES], "texture": tex(json!([[surface, surface]]))})).is_ok());
    assert!(parse(json!({"type": "Solid", "lod": "2", "boundaries": [SURFACES], "texture": tex(json!([surface, surface]))})).is_err());
}

#[test]
fn nulls_round_trip_at_every_level_the_schema_allows() {
    let sem = |values: Value| json!({"surfaces": [{"type": "RoofSurface"}], "values": values});
    round_trips(
        json!({"type": "MultiSolid", "lod": "2", "boundaries": [[SURFACES], [SURFACES]],
        "semantics": sem(json!([[[0, null]], null]))}),
    );
    round_trips(
        json!({"type": "Solid", "lod": "2", "boundaries": [SURFACES, SURFACES],
        "semantics": sem(json!([null, [null, 0]]))}),
    );
    round_trips(
        json!({"type": "Solid", "lod": "2", "boundaries": [SURFACES],
        "semantics": sem(Value::Null)}),
    );
    round_trips(
        json!({"type": "Solid", "lod": "2", "boundaries": [SURFACES],
        "material": {"m": {"values": [[null, 1]]}, "n": {"values": null}, "o": {"value": 2}}}),
    );
}

#[test]
fn empty_values_are_unambiguous_now() {
    let sem = |values: Value| json!({"surfaces": [], "values": values});
    round_trips(
        json!({"type": "MultiSurface", "lod": "2", "boundaries": [], "semantics": sem(json!([]))}),
    );
    round_trips(
        json!({"type": "Solid", "lod": "2", "boundaries": [], "semantics": sem(json!([]))}),
    );
    round_trips(
        json!({"type": "Solid", "lod": "2", "boundaries": [[]], "semantics": sem(json!([[]]))}),
    );
}

#[test]
fn each_variant_exposes_its_own_value_types() {
    let g = parse(
        json!({"type": "Solid", "lod": "2", "boundaries": [SURFACES],
        "semantics": {"surfaces": [{"type": "RoofSurface"}], "values": [[0, null]]},
        "material": {"m": {"values": [[1, null]]}},
        "texture": {"t": {"values": [[[[0, 0, 1, 2]], [[null]]]]}}}),
    )
    .unwrap();
    let Geometry::Solid {
        semantics: Some(sem),
        material: Some(mat),
        texture: Some(tex),
        ..
    } = g
    else {
        panic!()
    };
    let v: SolidValues<SurfaceIndex> = sem.values.unwrap();
    assert_eq!(v, vec![Some(vec![Some(SurfaceIndex(0)), None])]);
    let m: &Option<Option<SolidValues<MaterialIndex>>> = &mat["m"].values;
    assert_eq!(
        m,
        &Some(Some(vec![Some(vec![Some(MaterialIndex(1)), None])]))
    );
    assert_eq!(
        tex["t"].values.as_ref().unwrap()[0][1][0],
        TexturedRing::Untextured
    );

    //-- and the shallower and deeper aliases exist for the other variants
    let _: PrimitiveValues<SurfaceIndex> = vec![];
    let _: MultiSolidValues<SurfaceIndex> = vec![];
}
