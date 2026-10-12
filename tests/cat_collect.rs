//! `collect(cat(doc)) == doc`, at the source level.
//!
//! `cat` renumbers each feature's vertices and palettes, and `collect`
//! merges them and moves the transform, so the two documents are compared
//! with every index resolved to what it points at: a vertex to its
//! coordinates on the original document's grid, a material, texture or
//! texture vertex to the entry itself, a GeometryInstance's template to the
//! template. Everything else is compared as it is.

use cjseq::{CityJSON, Collector, SortingStrategy};
use serde_json::{json, Map, Value};
use std::str::FromStr;

fn cat_then_collect(doc: &CityJSON) -> CityJSON {
    let mut c = Collector::new(doc.header().unwrap());
    for f in doc.features(SortingStrategy::AsIs) {
        c.add(f.unwrap()).unwrap();
    }
    assert!(c.warnings().is_empty(), "{:?}", c.warnings());
    c.finish()
}

/// What each kind of index resolves to in one document.
struct Palettes<'a> {
    vertices: Vec<[i64; 3]>,
    materials: &'a [Value],
    textures: &'a [Value],
    uvs: &'a [Value],
    templates: &'a [Value],
    template_vertices: &'a [Value],
}

const NONE: &[Value] = &[];

fn palettes<'a>(doc: &'a Value, grid: &Value) -> Palettes<'a> {
    let t = &doc["transform"];
    let g = |k: &str, i: usize| grid[k][i].as_f64().unwrap();
    let vertices = doc["vertices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            [0, 1, 2].map(|i| {
                let real = v[i].as_f64().unwrap() * t["scale"][i].as_f64().unwrap()
                    + t["translate"][i].as_f64().unwrap();
                ((real - g("translate", i)) / g("scale", i)).round() as i64
            })
        })
        .collect();
    let arr = |v: &'a Value| v.as_array().map_or(NONE, |a| &a[..]);
    let app = &doc["appearance"];
    Palettes {
        vertices,
        materials: arr(&app["materials"]),
        textures: arr(&app["textures"]),
        uvs: arr(&app["vertices-texture"]),
        templates: arr(&doc["geometry-templates"]["templates"]),
        template_vertices: arr(&doc["geometry-templates"]["vertices-templates"]),
    }
}

/// Replace every vertex index in `b` (boundaries of any depth).
fn resolve_boundaries(b: &Value, vertex: &dyn Fn(usize) -> Value) -> Value {
    match b {
        Value::Array(a) => Value::Array(a.iter().map(|x| resolve_boundaries(x, vertex)).collect()),
        Value::Number(n) => vertex(n.as_u64().unwrap() as usize),
        other => other.clone(),
    }
}

fn resolve_material_values(v: &Value, p: &Palettes) -> Value {
    match v {
        Value::Array(a) => Value::Array(a.iter().map(|x| resolve_material_values(x, p)).collect()),
        Value::Number(n) => p.materials[n.as_u64().unwrap() as usize].clone(),
        other => other.clone(),
    }
}

fn resolve_texture_values(v: &Value, p: &Palettes) -> Value {
    match v {
        Value::Array(a) if a.first().is_some_and(|x| !x.is_array()) => {
            //-- a ring: [texture, uv, uv, ...] or [null]
            match a[0].as_u64() {
                None => Value::Array(vec![Value::Null]),
                Some(t) => {
                    let mut out = vec![p.textures[t as usize].clone()];
                    out.extend(
                        a[1..]
                            .iter()
                            .map(|uv| p.uvs[uv.as_u64().unwrap() as usize].clone()),
                    );
                    Value::Array(out)
                }
            }
        }
        Value::Array(a) => Value::Array(a.iter().map(|x| resolve_texture_values(x, p)).collect()),
        other => other.clone(),
    }
}

fn resolve_geometry(g: &Value, p: &Palettes, vertex: &dyn Fn(usize) -> Value) -> Value {
    let mut g = g.as_object().unwrap().clone();
    if let Some(b) = g.get("boundaries") {
        g.insert("boundaries".into(), resolve_boundaries(b, vertex));
    }
    if let Some(Value::Object(themes)) = g.get_mut("material") {
        for m in themes.values_mut() {
            let m = m.as_object_mut().unwrap();
            for k in ["value", "values"] {
                if let Some(v) = m.get(k) {
                    let r = resolve_material_values(v, p);
                    m.insert(k.into(), r);
                }
            }
        }
    }
    if let Some(Value::Object(themes)) = g.get_mut("texture") {
        for t in themes.values_mut() {
            if let Some(v) = t.get("values") {
                let r = resolve_texture_values(v, p);
                t["values"] = r;
            }
        }
    }
    if let Some(t) = g.get("template").and_then(Value::as_u64) {
        let tv = |i: usize| p.template_vertices[i].clone();
        g.insert(
            "template".into(),
            resolve_geometry(&p.templates[t as usize], p, &tv),
        );
    }
    Value::Object(g)
}

/// `doc` with every index resolved, its palettes and vertices gone, and the
/// geographical extent (which `collect` recomputes) left out.
fn resolved(doc: &CityJSON, grid: &Value) -> Value {
    let v = serde_json::to_value(doc).unwrap();
    let p = palettes(&v, grid);
    let vertex = |i: usize| json!(p.vertices[i]);
    let mut out = v.as_object().unwrap().clone();
    for k in ["vertices", "transform", "geometry-templates"] {
        out.remove(k);
    }
    if let Some(Value::Object(app)) = out.get_mut("appearance") {
        for k in ["materials", "textures", "vertices-texture"] {
            app.remove(k);
        }
        if app.is_empty() {
            out.remove("appearance");
        }
    }
    if let Some(Value::Object(m)) = out.get_mut("metadata") {
        m.remove("geographicalExtent");
    }
    let mut cos = Map::new();
    for (id, co) in v["CityObjects"].as_object().unwrap() {
        let mut co = co.as_object().unwrap().clone();
        if let Some(Value::Array(gs)) = co.get("geometry") {
            let gs = gs
                .iter()
                .map(|g| resolve_geometry(g, &p, &vertex))
                .collect();
            co.insert("geometry".into(), Value::Array(gs));
        }
        if let Some(Value::Array(addrs)) = co.get("address") {
            let addrs = addrs
                .iter()
                .map(|a| {
                    let mut a = a.as_object().unwrap().clone();
                    if let Some(l) = a.get("location") {
                        let r = resolve_geometry(l, &p, &vertex);
                        a.insert("location".into(), r);
                    }
                    Value::Object(a)
                })
                .collect();
            co.insert("address".into(), Value::Array(addrs));
        }
        cos.insert(id.clone(), Value::Object(co));
    }
    out.insert("CityObjects".into(), Value::Object(cos));
    Value::Object(out)
}

fn assert_round_trips(doc: &CityJSON, what: &str) {
    doc.validate()
        .unwrap_or_else(|e| panic!("{what} must be valid: {e}"));
    let back = cat_then_collect(doc);
    back.validate()
        .unwrap_or_else(|e| panic!("{what} collected: {e}"));
    let grid = serde_json::to_value(&doc.transform).unwrap();
    let (a, b) = (resolved(doc, &grid), resolved(&back, &grid));
    if a != b {
        let (a, b) = (a.as_object().unwrap(), b.as_object().unwrap());
        for k in a.keys().chain(b.keys()) {
            if a.get(k) != b.get(k) {
                if k == "CityObjects" {
                    for (id, co) in a[k].as_object().unwrap() {
                        let other = b[k]
                            .get(id)
                            .unwrap_or_else(|| panic!("{what}: {id} is missing"));
                        for (m, v) in co.as_object().unwrap() {
                            assert_eq!(
                                Some(v),
                                other.get(m),
                                "{what}: City Object {id}, member {m}"
                            );
                        }
                        assert_eq!(co, other, "{what}: City Object {id}");
                    }
                }
                assert_eq!(a.get(k), b.get(k), "{what}: member {k}");
            }
        }
    }
}

/// One document exercising what a sequence must carry: an address with a
/// location, default themes, materials, textures, a geometry template with
/// its own appearance, a CityObjectGroup, an Extension City Object with a
/// child, and root, attribute and semantic-surface extensions.
fn everything() -> CityJSON {
    let tm =
        json!([1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]);
    CityJSON::from_str(&json!({
        "type": "CityJSON", "version": "2.0",
        "transform": {"scale": [0.01, 0.01, 0.01], "translate": [100.0, 200.0, 0.0]},
        "metadata": {"title": "everything", "geographicalExtent": [100.0, 200.0, 0.0, 100.09, 200.09, 0.0]},
        "extensions": {"Noise": {"url": "https://x.org/noise.json", "version": "2.0"}},
        "+census": {"percent_men": 49.5},
        "CityObjects": {
            "b1": {
                "type": "Building",
                "attributes": {"+colour": [1, 2, 3, 4]},
                "address": [{"country": "NL", "location": {"type": "MultiPoint", "lod": "1", "boundaries": [8]}}],
                "children": ["b1p"],
                "geometry": [{
                    "type": "MultiSurface", "lod": "2",
                    "boundaries": [[[0, 1, 2]], [[0, 2, 3]]],
                    "semantics": {"surfaces": [{"type": "RoofSurface"}, {"type": "+ThermalSurface", "azimuth": 3.0}], "values": [0, 1]},
                    "material": {"irradiation": {"values": [1, null]}, "visual": {"value": 0}},
                    "texture": {"winter": {"values": [[[1, 0, 1, 2]], [[null]]]}}
                }]
            },
            "b1p": {
                "type": "BuildingPart", "parents": ["b1"],
                "geometry": [{"type": "MultiSurface", "lod": "2", "boundaries": [[[4, 5, 6]]],
                    "material": {"visual": {"value": 2}},
                    "texture": {"winter": {"values": [[[0, 3, 4, 5]]]}}}]
            },
            "b2": {
                "type": "Building", "parents": ["g"],
                "geometry": [{"type": "Solid", "lod": "1", "boundaries": [[[[0, 1, 2]], [[2, 3, 7]]]],
                    "material": {"visual": {"values": [[0, 2]]}}}]
            },
            "tree": {
                "type": "SolitaryVegetationObject", "parents": ["g"],
                "geometry": [{"type": "GeometryInstance", "template": 0, "boundaries": [9], "transformationMatrix": tm}]
            },
            "g": {"type": "CityObjectGroup", "children": ["b2", "tree"], "children_roles": ["a", null]},
            "n": {"type": "+NoiseBuilding", "children": ["np"], "+lden": 55,
                "geometry": [{"type": "MultiPoint", "lod": "1", "boundaries": [1, 3]}]},
            "np": {"type": "+NoiseBuildingPart", "parents": ["n"],
                "geometry": [{"type": "MultiLineString", "lod": "1", "boundaries": [[4, 7]]}]}
        },
        "vertices": [[0,0,0],[9,0,0],[9,9,0],[0,9,0],[1,1,0],[2,1,0],[2,2,0],[5,5,0],[3,4,0],[6,6,0]],
        "appearance": {
            "default-theme-material": "visual",
            "default-theme-texture": "winter",
            "materials": [{"name": "m0"}, {"name": "m1", "transparency": 0.5}, {"name": "m2", "diffuseColor": [0.1, 0.2, 0.3]}, {"name": "tm"}],
            "textures": [{"type": "PNG", "image": "a.png"}, {"type": "JPG", "image": "b.jpg"}, {"type": "PNG", "image": "t.png"}],
            "vertices-texture": [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.5, 0.5], [0.6, 0.5], [0.6, 0.6], [0.9, 0.9], [0.8, 0.8], [0.7, 0.7]]
        },
        "geometry-templates": {
            "templates": [{"type": "MultiSurface", "lod": "2", "boundaries": [[[0, 1, 2]]],
                "material": {"visual": {"value": 3}},
                "texture": {"winter": {"values": [[[2, 6, 7, 8]]]}}}],
            "vertices-templates": [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
        }
    })
    .to_string())
    .unwrap()
}

#[test]
fn a_document_using_everything_survives_cat_and_collect() {
    assert_round_trips(&everything(), "everything()");
}

/// Without geometry templates the header has no appearance of its own to
/// carry, so the default themes must still get through.
#[test]
fn a_document_without_templates_keeps_its_default_themes() {
    let mut doc = everything();
    doc.geometry_templates = None;
    doc.city_objects.shift_remove("tree");
    let g = doc.city_objects.get_mut("g").unwrap();
    g.children = Some(vec!["b2".into()]);
    g.children_roles = Some(vec![Some("a".into())]);
    assert_round_trips(&doc, "everything() without templates");
}

#[test]
fn every_document_fixture_survives_cat_and_collect() {
    for path in ["data/3dbag_b2.city.json", "data/1b_w_texture.city.json"] {
        let doc = CityJSON::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert_round_trips(&doc, path);
    }
}

/// The sequence fixtures, collected and then cat-and-collected again.
#[test]
fn every_sequence_fixture_survives_cat_and_collect() {
    let mut paths: Vec<_> = std::fs::read_dir("tests/data")
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.to_string_lossy().ends_with(".city.jsonl"))
        .collect();
    paths.sort();
    for path in paths {
        let name = path.display().to_string();
        if name.ends_with("empty_appearance.city.jsonl") {
            //-- one of its features is invalid on purpose (see validate.rs)
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let mut lines = text.lines();
        let mut c = Collector::new(CityJSON::from_str(lines.next().unwrap()).unwrap());
        for l in lines {
            c.add(l.parse().unwrap()).unwrap();
        }
        assert_round_trips(&c.finish(), &name);
    }
}
