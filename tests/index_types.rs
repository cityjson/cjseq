//! Each kind of index has its own type, so a vertex index cannot be used where
//! a texture-vertex index is meant. In JSON they are plain integers.

use cjseq::{
    Geometry, MaterialIndex, MaterialReference, SemanticsSurface, SurfaceIndex, TemplateIndex,
    TextureIndex, TexturedRing, UvIndex, VertexIndex,
};
use serde_json::json;

#[test]
fn an_index_is_a_plain_integer_in_json() {
    assert_eq!(serde_json::to_value(VertexIndex(5)).unwrap(), json!(5));
    let i: UvIndex = serde_json::from_value(json!(7)).unwrap();
    assert_eq!(i, UvIndex(7));
    assert!(serde_json::from_value::<VertexIndex>(json!(-1)).is_err());
}

#[test]
fn boundaries_hold_vertex_indices() {
    let g: Geometry = serde_json::from_value(json!({
        "type": "MultiPoint", "lod": "1", "boundaries": [3, 4]
    }))
    .unwrap();
    let Geometry::MultiPoint { boundaries, .. } = g else {
        panic!()
    };
    assert_eq!(boundaries, vec![VertexIndex(3), VertexIndex(4)]);
}

#[test]
fn a_geometry_instance_holds_a_template_index() {
    let g: Geometry = serde_json::from_value(json!({
        "type": "GeometryInstance", "template": 2, "boundaries": [9],
        "transformationMatrix": [1.0,0.0,0.0,0.0, 0.0,1.0,0.0,0.0, 0.0,0.0,1.0,0.0, 0.0,0.0,0.0,1.0]
    }))
    .unwrap();
    let Geometry::GeometryInstance {
        template,
        boundaries,
        ..
    } = g
    else {
        panic!()
    };
    assert_eq!(template, TemplateIndex(2));
    assert_eq!(boundaries, [VertexIndex(9)]);
}

#[test]
fn appearance_and_semantics_indices_are_typed() {
    let ring: TexturedRing = serde_json::from_value(json!([0, 1, 2])).unwrap();
    assert_eq!(
        ring,
        TexturedRing::Textured {
            texture: TextureIndex(0),
            uvs: vec![UvIndex(1), UvIndex(2)]
        }
    );
    let s: SemanticsSurface =
        serde_json::from_value(json!({"type": "Window", "parent": 1, "children": [2]})).unwrap();
    assert_eq!(s.parent, Some(SurfaceIndex(1)));
    assert_eq!(s.children, Some(vec![SurfaceIndex(2)]));
    let m: MaterialReference = serde_json::from_value(json!({"value": 4})).unwrap();
    assert_eq!(m.value, Some(MaterialIndex(4)));
}
