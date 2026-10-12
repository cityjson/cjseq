//! The `cjseq` binary, run as a user would.

use std::io::Write;
use std::process::{Command, Output, Stdio};

fn cjseq(args: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_cjseq"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

#[test]
fn validate_prints_warnings_without_failing() {
    let out = cjseq(&["validate", "tests/data/noise_extension.city.jsonl"], "");
    let stdout = text(&out.stdout);
    assert!(out.status.success(), "{stdout}{}", text(&out.stderr));
    assert_eq!(
        stdout.matches("warning:").count(),
        2,
        "one warning per +NoiseCityFurnitureSegment under a CityFurniture:\n{stdout}"
    );
    assert!(stdout.contains("a_noisy_bench"), "{stdout}");
    assert!(stdout.trim_end().ends_with("valid"), "{stdout}");
}

#[test]
fn validate_still_fails_on_an_error() {
    let doc = r#"{"type":"CityJSON","version":"2.0","transform":{"scale":[1.0,1.0,1.0],"translate":[0.0,0.0,0.0]},"CityObjects":{"b":{"type":"Building","children":["x"]}},"vertices":[]}"#;
    let out = cjseq(&["validate"], doc);
    assert!(!out.status.success());
    assert!(text(&out.stdout).contains("\"x\""), "{}", text(&out.stdout));
}

#[test]
fn collect_warns_about_each_feature_member_it_drops() {
    let seq = concat!(
        r#"{"type":"CityJSON","version":"2.0","transform":{"scale":[1.0,1.0,1.0],"translate":[0.0,0.0,0.0]},"CityObjects":{},"vertices":[]}"#,
        "\n",
        r#"{"type":"CityJSONFeature","id":"a","CityObjects":{"a":{"type":"Building"}},"vertices":[],"+provenance":{"run":7}}"#,
        "\n",
        r#"{"type":"CityJSONFeature","id":"b","CityObjects":{"b":{"type":"Building"}},"vertices":[]}"#,
        "\n"
    );
    let out = cjseq(&["collect"], seq);
    assert!(out.status.success(), "{}", text(&out.stderr));
    let stderr = text(&out.stderr);
    assert_eq!(stderr.matches("warning:").count(), 1, "{stderr}");
    assert!(
        stderr.contains("+provenance") && stderr.contains("\"a\""),
        "{stderr}"
    );
    //-- the output itself is unchanged: the member is not in it
    assert!(!text(&out.stdout).contains("+provenance"));
}
