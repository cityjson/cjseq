use serde_json::{json, Value};
use std::fmt;
use std::fs::File;
use std::io::BufReader;
use std::io::{self, Read, Write};
use std::path::PathBuf;

use serde_derive::{Deserialize, Serialize};

use std::collections::HashMap;

use clap::{arg, command, value_parser, Command};

#[derive(Debug)]
enum MyError {
    IoError(std::io::Error),
    JsonError(serde_json::Error),
    CityJsonError(String),
}
impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MyError::JsonError(json_error) => write!(f, "Error (JSON): {}", json_error),
            MyError::IoError(io_error) => write!(f, "Error (io): {}", io_error),
            MyError::CityJsonError(cjson_error) => write!(f, "Error (CityJSON): {}", cjson_error),
        }
    }
}
impl std::error::Error for MyError {}
impl From<serde_json::Error> for MyError {
    fn from(err: serde_json::Error) -> Self {
        MyError::JsonError(err)
    }
}
impl From<std::io::Error> for MyError {
    fn from(err: std::io::Error) -> Self {
        MyError::IoError(err)
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Vertex {
    x: i32,
    y: i32,
    z: i32,
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
    let matches = command!()
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
        Some(("cat", sub_matches)) => match sub_matches.get_one::<PathBuf>("file") {
            Some(x) => {
                if let Err(e) = cat_from_file(x) {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
            None => {
                if let Err(e) = cat_from_stdin() {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        },
        Some(("collect", sub_matches)) => match sub_matches.get_one::<PathBuf>("file") {
            Some(x) => {
                if let Err(e) = collect_from_file(x) {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
            None => {
                if let Err(e) = collect_from_stdin() {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        },
        _ => (),
    }
}

fn collect_from_stdin() -> Result<(), MyError> {
    Ok(())
}

fn collect_from_file(file: &PathBuf) -> Result<(), MyError> {
    Ok(())
}

fn cat_from_stdin() -> Result<(), MyError> {
    let mut input = String::new();
    match std::io::stdin().read_to_string(&mut input) {
        Ok(_) => {
            let mut j: Value = serde_json::from_str(&input)?;
            // println!("{:?}", serde_json::to_string(&j).unwrap());
            let _ = cat(&mut j);
        }
        Err(error) => {
            println!("Error: {}", error);
        }
    }
    Ok(())
}

fn cat_from_file(file: &PathBuf) -> Result<(), MyError> {
    let f = File::open(file.canonicalize()?)?;
    let br = BufReader::new(f);
    let mut j: Value = serde_json::from_reader(br)?;
    cat(&mut j)?;
    Ok(())
}

fn cat(j: &mut Value) -> Result<(), MyError> {
    if j["type"] != "CityJSON" {
        return Err(MyError::CityJsonError(
            "Input file not CityJSON.".to_string(),
        ));
    }
    if j["version"] != "1.1" && j["version"] != "2.0" {
        return Err(MyError::CityJsonError(
            "Input file not CityJSON v1.1 nor v2.0.".to_string(),
        ));
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
    let oldvertices: Vec<Vec<i32>> = serde_json::from_value(j["vertices"].take()).unwrap();
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

            match &mut co2.geometry {
                Some(x) => {
                    for mut g in x.iter_mut() {
                        update_vi(&mut g, &mut violdnew, &mut newvi);
                    }
                }
                None => (),
            }
            cjf["CityObjects"][key] = serde_json::to_value(&co2).unwrap();

            //-- process all the children (only one-level lower)
            //-- TODO: to fix: children-of-children
            for childkey in co.get_children_keys() {
                let coc = cos.get(&childkey).unwrap();
                let coc2: &mut CityObject = &mut coc.clone();
                match &mut coc2.geometry {
                    Some(x) => {
                        for mut g in x.iter_mut() {
                            update_vi(&mut g, &mut violdnew, &mut newvi);
                        }
                    }
                    None => (),
                }
                cjf["CityObjects"][childkey] = serde_json::to_value(&coc2).unwrap();
            }

            //-- "slice" vertices
            let mut newvertices: Vec<Vec<i32>> = Vec::new();
            for v in &newvi {
                newvertices.push(oldvertices[*v].clone());
            }
            cjf["vertices"] = serde_json::to_value(&newvertices).unwrap();
            io::stdout()
                .write_all(&format!("{}\n", serde_json::to_string(&cjf).unwrap()).as_bytes())?;
        }
    }
    Ok(())
}

fn update_vi(g: &mut Geometry, violdnew: &mut HashMap<usize, usize>, newvi: &mut Vec<usize>) {
    // println!("{:?}", g.thetype);
    if g.thetype == "MultiPoint" {
        //TODO: MultiPoint
        let a: Vec<usize> = serde_json::from_value(g.boundaries.clone()).unwrap();
    } else if g.thetype == "MultiLineString" {
        //TODO: MultiPoint
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
    //TODO: CompositeSurface
    //TODO: MultiSolid
    //TODO: CompositeSolid
}
