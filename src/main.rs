use serde_json::{json, Result, Value};
use std::fs::File;
use std::io::BufReader;
use std::io::{self, Write};
use std::path::PathBuf;

use serde_derive::{Deserialize, Serialize};

use std::collections::HashMap;

use clap::{arg, command, value_parser, Command};

#[derive(Serialize, Deserialize, Debug)]
struct Vertex {
    x: u32,
    y: u32,
    z: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct CityObject {
    #[serde(rename = "type")]
    thetype: String,
    #[serde(rename = "geographicalExtent")]
    #[serde(skip_serializing_if = "Option::is_none")]
    geographical_extent: Option<Vec<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attributes: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    geometry: Option<Vec<Geometry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parents: Option<Vec<String>>,
}

impl CityObject {
    fn is_toplevel(&self) -> bool {
        match &self.parents {
            Some(x) => {
                if x.is_empty() {
                    return true;
                } else {
                    return false;
                }
            }
            None => return true,
        }
    }
    fn get_children_keys(&self) -> Vec<String> {
        let mut re: Vec<String> = Vec::new();
        match &self.children {
            Some(x) => {
                for each in x {
                    re.push(each.to_string());
                }
            }
            None => (),
        }
        re
    }
    fn get_geometries(&self) -> Vec<Geometry> {
        let mut re: Vec<Geometry> = Vec::new();
        match &self.geometry {
            Some(x) => {
                for each in x {
                    re.push(each.clone());
                }
            }
            None => (),
        }
        re
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Geometry {
    #[serde(rename = "type")]
    thetype: String,
    lod: String,
    boundaries: Value,
    semantics: Option<Value>,
}

impl Geometry {
    fn get_lod(&self) -> String {
        format!("{:.1}", self.lod)
    }
}

fn main() {
    let matches = command!() // requires `cargo` feature
        .propagate_version(true)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("cat").about("CityJSON=>CityJSONSeq").arg(
                arg!(
                    -f --file <FILE> "Read from file instead of stdin"
                )
                .required(false)
                .value_parser(value_parser!(PathBuf)),
            ),
        )
        .subcommand(
            Command::new("collect").about("CityJSONSeq=>CityJSON").arg(
                arg!(
                    -f --file <FILE> "Read from file instead of stdin"
                )
                .required(false)
                .value_parser(value_parser!(PathBuf)),
            ),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("cat", sub_matches)) => {
            // println!("'cat' was used");
            let _re = cat_from_file(sub_matches.get_one::<PathBuf>("file"));
            // println!("{:?}", re);
        }
        Some(("collect", sub_matches)) => println!(
            "'collect' was used, name is: {:?}",
            sub_matches
                .get_one::<PathBuf>("file")
                .expect("Wrong path")
                .canonicalize()
                .unwrap()
        ),
        _ => (),
    }
}

fn cat_from_file(file: Option<&PathBuf>) -> io::Result<()> {
    let br = BufReader::new(File::open(file.unwrap().canonicalize().unwrap()).unwrap());
    let mut j: Value = serde_json::from_reader(br)?;

    if j["type"] != "CityJSON" {
        eprintln!("Input file not CityJSON v1.1 nor v2.0.");
        std::process::exit(1);
    }
    if j["version"] != "1.1" && j["version"] != "2.0" {
        eprintln!("Input file not CityJSON v1.1 nor v2.0.");
        std::process::exit(1);
    }

    //-- 1st line: CityJSON metadata
    let mut cj1 = json!({
        "type": "CityJSON",
        "version": j["version"],
        "transform": j["transform"],
        "CityObjects": json!({}),
        "vertices": json!([])
    });
    if j["metadata"] != json!(null) {
        cj1["metadata"] = j["metadata"].clone();
    }
    if j["geometry-templates"] != json!(null) {
        cj1["geometry-templates"] = j["geometry-templates"].clone();
    }
    if j["extensions"] != json!(null) {
        cj1["extensions"] = j["extensions"].clone();
    }
    io::stdout().write_all(&format!("{}\n", serde_json::to_string(&cj1).unwrap()).as_bytes())?;

    let cos: HashMap<String, CityObject> = serde_json::from_value(j["CityObjects"].take()).unwrap();
    let oldvertices: Vec<Vec<u32>> = serde_json::from_value(j["vertices"].take()).unwrap();
    for (key, co) in &cos {
        if co.is_toplevel() {
            let mut cjf = json!({
                    "type": "CityJSONFeature",
                    "id": key,
                    "CityObjects": json!({}),
                    "vertices": json!([])
            });
            let co2: &mut CityObject = &mut co.clone();
            let mut newvi: Vec<usize> = Vec::new();
            let mut violdnew: HashMap<usize, usize> = HashMap::new();
            match &co.geometry {
                Some(x) => {
                    let mut x2: Vec<Geometry> = x.clone();
                    for g in x2.iter_mut() {
                        update_vi(g, &mut violdnew, &mut newvi);
                    }
                    co2.geometry = Some(x2);
                }
                None => (),
            }
            cjf["CityObjects"][key] = serde_json::to_value(&co2).unwrap();

            //-- process all the children (only one-level lower)
            //-- TODO: to fix: children-of-children
            for childkey in co.get_children_keys() {
                // cjf["CityObjects"][childkey] = serde_json::to_value(&cos.get(&childkey)).unwrap();
                let coc = cos.get(&childkey).unwrap();
                let coc2: &mut CityObject = &mut coc.clone();
                match &coc.geometry {
                    Some(x) => {
                        let mut x2: Vec<Geometry> = x.clone();
                        for g in x2.iter_mut() {
                            update_vi(g, &mut violdnew, &mut newvi);
                        }
                        coc2.geometry = Some(x2);
                    }
                    None => (),
                }
                cjf["CityObjects"][childkey] = serde_json::to_value(&coc2).unwrap();
            }

            //-- "slice" vertices
            let mut newvertices: Vec<Vec<u32>> = Vec::new();
            for v in &newvi {
                newvertices.push(oldvertices[*v].clone());
            }
            cjf["vertices"] = serde_json::to_value(&newvertices).unwrap();
            io::stdout()
                .write_all(&format!("{}\n", serde_json::to_string(&cjf).unwrap()).as_bytes())?;
            // println!("{}", serde_json::to_string(&cjf).unwrap());
            // println!("{}", serde_json::to_string_pretty(&cjf).unwrap());
            // break;
        }
    }
    Ok(())
}

fn update_vi(g: &mut Geometry, violdnew: &mut HashMap<usize, usize>, newvi: &mut Vec<usize>) {
    // println!("{:?}", g.thetype);
    if g.thetype == "MultiPoint" {
        let a: Vec<usize> = serde_json::from_value(g.boundaries.clone()).unwrap();
    } else if g.thetype == "MultiLineString" {
        let a: Vec<Vec<usize>> = serde_json::from_value(g.boundaries.clone()).unwrap();
    } else if g.thetype == "MultiSurface" || g.thetype == "CompositeSurface" {
        let a: Vec<Vec<Vec<usize>>> = serde_json::from_value(g.boundaries.clone()).unwrap();
        let mut a2 = a.clone();
        for (i, x) in a.iter().enumerate() {
            for (j, y) in x.iter().enumerate() {
                for (k, z) in y.iter().enumerate() {
                    // r.push(z);
                    let kk = violdnew.get(&z);
                    if kk.is_none() {
                        let l = newvi.len();
                        violdnew.insert(*z, l);
                        newvi.push(*z);
                        a2[i][j][k] = l;
                    } else {
                        let kk = kk.unwrap();
                        a2[i][j][k] = *kk;
                    }
                }
            }
        }
        g.boundaries = serde_json::to_value(&a2).unwrap();
    } else if g.thetype == "Solid" {
        let a: Vec<Vec<Vec<Vec<usize>>>> = serde_json::from_value(g.boundaries.clone()).unwrap();
        let mut a2 = a.clone();
        for (i, x) in a.iter().enumerate() {
            for (j, y) in x.iter().enumerate() {
                for (k, z) in y.iter().enumerate() {
                    for (l, zz) in z.iter().enumerate() {
                        // r.push(z);
                        let kk = violdnew.get(&zz);
                        if kk.is_none() {
                            let length = newvi.len();
                            violdnew.insert(*zz, length);
                            newvi.push(*zz);
                            a2[i][j][k][l] = length;
                        } else {
                            let kk = kk.unwrap();
                            a2[i][j][k][l] = *kk;
                        }
                    }
                }
            }
        }
        g.boundaries = serde_json::to_value(&a2).unwrap();
    }
}
