use serde_json::{json, Value};
use std::fmt;
use std::fs::File;
use std::io::BufRead;
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
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Material {
    #[serde(skip_serializing_if = "Option::is_none")]
    values: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Geometry {
    #[serde(rename = "type")]
    thetype: String,
    lod: String,
    boundaries: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    semantics: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    material: Option<HashMap<String, Material>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    texture: Option<Value>,
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
    let stdin = std::io::stdin();

    let mut j: Value = json!(null);
    let mut allvertices: Vec<Vec<i32>> = Vec::new();
    for (i, line) in stdin.lock().lines().enumerate() {
        let l = line.unwrap();
        if i == 0 {
            j = serde_json::from_str(&l)?;
        } else {
            let mut cjf = serde_json::from_str(&l)?;
            collect_add_one_cjf(&mut j, &mut cjf, &mut allvertices);
        }
    }
    j["vertices"] = serde_json::to_value(&allvertices).unwrap();
    io::stdout().write_all(&format!("{}\n", serde_json::to_string(&j).unwrap()).as_bytes())?;
    Ok(())
}

fn collect_from_file(file: &PathBuf) -> Result<(), MyError> {
    let f = File::open(file.canonicalize()?)?;
    let br = BufReader::new(f);
    let mut j: Value = json!(null);
    let mut allvertices: Vec<Vec<i32>> = Vec::new();
    for (i, line) in br.lines().enumerate() {
        match line {
            Ok(content) => {
                // println!("{}: {:?}", i, content);
                if i == 0 {
                    j = serde_json::from_str(&content)?;
                } else {
                    let mut cjf = serde_json::from_str(&content)?;
                    collect_add_one_cjf(&mut j, &mut cjf, &mut allvertices);
                }
            }
            Err(error) => eprintln!("Error reading line: {}", error),
        }
    }
    j["vertices"] = serde_json::to_value(&allvertices).unwrap();
    io::stdout().write_all(&format!("{}\n", serde_json::to_string(&j).unwrap()).as_bytes())?;
    // println!("{}", serde_json::to_string_pretty(&j)?);
    Ok(())
}

fn collect_add_one_cjf(j: &mut Value, cjf: &mut Value, allvertices: &mut Vec<Vec<i32>>) {
    let offset = allvertices.len();
    for (_key, co) in cjf["CityObjects"].as_object_mut().unwrap() {
        let x = co["geometry"].as_array_mut();
        if x.is_some() {
            for g in x.unwrap() {
                // TODO : add other Geometric primitives
                if g["type"] == "MultiSurface" || g["type"] == "CompositeSurface" {
                    for surface in g["boundaries"].as_array_mut().unwrap() {
                        for ring in surface.as_array_mut().unwrap() {
                            for p in ring.as_array_mut().unwrap() {
                                let p1: i64 = p.as_i64().unwrap();
                                *p = Value::Number((p1 + offset as i64).into());
                            }
                        }
                    }
                } else if g["type"] == "Solid" {
                    for shell in g["boundaries"].as_array_mut().unwrap() {
                        for surface in shell.as_array_mut().unwrap() {
                            for ring in surface.as_array_mut().unwrap() {
                                for p in ring.as_array_mut().unwrap() {
                                    let p1: i64 = p.as_i64().unwrap();
                                    *p = Value::Number((p1 + offset as i64).into());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    //-- copy the offsetted CO
    for (key, value) in cjf["CityObjects"].as_object_mut().unwrap() {
        j["CityObjects"][key] = value.clone();
    }
    //-- add the new vertices
    let mut vertices: Vec<Vec<i32>> = serde_json::from_value(cjf["vertices"].take()).unwrap();
    allvertices.append(&mut vertices);
}

fn cat_from_stdin() -> Result<(), MyError> {
    let mut input = String::new();
    match std::io::stdin().read_to_string(&mut input) {
        Ok(_) => {
            let mut j: Value = serde_json::from_str(&input)?;
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
    let g_old_vertices: Vec<Vec<i32>> = serde_json::from_value(j["vertices"].take()).unwrap();
    for (key, co) in &cos {
        if co.is_toplevel() {
            let mut cjf = json!({
                    "type": "CityJSONFeature",
                    "id": key,
                    "CityObjects": json!({}),
                    "vertices": json!([])
            });
            let co2: &mut CityObject = &mut co.clone();

            let mut g_vi_oldnew: HashMap<usize, usize> = HashMap::new();
            let mut g_new_vi: Vec<usize> = Vec::new();
            let mut m_oldnew: HashMap<usize, usize> = HashMap::new();
            match &mut co2.geometry {
                Some(x) => {
                    for mut g in x.iter_mut() {
                        //-- geometry/boundaries
                        update_geometry_vi(&mut g, &mut g_vi_oldnew, &mut g_new_vi);
                        //-- geometry/material
                        update_material(&mut g, &mut m_oldnew);
                        // println!("== {:?}", m_oldnew);
                    }
                }
                None => (),
            }
            cjf["CityObjects"][key] = serde_json::to_value(&co2).unwrap();

            //-- process all the children (only one-level lower)
            //-- TODO: to fix: children-of-children?
            for childkey in co.get_children_keys() {
                let coc = cos.get(&childkey).unwrap();
                let coc2: &mut CityObject = &mut coc.clone();
                match &mut coc2.geometry {
                    Some(x) => {
                        for mut g in x.iter_mut() {
                            update_geometry_vi(&mut g, &mut g_vi_oldnew, &mut g_new_vi);
                        }
                    }
                    None => (),
                }
                cjf["CityObjects"][childkey] = serde_json::to_value(&coc2).unwrap();
            }

            //-- "slice" geometry vertices
            let mut g_new_vertices: Vec<Vec<i32>> = Vec::new();
            for v in &g_new_vi {
                g_new_vertices.push(g_old_vertices[*v].clone());
            }
            cjf["vertices"] = serde_json::to_value(&g_new_vertices).unwrap();

            //-- "slice" material vertices
            let mats: Vec<Value> =
                serde_json::from_value(j["appearance"]["materials"].take()).unwrap();
            let mut mats2: Vec<Value> = Vec::new();
            mats2.resize(m_oldnew.len(), json!(null));
            for (old, new) in &m_oldnew {
                mats2[*new] = mats[*old].clone();
            }
            cjf["appearance"]["materials"] = serde_json::to_value(&mats2).unwrap();

            //-- write to stdout
            io::stdout()
                .write_all(&format!("{}\n", serde_json::to_string(&cjf).unwrap()).as_bytes())?;
        }
    }
    Ok(())
}

fn update_material(g: &mut Geometry, m_oldnew: &mut HashMap<usize, usize>) {
    match &mut g.material {
        Some(x) => {
            for (_key, mat) in &mut *x {
                //-- material.value
                if mat.value.is_some() {
                    let thevalue: usize = mat.value.unwrap();
                    let r = m_oldnew.get(&thevalue);
                    if r.is_none() {
                        let l = m_oldnew.len();
                        m_oldnew.insert(thevalue, l);
                        mat.value = Some(l);
                    } else {
                        let r2 = r.unwrap();
                        mat.value = Some(*r2);
                    }
                    continue;
                }
                //-- else it's material.values (which differs per geom type)
                if g.thetype == "MultiSurface" || g.thetype == "CompositeSurface" {
                    if mat.values.is_some() {
                        let a: Vec<Option<usize>> =
                            serde_json::from_value(mat.values.take().into()).unwrap();
                        let mut a2 = a.clone();
                        for (i, x) in a.iter().enumerate() {
                            if x.is_some() {
                                let y2 = m_oldnew.get(&x.unwrap());
                                if y2.is_none() {
                                    let l = m_oldnew.len();
                                    m_oldnew.insert(x.unwrap(), l);
                                    a2[i] = Some(l);
                                } else {
                                    let y2 = y2.unwrap();
                                    a2[i] = Some(*y2);
                                }
                            }
                        }
                        mat.values = Some(serde_json::to_value(&a2).unwrap());
                    }
                } else if g.thetype == "Solid" {
                    if mat.values.is_some() {
                        let a: Vec<Vec<Option<usize>>> =
                            serde_json::from_value(mat.values.take().into()).unwrap();
                        let mut a2 = a.clone();
                        for (i, x) in a.iter().enumerate() {
                            for (j, y) in x.iter().enumerate() {
                                if y.is_some() {
                                    let y2 = m_oldnew.get(&y.unwrap());
                                    if y2.is_none() {
                                        let l = m_oldnew.len();
                                        m_oldnew.insert(y.unwrap(), l);
                                        a2[i][j] = Some(l);
                                    } else {
                                        let y2 = y2.unwrap();
                                        a2[i][j] = Some(*y2);
                                    }
                                }
                            }
                        }
                        mat.values = Some(serde_json::to_value(&a2).unwrap());
                    }
                } else if g.thetype == "MultiSolid" || g.thetype == "CompositeSolid" {
                    if mat.values.is_some() {
                        let a: Vec<Vec<Vec<Option<usize>>>> =
                            serde_json::from_value(mat.values.take().into()).unwrap();
                        let mut a2 = a.clone();
                        for (i, x) in a.iter().enumerate() {
                            for (j, y) in x.iter().enumerate() {
                                for (k, z) in y.iter().enumerate() {
                                    if z.is_some() {
                                        let y2 = m_oldnew.get(&z.unwrap());
                                        if y2.is_none() {
                                            let l = m_oldnew.len();
                                            m_oldnew.insert(z.unwrap(), l);
                                            a2[i][j][k] = Some(l);
                                        } else {
                                            let y2 = y2.unwrap();
                                            a2[i][j][k] = Some(*y2);
                                        }
                                    }
                                }
                            }
                        }
                        mat.values = Some(serde_json::to_value(&a2).unwrap());
                    }
                }
            }
            g.material = Some(x.clone());
        }
        None => (),
    }
}

fn update_geometry_vi(
    g: &mut Geometry,
    violdnew: &mut HashMap<usize, usize>,
    newvi: &mut Vec<usize>,
) {
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
