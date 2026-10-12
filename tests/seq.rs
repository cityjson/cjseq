//! CityJSONSeq in and out: a document's features as an iterator, a Collector
//! for the reverse, and a streaming reader and writer.

use cjseq::{
    CityJSON, CityJSONFeature, CjseqError, Collector, Line, SeqReader, SeqWriter, SortingStrategy,
};
use serde_json::json;
use std::io::BufReader;
use std::str::FromStr;

fn read(path: &str) -> (CityJSON, Vec<CityJSONFeature>) {
    let mut header = None;
    let mut features = Vec::new();
    for line in SeqReader::new(BufReader::new(std::fs::File::open(path).unwrap())) {
        match line.unwrap() {
            Line::Header(h) => header = Some(h),
            Line::Feature(f) => features.push(f),
        }
    }
    (header.unwrap(), features)
}

fn collect(header: CityJSON, features: Vec<CityJSONFeature>) -> CityJSON {
    let mut c = Collector::new(header);
    for f in features {
        c.add(f).unwrap();
    }
    c.finish()
}

#[test]
fn a_line_is_a_header_or_a_feature() {
    let h = Line::from_str(
        r#"{"type":"CityJSON","version":"2.0","transform":{"scale":[1.0,1.0,1.0],"translate":[0.0,0.0,0.0]},"CityObjects":{},"vertices":[]}"#,
    )
    .unwrap();
    assert!(matches!(h, Line::Header(_)));
    let f = Line::from_str(r#"{"type":"CityJSONFeature","id":"a","CityObjects":{"a":{"type":"Building"}},"vertices":[]}"#).unwrap();
    assert!(matches!(f, Line::Feature(ref f) if f.id == "a"));
    //-- a broken feature reports the feature's problem, not a header's
    let e = Line::from_str(r#"{"type":"CityJSONFeature","id":"a","CityObjects":{"a":{"type":"Buildng"}},"vertices":[]}"#).unwrap_err();
    assert!(e.to_string().contains("Buildng"), "{e}");
    assert!(Line::from_str(r#"{"type":"Something"}"#).is_err());
}

#[test]
fn the_reader_says_which_line_is_bad() {
    let input = "{\"type\":\"CityJSON\",\"version\":\"2.0\",\"transform\":{\"scale\":[1.0,1.0,1.0],\"translate\":[0.0,0.0,0.0]},\"CityObjects\":{},\"vertices\":[]}\n\nnot json\n";
    let lines: Vec<_> = SeqReader::new(input.as_bytes()).collect();
    assert_eq!(lines.len(), 2, "a blank line is skipped");
    assert!(lines[0].is_ok());
    match &lines[1] {
        Err(CjseqError::AtLine { line, .. }) => assert_eq!(*line, 3),
        other => panic!("expected AtLine, got {other:?}"),
    }
}

#[test]
fn features_come_out_in_the_order_asked_for() {
    let cj = CityJSON::from_str(
        r#"{"type":"CityJSON","version":"2.0","transform":{"scale":[1.0,1.0,1.0],"translate":[0.0,0.0,0.0]},"CityObjects":{"c":{"type":"Building"},"a":{"type":"Building"},"b":{"type":"Building"}},"vertices":[]}"#,
    )
    .unwrap();
    let ids = |o| -> Vec<String> { cj.features(o).map(|f| f.unwrap().id).collect() };
    assert_eq!(ids(SortingStrategy::AsIs), ["c", "a", "b"]);
    assert_eq!(ids(SortingStrategy::Lexicographical), ["a", "b", "c"]);
    let mut r = ids(SortingStrategy::Random);
    r.sort();
    assert_eq!(r, ["a", "b", "c"]);
}

#[test]
fn collecting_and_slicing_round_trip_every_fixture() {
    for path in [
        "tests/data/small.city.jsonl",
        "tests/data/noise_extension.city.jsonl",
        "data/3dbag_b2.city.jsonl",
    ] {
        let (header, features) = read(path);
        let doc = collect(header, features.clone());
        doc.validate().unwrap();
        //-- slicing the collected document gives the same features back,
        //-- up to vertex and palette numbering, which compares equal once
        //-- collected again
        let back: Vec<CityJSONFeature> = doc
            .features(SortingStrategy::AsIs)
            .map(|f| f.unwrap())
            .collect();
        assert_eq!(back.len(), features.len(), "{path}");
        for (a, b) in back.iter().zip(&features) {
            assert_eq!(a.id, b.id);
            assert_eq!(a.city_objects.len(), b.city_objects.len());
        }
        let again = collect(doc.header().unwrap(), back);
        assert_eq!(again, doc, "{path}");
    }
}

#[test]
fn the_writer_writes_what_the_reader_reads() {
    let (header, features) = read("tests/data/small.city.jsonl");
    let mut out = Vec::new();
    {
        let mut w = SeqWriter::new(&mut out);
        w.write_header(&header).unwrap();
        for f in &features {
            w.write_feature(f).unwrap();
        }
    }
    let text = String::from_utf8(out).unwrap();
    assert_eq!(text.lines().count(), features.len() + 1);
    let (h2, f2) = {
        let mut header = None;
        let mut fs = Vec::new();
        for l in SeqReader::new(text.as_bytes()) {
            match l.unwrap() {
                Line::Header(h) => header = Some(h),
                Line::Feature(f) => fs.push(f),
            }
        }
        (header.unwrap(), fs)
    };
    assert_eq!(h2, header);
    assert_eq!(f2, features);
}

/// Features from sequences with different transforms keep their real
/// coordinates when collected into one document.
#[test]
fn a_second_header_re_expresses_later_features_in_the_first_transform() {
    let header = |scale: f64, tx: f64| {
        CityJSON::from_str(
            &json!({"type": "CityJSON", "version": "2.0",
                "transform": {"scale": [scale, scale, scale], "translate": [tx, 0.0, 0.0]},
                "CityObjects": {}, "vertices": []})
            .to_string(),
        )
        .unwrap()
    };
    let feature = |id: &str, v: [i64; 3]| {
        CityJSONFeature::from_str(
            &json!({"type": "CityJSONFeature", "id": id,
                "CityObjects": {id: {"type": "Building", "geometry": [{"type": "MultiPoint", "lod": "1", "boundaries": [0]}]}},
                "vertices": [v]})
            .to_string(),
        )
        .unwrap()
    };
    let mut c = Collector::new(header(0.01, 100.0));
    c.add(feature("a", [100, 200, 300])).unwrap(); //-- real (101, 2, 3)
    c.add_header(header(0.001, 200.0)).unwrap();
    c.add(feature("b", [500, 1000, 1500])).unwrap(); //-- real (200.5, 1, 1.5)
    let doc = c.finish();
    let real = |id: &str| {
        let f = doc
            .features(SortingStrategy::AsIs)
            .map(|f| f.unwrap())
            .find(|f| f.id == id)
            .unwrap();
        let t = &doc.transform;
        let v = f.vertices[0];
        [0, 1, 2].map(|k| ((v[k] as f64 * t.scale[k] + t.translate[k]) * 1000.0).round() / 1000.0)
    };
    assert_eq!(real("a"), [101.0, 2.0, 3.0]);
    assert_eq!(real("b"), [200.5, 1.0, 1.5]);
}

#[test]
fn a_feature_whose_id_is_not_among_its_objects_cannot_be_collected() {
    let (header, _) = read("tests/data/small.city.jsonl");
    let mut c = Collector::new(header);
    let bad = CityJSONFeature::from_str(
        r#"{"type":"CityJSONFeature","id":"x","CityObjects":{"a":{"type":"Building"}},"vertices":[]}"#,
    )
    .unwrap();
    assert!(c.add(bad).is_err());
}

/// Collecting merges identical materials and textures into one palette entry,
/// including ones already in the header, and keeps different ones apart.
#[test]
fn collecting_merges_identical_palette_entries() {
    let header = CityJSON::from_str(
        &json!({"type": "CityJSON", "version": "2.0",
            "transform": {"scale": [1.0, 1.0, 1.0], "translate": [0.0, 0.0, 0.0]},
            "CityObjects": {}, "vertices": [],
            "appearance": {"materials": [{"name": "h", "diffuseColor": [0.0, 0.0, 0.0]}]}})
        .to_string(),
    )
    .unwrap();
    let feature = |id: &str, mat: serde_json::Value, tex: serde_json::Value| {
        CityJSONFeature::from_str(
            &json!({"type": "CityJSONFeature", "id": id,
                "CityObjects": {id: {"type": "Building", "geometry": [{
                    "type": "MultiSurface", "lod": "1", "boundaries": [[[0, 1, 2]]],
                    "material": {"m": {"value": 0}},
                    "texture": {"t": {"values": [[[0, 0, 1, 2]]]}}}]}},
                "vertices": [[0, 0, 0], [1, 0, 0], [0, 1, 0]],
                "appearance": {"materials": [mat], "textures": [tex],
                    "vertices-texture": [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]}})
            .to_string(),
        )
        .unwrap()
    };
    let png = json!({"type": "PNG", "image": "a.png", "borderColor": [0.0, 0.0, 0.0, 1.0]});
    let mut c = Collector::new(header);
    //-- the header's own material, spelled with a negative zero
    c.add(feature(
        "a",
        json!({"name": "h", "diffuseColor": [-0.0, 0.0, 0.0]}),
        png.clone(),
    ))
    .unwrap();
    c.add(feature("b", json!({"name": "x"}), png.clone()))
        .unwrap();
    c.add(feature(
        "c",
        json!({"name": "x"}),
        json!({"type": "JPG", "image": "a.png"}),
    ))
    .unwrap();
    let doc = c.finish();
    doc.validate().unwrap();
    let app = doc.appearance.as_ref().unwrap();
    assert_eq!(app.materials.as_ref().unwrap().len(), 2);
    assert_eq!(app.textures.as_ref().unwrap().len(), 2);
    let v = serde_json::to_value(&doc).unwrap();
    let m = |id: &str| v["CityObjects"][id]["geometry"][0]["material"]["m"]["value"].clone();
    assert_eq!((m("a"), m("b"), m("c")), (json!(0), json!(1), json!(1)));
}

/// A CityJSONFeature may carry root members the schema does not name
/// (`additionalProperties: true`), but a CityJSON document has nowhere to
/// put them per feature: collecting drops each and says so.
#[test]
fn collecting_reports_each_feature_root_member_it_drops() {
    let (header, _) = read("tests/data/small.city.jsonl");
    let mut c = Collector::new(header);
    let f = CityJSONFeature::from_str(
        r#"{"type":"CityJSONFeature","id":"a","CityObjects":{"a":{"type":"Building"}},"vertices":[],"+provenance":{"run":7},"+note":"x"}"#,
    )
    .unwrap();
    assert_eq!(f.other.len(), 2, "the feature itself keeps them");
    c.add(f).unwrap();
    let w = c.warnings();
    assert_eq!(w.len(), 2, "{w:?}");
    assert!(
        matches!(&w[0], CjseqError::DroppedMember { feature, member } if feature == "a" && member == "+provenance")
    );
    assert!(matches!(&w[1], CjseqError::DroppedMember { member, .. } if member == "+note"));
    let doc = c.finish();
    assert!(!doc.other.contains_key("+provenance"));
}

/// Default themes belong to the document: the header carries them, a
/// feature's are adopted when the document has none, and one that
/// disagrees with the document's is dropped with a warning.
#[test]
fn default_themes_travel_with_the_header_and_are_reconciled() {
    let doc = CityJSON::from_str(
        &json!({"type": "CityJSON", "version": "2.0",
            "transform": {"scale": [1.0, 1.0, 1.0], "translate": [0.0, 0.0, 0.0]},
            "CityObjects": {}, "vertices": [],
            "appearance": {"default-theme-material": "visual", "materials": [{"name": "m"}]}})
        .to_string(),
    )
    .unwrap();
    let header = doc.header().unwrap();
    let app = header.appearance.as_ref().unwrap();
    assert_eq!(app.default_theme_material.as_deref(), Some("visual"));
    assert!(
        app.materials.is_none(),
        "no templates, so no palette in the header"
    );

    let feature = |theme: &str| {
        CityJSONFeature::from_str(
            &json!({"type": "CityJSONFeature", "id": "a", "CityObjects": {"a": {"type": "Building"}},
                "vertices": [], "appearance": {"default-theme-material": theme, "default-theme-texture": "winter"}})
            .to_string(),
        )
        .unwrap()
    };
    let mut c = Collector::new(header);
    c.add(feature("summer")).unwrap();
    assert_eq!(c.warnings().len(), 1, "{:?}", c.warnings());
    assert!(c.warnings()[0].to_string().contains("summer"));
    let back = c.finish();
    let app = back.appearance.unwrap();
    assert_eq!(app.default_theme_material.as_deref(), Some("visual"));
    assert_eq!(app.default_theme_texture.as_deref(), Some("winter"));
}
