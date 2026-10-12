use cjseq::{CityJSON, CityJSONFeature, Collector, SortingStrategy};
use std::str::FromStr;

#[test]
fn test1() {
    let text = std::fs::read_to_string("data/3dbag_b2.city.jsonl").unwrap();
    let mut lines = text.lines();
    let header = CityJSON::from_str(lines.next().unwrap()).unwrap();
    assert_eq!(header.number_of_city_objects(), 0);
    assert_eq!(header.features(SortingStrategy::AsIs).count(), 0);
    assert!(header.vertices.is_empty());
    let mut c = Collector::new(header);
    let mut n = 0;
    for l in lines {
        c.add(CityJSONFeature::from_str(l).unwrap()).unwrap();
        n += 1;
    }
    let cj = c.finish();
    assert_eq!(n, 2);
    assert_eq!(cj.number_of_city_objects(), 2);
}
