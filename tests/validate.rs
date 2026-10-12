//! `validate()` checks every reference a document makes: what serde alone
//! cannot express.

use cjseq::{CityJSON, CityJSONFeature, CjseqError, IndexKind};
use serde_json::{json, Value};
use std::str::FromStr;

fn doc(city_objects: Value, vertices: Value, extra: Value) -> CityJSON {
    let mut d = json!({
        "type": "CityJSON", "version": "2.0",
        "transform": {"scale": [1.0, 1.0, 1.0], "translate": [0.0, 0.0, 0.0]},
        "CityObjects": city_objects, "vertices": vertices
    });
    d.as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    CityJSON::from_str(&d.to_string()).unwrap()
}

const V4: [[i64; 3]; 4] = [[0, 0, 0], [1, 0, 0], [1, 1, 0], [0, 1, 0]];

fn building(geometry: Value) -> Value {
    json!({"b": {"type": "Building", "geometry": [geometry]}})
}

/// The error inside the `InCityObject` wrapper, checking the id.
fn inner(e: CjseqError, id: &str) -> CjseqError {
    match e {
        CjseqError::InCityObject { id: got, source } => {
            assert_eq!(got, id);
            *source
        }
        other => panic!("expected an error in {id}, got {other:?}"),
    }
}

fn first_error(cj: &CityJSON) -> CjseqError {
    cj.validate().expect_err("must not validate")
}

#[test]
fn a_well_formed_document_validates() {
    let cj = doc(
        json!({
            "b": {"type": "Building", "children": ["p"], "geometry": [{
                "type": "MultiSurface", "lod": "2", "boundaries": [[[0, 1, 2]], [[0, 2, 3]]],
                "semantics": {"surfaces": [{"type": "RoofSurface", "children": [1]}, {"type": "Window", "parent": 0}], "values": [0, null]},
                "material": {"m": {"values": [0, null]}, "n": {"value": 0}},
                "texture": {"t": {"values": [[[0, 0, 1, 2]], [[null]]]}}
            }]},
            "p": {"type": "BuildingPart", "parents": ["b"]}
        }),
        json!(V4),
        json!({"appearance": {
            "materials": [{"name": "m"}],
            "textures": [{"type": "PNG", "image": "a.png"}],
            "vertices-texture": [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]
        }}),
    );
    cj.validate().unwrap();
    assert!(cj.errors().is_empty());
}

#[test]
fn a_vertex_index_past_the_vertices_is_reported_with_its_object() {
    let cj = doc(
        building(json!({"type": "MultiPoint", "lod": "1", "boundaries": [0, 9]})),
        json!(V4),
        json!({}),
    );
    assert!(matches!(
        inner(first_error(&cj), "b"),
        CjseqError::IndexOutOfRange {
            kind: IndexKind::Vertex,
            index: 9,
            len: 4
        }
    ));
}

#[test]
fn appearance_indices_are_checked_against_the_palettes() {
    let tex = |ring: Value| {
        building(
            json!({"type": "MultiSurface", "lod": "1", "boundaries": [[[0, 1, 2]]],
            "texture": {"t": {"values": [[ring]]}}}),
        )
    };
    let app = json!({"appearance": {"textures": [{"type": "PNG", "image": "a.png"}], "vertices-texture": [[0.0, 0.0]]}});
    let e = inner(
        first_error(&doc(tex(json!([1, 0, 0, 0])), json!(V4), app.clone())),
        "b",
    );
    assert!(
        matches!(
            e,
            CjseqError::IndexOutOfRange {
                kind: IndexKind::Texture,
                index: 1,
                len: 1
            }
        ),
        "{e:?}"
    );
    let e = inner(
        first_error(&doc(tex(json!([0, 0, 5, 0])), json!(V4), app)),
        "b",
    );
    assert!(
        matches!(
            e,
            CjseqError::IndexOutOfRange {
                kind: IndexKind::TextureVertex,
                index: 5,
                len: 1
            }
        ),
        "{e:?}"
    );
    //-- a material reference with no appearance at all is out of range of an empty palette
    let e = inner(
        first_error(&doc(
            building(
                json!({"type": "MultiSurface", "lod": "1", "boundaries": [[[0, 1, 2]]], "material": {"m": {"value": 0}}}),
            ),
            json!(V4),
            json!({}),
        )),
        "b",
    );
    assert!(
        matches!(
            e,
            CjseqError::IndexOutOfRange {
                kind: IndexKind::Material,
                index: 0,
                len: 0
            }
        ),
        "{e:?}"
    );
}

#[test]
fn semantic_indices_are_checked_against_the_surfaces() {
    let sem = |s: Value| {
        building(
            json!({"type": "MultiSurface", "lod": "1", "boundaries": [[[0, 1, 2]]], "semantics": s}),
        )
    };
    for s in [
        json!({"surfaces": [{"type": "RoofSurface"}], "values": [1]}),
        json!({"surfaces": [{"type": "RoofSurface", "parent": 3}], "values": [0]}),
        json!({"surfaces": [{"type": "RoofSurface", "children": [2]}], "values": [0]}),
    ] {
        let e = inner(first_error(&doc(sem(s.clone()), json!(V4), json!({}))), "b");
        assert!(
            matches!(
                e,
                CjseqError::IndexOutOfRange {
                    kind: IndexKind::Surface,
                    ..
                }
            ),
            "{s}: {e:?}"
        );
    }
}

#[test]
fn value_counts_must_match_the_boundaries() {
    let g = |extra: Value| {
        let mut g =
            json!({"type": "Solid", "lod": "1", "boundaries": [[[[0, 1, 2]], [[0, 2, 3]]]]});
        g.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        building(g)
    };
    let app = json!({"appearance": {"materials": [{"name": "m"}], "textures": [{"image": "a.png"}],
        "vertices-texture": [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]}});
    for extra in [
        //-- one value for two surfaces
        json!({"semantics": {"surfaces": [{"type": "RoofSurface"}], "values": [[0]]}}),
        //-- two shells of values for one shell
        json!({"material": {"m": {"values": [[0, 0], [0, 0]]}}}),
        //-- a textured ring with two UVs for three vertices
        json!({"texture": {"t": {"values": [[[[0, 0, 1]], [[null]]]]}}}),
        //-- one surface of texture values for two
        json!({"texture": {"t": {"values": [[[[0, 0, 1, 2]]]]}}}),
    ] {
        let e = inner(
            first_error(&doc(g(extra.clone()), json!(V4), app.clone())),
            "b",
        );
        assert!(matches!(e, CjseqError::ValueCount { .. }), "{extra}: {e:?}");
    }
    //-- a null shell or surface is not a count mismatch
    doc(
        g(
            json!({"semantics": {"surfaces": [{"type": "RoofSurface"}], "values": [null]},
        "material": {"m": {"values": [[0, null]]}}}),
        ),
        json!(V4),
        app,
    )
    .validate()
    .unwrap();
}

#[test]
fn a_material_theme_has_exactly_one_of_value_and_values() {
    for m in [json!({}), json!({"value": 0, "values": [0]})] {
        let cj = doc(
            building(
                json!({"type": "MultiSurface", "lod": "1", "boundaries": [[[0, 1, 2]]], "material": {"m": m}}),
            ),
            json!(V4),
            json!({"appearance": {"materials": [{"name": "m"}]}}),
        );
        let e = inner(first_error(&cj), "b");
        assert!(matches!(e, CjseqError::Validation(_)), "{e:?}");
    }
}

#[test]
fn the_hierarchy_must_be_consistent() {
    let e = first_error(&doc(
        json!({"b": {"type": "Building", "children": ["x"]}}),
        json!([]),
        json!({}),
    ));
    assert!(
        matches!(e, CjseqError::DanglingReference { ref from, ref to } if from == "b" && to == "x"),
        "{e:?}"
    );
    let e = first_error(&doc(
        json!({"p": {"type": "BuildingPart", "parents": ["x"]}}),
        json!([]),
        json!({}),
    ));
    assert!(matches!(e, CjseqError::DanglingReference { .. }), "{e:?}");
    //-- b lists p as a child, but p does not name b as a parent
    let e = first_error(&doc(
        json!({"b": {"type": "Building", "children": ["p"]}, "p": {"type": "BuildingPart"}}),
        json!([]),
        json!({}),
    ));
    assert!(
        matches!(e, CjseqError::HierarchyMismatch { ref parent, ref child } if parent == "b" && child == "p"),
        "{e:?}"
    );
}

#[test]
fn geometry_instances_and_templates_are_checked() {
    let tm =
        json!([1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]);
    let templates = json!({"geometry-templates": {
        "templates": [{"type": "MultiPoint", "lod": "1", "boundaries": [0, 1]}],
        "vertices-templates": [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]
    }});
    let gi = |t: usize, v: usize| {
        building(
            json!({"type": "GeometryInstance", "template": t, "boundaries": [v], "transformationMatrix": tm}),
        )
    };
    doc(gi(0, 3), json!(V4), templates.clone())
        .validate()
        .unwrap();
    let e = inner(
        first_error(&doc(gi(1, 0), json!(V4), templates.clone())),
        "b",
    );
    assert!(
        matches!(
            e,
            CjseqError::IndexOutOfRange {
                kind: IndexKind::Template,
                index: 1,
                len: 1
            }
        ),
        "{e:?}"
    );
    let e = inner(first_error(&doc(gi(0, 4), json!(V4), templates)), "b");
    assert!(
        matches!(
            e,
            CjseqError::IndexOutOfRange {
                kind: IndexKind::Vertex,
                index: 4,
                len: 4
            }
        ),
        "{e:?}"
    );
    //-- a template's own boundaries index vertices-templates
    let bad = json!({"geometry-templates": {
        "templates": [{"type": "MultiPoint", "lod": "1", "boundaries": [0, 2]}],
        "vertices-templates": [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]
    }});
    let e = first_error(&doc(json!({}), json!([]), bad));
    assert!(
        matches!(e, CjseqError::InTemplate { index: 0, .. }),
        "{e:?}"
    );
}

#[test]
fn every_error_is_reported_not_only_the_first() {
    let cj = doc(
        json!({
            "a": {"type": "Building", "geometry": [{"type": "MultiPoint", "lod": "1", "boundaries": [7]}]},
            "b": {"type": "Building", "geometry": [{"type": "MultiPoint", "lod": "1", "boundaries": [8]}]}
        }),
        json!([]),
        json!({}),
    );
    assert_eq!(cj.errors().len(), 2);
}

#[test]
fn a_feature_is_checked_on_its_own() {
    let f = |v: Value| CityJSONFeature::from_str(&v.to_string()).unwrap();
    f(json!({"type": "CityJSONFeature", "id": "b", "CityObjects": {"b": {"type": "Building"}}, "vertices": []}))
        .validate()
        .unwrap();
    //-- the id must name one of its City Objects
    let e = f(json!({"type": "CityJSONFeature", "id": "x", "CityObjects": {"b": {"type": "Building"}}, "vertices": []}))
        .validate()
        .unwrap_err();
    assert!(matches!(e, CjseqError::DanglingReference { .. }), "{e:?}");
    //-- vertex indices against its own vertices
    let e = f(
        json!({"type": "CityJSONFeature", "id": "b", "CityObjects": {"b": {"type": "Building",
        "geometry": [{"type": "MultiPoint", "lod": "1", "boundaries": [0]}]}}, "vertices": []}),
    )
    .validate()
    .unwrap_err();
    assert!(matches!(
        inner(e, "b"),
        CjseqError::IndexOutOfRange {
            kind: IndexKind::Vertex,
            ..
        }
    ));
    //-- and the root members only the document may carry
    for m in [
        "transform",
        "version",
        "metadata",
        "geometry-templates",
        "extensions",
    ] {
        let mut v = json!({"type": "CityJSONFeature", "id": "b", "CityObjects": {"b": {"type": "Building"}}, "vertices": []});
        v[m] = json!({});
        let e = f(v).validate().unwrap_err();
        assert!(
            matches!(e, CjseqError::MisplacedMember { ref member } if member == m),
            "{m}: {e:?}"
        );
    }
}

/// Every fixture is valid, and so is everything `collect` and `cat` make
/// from it.
#[test]
fn every_fixture_and_everything_made_from_it_validates() {
    let mut seqs: Vec<std::path::PathBuf> = std::fs::read_dir("tests/data")
        .unwrap()
        .chain(std::fs::read_dir("data").unwrap())
        .map(|e| e.unwrap().path())
        .collect();
    seqs.sort();
    let mut checked = 0;
    for path in seqs {
        let name = path.display().to_string();
        let text = std::fs::read_to_string(&path).unwrap();
        let docs: Vec<CityJSON> = if name.ends_with(".jsonl") {
            let mut lines = text.lines();
            let header = CityJSON::from_str(lines.next().unwrap()).unwrap();
            header
                .validate()
                .unwrap_or_else(|e| panic!("{name} header: {e}"));
            let features: Vec<CityJSONFeature> = lines
                .map(|l| CityJSONFeature::from_str(l).unwrap())
                .collect();
            let (bad, features): (Vec<_>, Vec<_>) = features.into_iter().partition(|f| {
                name.ends_with("empty_appearance.city.jsonl") && f.id == "null_indices"
            });
            //-- this fixture exercises null material indices, and its one
            //-- non-null index points into a materials palette nobody
            //-- defined: it really is invalid, and must be reported so
            for f in &bad {
                let e = f.validate().unwrap_err();
                assert!(
                    e.to_string()
                        .contains("Material index 0 is out of range (there are 0)"),
                    "{e}"
                );
            }
            for f in &features {
                f.validate()
                    .unwrap_or_else(|e| panic!("{name} feature {}: {e}", f.id));
            }
            vec![cjseq::cjseq_to_cj(header, features).unwrap()]
        } else if name.ends_with(".json") {
            vec![CityJSON::from_str(&text).unwrap()]
        } else {
            continue;
        };
        for cj in docs {
            cj.validate().unwrap_or_else(|e| panic!("{name}: {e}"));
            for f in cj.features(cjseq::SortingStrategy::AsIs) {
                let f = f.unwrap();
                f.validate()
                    .unwrap_or_else(|e| panic!("{name} sliced feature {}: {e}", f.id));
            }
            checked += 1;
        }
    }
    assert!(checked >= 10, "only {checked} fixtures found");
}

/// The header `cat` writes slices the appearance down to what the geometry
/// templates use, so the templates it writes must be renumbered to match.
#[test]
fn the_header_renumbers_its_templates_to_the_sliced_appearance() {
    let cj = doc(
        json!({}),
        json!([]),
        json!({
            "appearance": {"materials": [{"name": "a"}, {"name": "b"}, {"name": "c"}]},
            "geometry-templates": {
                "templates": [{"type": "MultiSurface", "lod": "1", "boundaries": [[[0, 1, 2]]],
                    "material": {"visual": {"value": 2}}}],
                "vertices-templates": [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]]
            }
        }),
    );
    let header = cj.header().unwrap();
    header.validate().unwrap();
    let v = serde_json::to_value(&header).unwrap();
    assert_eq!(v["appearance"]["materials"], json!([{"name": "c"}]));
    assert_eq!(
        v["geometry-templates"]["templates"][0]["material"]["visual"]["value"],
        json!(0)
    );
}

/// The schemas' name patterns: an Extension City Object type is `+` then a
/// capital letter then word characters, an Extension semantic surface type
/// `+` then word characters, and an Extension's version `X.Y` (or `X.Y.Z`).
#[test]
fn extension_names_and_versions_follow_the_schema_patterns() {
    let co = |t: &str| doc(json!({"x": {"type": t}}), json!([]), json!({}));
    co("+NoiseBuilding").validate().unwrap();
    for bad in ["+noiseBuilding", "+N", "+1Building"] {
        let e = inner(first_error(&co(bad)), "x");
        assert!(
            matches!(e, CjseqError::Pattern { ref value, .. } if value == bad),
            "{bad}: {e:?}"
        );
    }

    let sem = |t: &str| {
        building(
            json!({"type": "MultiSurface", "lod": "1", "boundaries": [[[0, 1, 2]]],
            "semantics": {"surfaces": [{"type": t}], "values": [0]}}),
        )
    };
    doc(sem("+thermal_surface"), json!(V4), json!({}))
        .validate()
        .unwrap();
    let e = inner(first_error(&doc(sem("+-x"), json!(V4), json!({}))), "b");
    assert!(
        matches!(e, CjseqError::Pattern { ref value, .. } if value == "+-x"),
        "{e:?}"
    );

    let ext = |v: &str| {
        doc(
            json!({}),
            json!([]),
            json!({"extensions": {"Noise": {"url": "https://x.org/n.json", "version": v}}}),
        )
    };
    ext("2.0").validate().unwrap();
    ext("0.10.1").validate().unwrap();
    for bad in ["2", "v2.0", "2.0.", "02.0", "2.x"] {
        let e = first_error(&ext(bad));
        assert!(
            matches!(e, CjseqError::Pattern { ref value, .. } if value == bad),
            "{bad}: {e:?}"
        );
    }
}

/// § 8.7 rule 3: a core City Object should not have an Extension City
/// Object as a child. The Noise Extension published with the spec does
/// exactly that, so it is a warning, not an error: `validate()` passes and
/// `warnings()` reports it.
#[test]
fn an_extension_child_under_a_core_city_object_is_a_warning() {
    let cj = doc(
        json!({"b": {"type": "Building", "children": ["x"]},
               "x": {"type": "+Balcony", "parents": ["b"]}}),
        json!([]),
        json!({}),
    );
    cj.validate().unwrap();
    let w = cj.warnings();
    assert_eq!(w.len(), 1, "{w:?}");
    assert!(
        matches!(&w[0], CjseqError::ExtensionChildOfCoreObject { parent, child } if parent == "b" && child == "x"),
        "{w:?}"
    );
    //-- an Extension parent may have either kind of child
    let ok = doc(
        json!({"f": {"type": "+FancyBuilding", "children": ["x", "p"]},
               "x": {"type": "+Balcony", "parents": ["f"]},
               "p": {"type": "BuildingPart", "parents": ["f"]}}),
        json!([]),
        json!({}),
    );
    assert!(ok.warnings().is_empty());
    //-- a CityObjectGroup's children are its members, not a hierarchy
    let group = doc(
        json!({"g": {"type": "CityObjectGroup", "children": ["x"]},
               "x": {"type": "+Balcony", "parents": ["g"]}}),
        json!([]),
        json!({}),
    );
    assert!(group.warnings().is_empty());
}

#[test]
fn the_noise_extension_example_is_valid_with_a_warning_per_segment() {
    let text = std::fs::read_to_string("tests/data/noise_extension.city.jsonl").unwrap();
    let mut lines = text.lines();
    let header = CityJSON::from_str(lines.next().unwrap()).unwrap();
    let mut c = cjseq::Collector::new(header);
    for l in lines {
        let f = CityJSONFeature::from_str(l).unwrap();
        f.validate().unwrap();
        c.add(f).unwrap();
    }
    let cj = c.finish();
    cj.validate().unwrap();
    let w = cj.warnings();
    assert_eq!(w.len(), 2, "{w:?}");
    assert!(w.iter().all(|e| matches!(e, CjseqError::ExtensionChildOfCoreObject { parent, .. } if parent == "a_noisy_bench")));
}
