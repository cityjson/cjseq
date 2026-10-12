//! Real-file round trips.
//!
//! Every other test in this crate is synthetic: hand-built JSON literals
//! written from the spec. These are the first tests that put the typed model
//! in front of files nobody wrote for it. The shape matters: a reader
//! compared against another reader's output can agree on the wrong answer
//! (that is exactly how the two bugs motivating this rewrite stayed
//! invisible), but a parse -> serialize -> compare cycle cannot hide a
//! silently dropped member.
//!
//! Comparison is on parsed `serde_json::Value`s, not on strings: several
//! members are backed by `HashMap` and have no stable serialization order, so
//! a byte comparison would fail for reasons that are not data loss. `Value`
//! equality is order-insensitive for object members and exact for everything
//! else, which is the property this test wants.

use cjseq::{CityJSON, CityJSONFeature};
use serde_json::{Number, Value};
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Do two JSON numbers denote the same value?
///
/// `serde_json::Number` compares by representation, not by value: the `0` a
/// fixture writes in `metadata.geographicalExtent` and the `0.0` that comes
/// back out of an `f64`-typed field are `!=` even though JSON has a single
/// number type and both are the number zero. `geographicalExtent`'s items are
/// `{"type": "number"}` in `metadata.schema.json`, so integer-vs-decimal
/// spelling carries no information and re-spelling it is not data loss.
///
/// This is deliberately *not* `a.as_f64() == b.as_f64()`, which would also
/// call `9007199254740993` equal to `9007199254740992.0` and so hide the one
/// number difference that IS data loss -- an integer too large to survive a
/// round trip through `f64`. The integer must come back exactly.
fn same_number(a: &Number, b: &Number) -> bool {
    if a == b {
        return true;
    }
    let exact = |i: i64, f: f64| f == i as f64 && f as i64 == i;
    match (a.as_i64(), b.as_f64(), a.as_f64(), b.as_i64()) {
        (Some(i), Some(f), _, _) => exact(i, f),
        (_, _, Some(f), Some(i)) => exact(i, f),
        _ => false,
    }
}

/// CityJSONSeq fixtures: line 1 is a `CityJSON` header, every subsequent
/// line a `CityJSONFeature`.
///
/// Every fixture is round-tripped; nothing in `tests/data/` is excluded.
/// (`EXCLUDED` below exists so that stays true by assertion rather than by
/// good intentions -- see `every_fixture_on_disk_is_accounted_for`.)
///
/// `delft.city.jsonl` from flatcitybuf (6.6 MB) was not vendored at all: too
/// heavy to commit for the coverage it adds over `small.city.jsonl`, which
/// comes from the same 3DBAG pipeline.
fn seq_fixtures() -> Vec<PathBuf> {
    [
        //-- Extension City Object and semantic-surface types, spelled with a
        //-- leading `+`
        "tests/data/noise_extension.city.jsonl",
        //-- geometry templates, GeometryInstance, header-level appearance
        "tests/data/geom_temp.city.jsonl",
        //-- an `appearance` object with empty arrays in it
        "tests/data/empty_appearance.city.jsonl",
        "tests/data/small.city.jsonl",
        "tests/data/degenerate_extent.city.jsonl",
        //-- despite the name, this file contains no repeated JSON member
        //-- names at any depth (verified with an `object_pairs_hook` that
        //-- reports them). It is five structurally identical Building
        //-- features whose `grp` *attribute value* repeats -- a duplicate
        //-- grouping case, not a duplicate-key case. It was briefly excluded
        //-- on the strength of its filename; that was wrong.
        "tests/data/duplicate_keys.city.jsonl",
        "tests/data/inferable_types.city.jsonl",
        "tests/data/long_strings.city.jsonl",
        "tests/data/single_feature.city.jsonl",
        //-- already committed to this repo before this test existed
        "data/3dbag_b2.city.jsonl",
    ]
    .iter()
    .map(PathBuf::from)
    .collect()
}

/// Fixtures in `tests/data/` deliberately left out of the round trip, each
/// with the reason it cannot be round-tripped. Empty, and it should stay that
/// way: a fixture that cannot survive a round trip is a finding, not a
/// housekeeping detail.
const EXCLUDED: &[(&str, &str)] = &[];

/// Whole-document CityJSON fixtures (not a sequence).
fn cj_fixtures() -> Vec<PathBuf> {
    ["data/1b_w_texture.city.json", "data/3dbag_b2.city.json"]
        .iter()
        .map(PathBuf::from)
        .collect()
}

/// `assert_eq!` on two large `Value`s prints both in full, which for a
/// 60 kB feature is unreadable. Walk them instead and report the first
/// differing path.
fn first_difference(a: &Value, b: &Value, path: &str) -> Option<String> {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for (k, xv) in x {
                match y.get(k) {
                    Some(yv) => {
                        if let Some(d) = first_difference(xv, yv, &format!("{path}/{k}")) {
                            return Some(d);
                        }
                    }
                    None => return Some(format!("{path}/{k}: dropped on round trip ({xv})")),
                }
            }
            for k in y.keys() {
                if !x.contains_key(k) {
                    return Some(format!("{path}/{k}: invented on round trip ({})", y[k]));
                }
            }
            None
        }
        (Value::Array(x), Value::Array(y)) => {
            if x.len() != y.len() {
                return Some(format!("{path}: length {} became {}", x.len(), y.len()));
            }
            for (i, (xv, yv)) in x.iter().zip(y).enumerate() {
                if let Some(d) = first_difference(xv, yv, &format!("{path}[{i}]")) {
                    return Some(d);
                }
            }
            None
        }
        (Value::Number(x), Value::Number(y)) if same_number(x, y) => None,
        _ => {
            if a == b {
                None
            } else {
                Some(format!("{path}: {a} became {b}"))
            }
        }
    }
}

/// Rewrite every number as a canonical string, so that `0` and `0.0` become
/// the same `Value` while `9007199254740993` and `9007199254740992.0` stay
/// different.
///
/// `first_difference` above is the diagnostic; this exists so the assertion
/// does not rest on that hand-rolled walk being correct. It is a second,
/// independent implementation of the same comparison, and the two must agree.
fn canonical_numbers(v: &Value) -> Value {
    match v {
        Value::Number(n) => {
            let s = if let Some(i) = n.as_i64() {
                i.to_string()
            } else if let Some(u) = n.as_u64() {
                u.to_string()
            } else {
                let f = n.as_f64().expect("a JSON number is i64, u64, or f64");
                //-- an f64 holding an exact integral value canonicalizes the
                //-- same way an integer literal does, but only inside the
                //-- range where the conversion is lossless. That range ends
                //-- at 2^53, above which consecutive integers are no longer
                //-- all representable -- so spell 2^53, not an approximation
                //-- of it: a rounder constant slightly below it would make
                //-- integral values in the gap canonicalize differently
                //-- depending on whether they arrived float- or
                //-- integer-spelled, i.e. report a difference that isn't one.
                if f.fract() == 0.0 && f.abs() < 9007199254740992.0 {
                    (f as i64).to_string()
                } else {
                    format!("{f:?}")
                }
            };
            Value::String(s)
        }
        Value::Array(a) => Value::Array(a.iter().map(canonical_numbers).collect()),
        Value::Object(o) => Value::Object(
            o.iter()
                .map(|(k, v)| (k.clone(), canonical_numbers(v)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Parse `line` as `T`, re-serialize, and require the result to equal the
/// line's own parsed form. Anything that changes on the way through is data
/// loss.
fn assert_roundtrips<T>(line: &str, what: &str)
where
    T: serde::de::DeserializeOwned + serde::Serialize,
{
    let original: Value = serde_json::from_str(line).expect("fixture is not valid JSON");
    let typed: T = serde_json::from_str(line).unwrap_or_else(|e| panic!("{what}: {e}"));
    let reserialized = serde_json::to_value(&typed).unwrap_or_else(|e| panic!("{what}: {e}"));
    assert_equivalent(&original, &reserialized, what);
}

/// The assertion this whole file exists for: nothing may change on the way
/// through except the lexical spelling of a number.
fn assert_equivalent(original: &Value, reserialized: &Value, what: &str) {
    if let Some(diff) = first_difference(original, reserialized, "") {
        panic!("{what} changed on round trip: {diff}");
    }
    //-- `original` goes on the left so that assert_eq!'s "left"/"right"
    //-- labels read the way the failure does: left is what the file said,
    //-- right is what came back out.
    assert_eq!(
        canonical_numbers(original),
        canonical_numbers(reserialized),
        "{what} changed on round trip"
    );
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{}: {e} (run from the crate root)", path.display()))
}

#[test]
fn every_cityjsonseq_fixture_roundtrips_exactly() {
    for path in seq_fixtures() {
        let contents = read(&path);
        let mut lines = contents
            .lines()
            .enumerate()
            .filter(|(_, l)| !l.trim().is_empty());
        let (n, header) = lines
            .next()
            .unwrap_or_else(|| panic!("{} is empty", path.display()));
        assert_roundtrips::<CityJSON>(header, &format!("{}:{}", path.display(), n + 1));
        let mut features = 0;
        for (n, line) in lines {
            assert_roundtrips::<CityJSONFeature>(line, &format!("{}:{}", path.display(), n + 1));
            features += 1;
        }
        assert!(
            features > 0,
            "{} has no features; it would round-trip vacuously",
            path.display()
        );
    }
}

#[test]
fn every_cityjson_fixture_roundtrips_exactly() {
    for path in cj_fixtures() {
        assert_roundtrips::<CityJSON>(&read(&path), &path.display().to_string());
    }
}

/// The fixture lists above are hardcoded rather than globbed, deliberately: a
/// glob silently passes when it matches nothing, so it cannot fail the way a
/// missing fixture should. The cost of hardcoding is that a file can sit in
/// `tests/data/` unexercised and nothing says so -- which is exactly what
/// happened to `duplicate_keys.city.jsonl`, excluded on the strength of its
/// filename and never rechecked. This closes that gap: every file on disk is
/// either round-tripped or explicitly excluded with a reason.
#[test]
fn every_fixture_on_disk_is_accounted_for() {
    let listed: std::collections::HashSet<PathBuf> = seq_fixtures()
        .into_iter()
        .chain(cj_fixtures())
        .chain(EXCLUDED.iter().map(|(p, _)| PathBuf::from(*p)))
        .collect();

    let mut on_disk: Vec<PathBuf> = std::fs::read_dir("tests/data")
        .expect("tests/data must exist (run from the crate root)")
        .map(|e| e.expect("readable dir entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == "jsonl" || e == "json"))
        .collect();
    on_disk.sort();
    assert!(!on_disk.is_empty(), "tests/data is empty");

    let unaccounted: Vec<&PathBuf> = on_disk.iter().filter(|p| !listed.contains(*p)).collect();
    assert!(
        unaccounted.is_empty(),
        "these fixtures are in tests/data/ but are neither round-tripped nor \
         listed in EXCLUDED with a reason: {unaccounted:?}"
    );

    //-- and the reverse: a listed path that no longer exists must not pass
    //-- silently either
    for p in seq_fixtures().iter().chain(cj_fixtures().iter()) {
        assert!(
            p.exists(),
            "{} is listed as a fixture but is missing",
            p.display()
        );
    }
}

/// The comparison forgives an integer literal coming back as a decimal,
/// because JSON has one number type and `metadata.schema.json` types
/// `geographicalExtent`'s items as `{"type": "number"}`. It must not forgive
/// a number whose *value* changed -- in particular an integer too large to
/// survive an `f64`, which is the one case where the difference between
/// "respelled" and "lost" actually matters.
#[test]
fn the_comparison_forgives_respelling_but_not_precision_loss() {
    let forgiven = [
        (serde_json::json!({"a": 0}), serde_json::json!({"a": 0.0})),
        (
            serde_json::json!({"a": 100}),
            serde_json::json!({"a": 100.0}),
        ),
        (serde_json::json!({"a": -5}), serde_json::json!({"a": -5.0})),
    ];
    for (int, float) in forgiven {
        assert!(
            first_difference(&int, &float, "").is_none(),
            "{int} vs {float}"
        );
        assert_eq!(canonical_numbers(&int), canonical_numbers(&float));
    }

    let caught = [
        //-- precision loss: 2^53 + 1 is not representable as an f64
        (
            serde_json::json!({"a": 9007199254740993i64}),
            serde_json::json!({"a": 9007199254740992.0f64}),
        ),
        //-- an ordinary wrong value
        (serde_json::json!({"a": 1}), serde_json::json!({"a": 2.0})),
        (serde_json::json!({"a": 1.5}), serde_json::json!({"a": 1.0})),
    ];
    for (before, after) in caught {
        assert!(
            first_difference(&before, &after, "").is_some(),
            "{before} -> {after} is data loss and must be caught"
        );
        assert_ne!(canonical_numbers(&before), canonical_numbers(&after));
    }
}

/// A round trip through `CityJSON::from_str` (which additionally populates
/// the crate's internal `sorted_ids` bookkeeping) must be no lossier than a
/// bare `serde_json::from_str`.
#[test]
fn cityjson_from_str_roundtrips_exactly() {
    for path in cj_fixtures() {
        let contents = read(&path);
        let original: Value = serde_json::from_str(&contents).unwrap();
        let cj = CityJSON::from_str(&contents).unwrap();
        let reserialized = serde_json::to_value(&cj).unwrap();
        assert_equivalent(&original, &reserialized, &path.display().to_string());
    }
}
