use crate::cjseq::{CityJSON, CityJSONFeature};
use cjseq;
use std::path::PathBuf;

use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;

#[test]
fn test1() {
    let mut cjj = CityJSON::new();
    let path = PathBuf::from("data/3dbag_b2.city.jsonl");
    let f = File::open(path.canonicalize().unwrap()).unwrap();
    let br = BufReader::new(f);
    for (i, line) in br.lines().enumerate() {
        match &line {
            Ok(l) => {
                if i == 0 {
                    cjj = CityJSON::from_str(&l).unwrap();
                    assert!(cjj.number_of_city_objects() == 0);
                    assert!(cjj.get_cjfeature(0).is_none());
                    assert_eq!(cjj.vertices.is_empty(), true);
                } else {
                    let mut cjf: CityJSONFeature = CityJSONFeature::from_str(&l).unwrap();
                    cjj.add_cjfeature(&mut cjf);
                    assert_eq!(cjj.number_of_city_objects(), i);
                }
            }
            Err(error) => eprintln!("Error reading line: {}", error),
        }
    }
    assert_eq!(cjj.number_of_city_objects(), 2);
}
