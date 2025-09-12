use crate::cjseq::CityJSON;
use cjseq;

use serde_json::Value;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;

fn get_empty_file() -> Value {
    let j_1 = r#"
        {
          "type": "CityJSON",
          "version": "2.0",
          "CityObjects": {},
          "vertices": [],
          "transform": {
            "scale": [
              0.001,
              0.001,
              0.001
            ],
            "translate": [
              -1.0,
              -1.0,
              0.0
            ]
          },
          "metadata": {
            "geographicalExtent": [
              -1.0,
              -1.0,
              0.0,
              1.0,
              1.0,
              1.0
            ]
          }
        }
        "#;
    let v = serde_json::from_str(&j_1).unwrap();
    v
}

#[test]
fn empty() {
    let j = get_empty_file();
    let cjj = CityJSON::from_str(&j.to_string()).unwrap();
    assert!(cjj.number_of_city_objects() == 0);
    assert!(cjj.get_cjfeature(0).is_none());
}

#[test]
fn basic_reading_of_cj() {
    let f = File::open("data/3dbag_b2.city.json").unwrap();
    let mut br = BufReader::new(f);
    let mut json_content = String::new();
    br.read_to_string(&mut json_content).unwrap();
    let mut cjj: CityJSON = CityJSON::from_str(&json_content).unwrap();
    assert!(cjj.number_of_city_objects() == 2);

    let cj1 = cjj.get_metadata();
    assert!(cj1.number_of_city_objects() == 0);
    assert!(cj1.vertices.is_empty());

    cjj.sort_cjfeatures(cjseq::SortingStrategy::Lexicographical);
    let mut cjnext = cjj.get_cjfeature(0).unwrap();
    assert_eq!(cjnext.thetype, "CityJSONFeature");
    assert_eq!(cjnext.id, "NL.IMBAG.Pand.0503100000028341");

    cjnext = cjj.get_cjfeature(1).unwrap();
    assert_eq!(cjnext.id, "NL.IMBAG.Pand.0503100000031927");
}
