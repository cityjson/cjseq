//! Texture coordinates must survive slicing a document into features and
//! collecting features back into a document.

use cjseq::{cjseq_to_cj, CityJSON, CityJSONFeature};
use serde_json::{json, Value};
use std::str::FromStr;

/// One textured MultiSurface feature. `uvs` is the feature's own
/// `vertices-texture` palette; `rings` lists, per surface, the palette
/// indices its exterior ring uses.
fn feature(id: &str, uvs: &[[f64; 2]], rings: &[[usize; 3]]) -> CityJSONFeature {
    let boundaries: Vec<Value> = (0..rings.len())
        .map(|i| json!([[3 * i, 3 * i + 1, 3 * i + 2]]))
        .collect();
    let values: Vec<Value> = rings
        .iter()
        .map(|r| json!([[0, r[0], r[1], r[2]]]))
        .collect();
    let vertices: Vec<Value> = (0..3 * rings.len() as i64)
        .map(|i| json!([i, i * 10, 0]))
        .collect();
    serde_json::from_value(json!({
        "type": "CityJSONFeature",
        "id": id,
        "CityObjects": {id: {"type": "Building", "geometry": [{
            "type": "MultiSurface", "lod": "2",
            "boundaries": boundaries,
            "texture": {"t": {"values": values}}
        }]}},
        "vertices": vertices,
        "appearance": {
            "textures": [{"type": "PNG", "image": "a.png"}],
            "vertices-texture": uvs
        }
    }))
    .unwrap()
}

/// The UV coordinates each surface of `id` resolves to, in surface order.
fn resolved_uvs(cj: &CityJSON, id: &str) -> Vec<Vec<[f64; 2]>> {
    let v = serde_json::to_value(cj).unwrap();
    let palette: Vec<[f64; 2]> =
        serde_json::from_value(v["appearance"]["vertices-texture"].clone()).unwrap();
    v["CityObjects"][id]["geometry"][0]["texture"]["t"]["values"]
        .as_array()
        .unwrap()
        .iter()
        .map(|surface| {
            surface[0].as_array().unwrap()[1..]
                .iter()
                .map(|i| palette[i.as_u64().unwrap() as usize])
                .collect()
        })
        .collect()
}

fn header() -> CityJSON {
    CityJSON::from_str(
        &json!({
            "type": "CityJSON", "version": "2.0",
            "transform": {"scale": [1.0, 1.0, 1.0], "translate": [0.0, 0.0, 0.0]},
            "CityObjects": {}, "vertices": []
        })
        .to_string(),
    )
    .unwrap()
}

#[test]
fn collecting_several_textured_features_keeps_every_uv() {
    let a_uvs = [[0.0, 0.0], [0.1, 0.0], [0.1, 0.1]];
    //-- b's palette is used out of first-use order, and reused across surfaces
    let b_uvs = [[0.5, 0.5], [0.6, 0.5], [0.6, 0.6], [0.5, 0.6]];
    let a = feature("a", &a_uvs, &[[0, 1, 2]]);
    let b = feature("b", &b_uvs, &[[3, 1, 2], [2, 0, 3]]);
    let c = feature("c", &a_uvs, &[[2, 1, 0]]);

    let cj = cjseq_to_cj(header(), vec![a, b, c]).unwrap();
    cj.validate().unwrap();

    assert_eq!(
        resolved_uvs(&cj, "a"),
        vec![vec![a_uvs[0], a_uvs[1], a_uvs[2]]]
    );
    assert_eq!(
        resolved_uvs(&cj, "b"),
        vec![
            vec![b_uvs[3], b_uvs[1], b_uvs[2]],
            vec![b_uvs[2], b_uvs[0], b_uvs[3]]
        ]
    );
    assert_eq!(
        resolved_uvs(&cj, "c"),
        vec![vec![a_uvs[2], a_uvs[1], a_uvs[0]]]
    );
}

#[test]
fn slicing_a_feature_out_keeps_only_the_uvs_it_uses() {
    let uvs = [[0.0, 0.0], [0.1, 0.0], [0.1, 0.1], [0.9, 0.9], [0.8, 0.8]];
    //-- uses palette entries 4, 1, 2 only
    let cj = cjseq_to_cj(header(), vec![feature("a", &uvs, &[[4, 1, 2]])]).unwrap();
    let cj = CityJSON::from_str(&serde_json::to_string(&cj).unwrap()).unwrap();

    let f = cj.feature("a").unwrap().unwrap();
    let app = f.appearance.as_ref().unwrap();
    assert_eq!(
        app.vertices_texture.as_deref(),
        Some(&[uvs[4], uvs[1], uvs[2]][..]),
        "a sliced feature's palette holds exactly the UVs it uses, in first-use order"
    );
}

#[test]
fn a_cat_then_collect_round_trip_keeps_every_uv() {
    let uvs = [[0.5, 0.5], [0.6, 0.5], [0.6, 0.6], [0.5, 0.6]];
    let doc = cjseq_to_cj(
        header(),
        vec![
            feature("a", &uvs, &[[3, 1, 2]]),
            feature("b", &uvs, &[[0, 2, 1], [1, 3, 0]]),
        ],
    )
    .unwrap();
    let expected_a = resolved_uvs(&doc, "a");
    let expected_b = resolved_uvs(&doc, "b");

    let doc = CityJSON::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
    let features: Vec<CityJSONFeature> = doc
        .features(cjseq::SortingStrategy::Lexicographical)
        .map(|f| f.unwrap())
        .collect();
    let back = cjseq_to_cj(doc.header().unwrap(), features).unwrap();
    back.validate().unwrap();

    assert_eq!(resolved_uvs(&back, "a"), expected_a);
    assert_eq!(resolved_uvs(&back, "b"), expected_b);
}

/// serde_json's default float parser can land one ulp off; with
/// `float_roundtrip` a coordinate survives a parse/serialize cycle exactly.
/// 107072.02818522777 is one the default parser reads as ...775.
#[test]
fn a_coordinate_float_round_trips_exactly() {
    let input = r#"{"type":"CityJSON","version":"2.0","transform":{"scale":[0.001,0.001,0.001],"translate":[107072.02818522777,446000.0,0.0]},"CityObjects":{},"vertices":[]}"#;
    let cj = CityJSON::from_str(input).unwrap();
    assert_eq!(
        cj.transform.translate[0],
        "107072.02818522777".parse::<f64>().unwrap()
    );
    assert!(serde_json::to_string(&cj)
        .unwrap()
        .contains("107072.02818522777"));
}
