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

#[derive(Serialize, Deserialize, Debug, Clone)]
struct CityJSON {
    #[serde(rename = "type")]
    thetype: String,
    version: String,
    transform: Value,
    #[serde(rename = "CityObjects")]
    city_objects: HashMap<String, CityObject>,
    vertices: Vec<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    appearance: Option<Appearance>,
    #[serde(rename = "geometry-templates")]
    #[serde(skip_serializing_if = "Option::is_none")]
    geometry_templates: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    extensions: Option<Value>,
    #[serde(flatten)]
    other: serde_json::Value,
}
impl CityJSON {
    fn get_empty_copy(&self) -> Self {
        let co: HashMap<String, CityObject> = HashMap::new();
        let v: Vec<Vec<i64>> = Vec::new();
        CityJSON {
            thetype: self.thetype.clone(),
            version: self.version.clone(),
            transform: self.transform.clone(),
            metadata: self.metadata.clone(),
            city_objects: co,
            vertices: v,
            appearance: None,
            geometry_templates: self.geometry_templates.clone(),
            other: self.other.clone(),
            extensions: self.extensions.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct CityJSONFeature {
    #[serde(rename = "type")]
    thetype: String,
    id: String,
    #[serde(rename = "CityObjects")]
    city_objects: HashMap<String, CityObject>,
    vertices: Vec<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    appearance: Option<Appearance>,
}
impl CityJSONFeature {
    fn new() -> Self {
        let co: HashMap<String, CityObject> = HashMap::new();
        let v: Vec<Vec<i64>> = Vec::new();
        CityJSONFeature {
            thetype: "CityJSONFeature".to_string(),
            id: "".to_string(),
            city_objects: co,
            vertices: v,
            appearance: None,
        }
    }
    fn add_co(&mut self, id: String, co: CityObject) {
        self.city_objects.insert(id, co);
    }
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
    #[serde(flatten)]
    other: serde_json::Value,
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
    texture: Option<HashMap<String, Texture>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Vertex {
    x: i64,
    y: i64,
    z: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Material {
    #[serde(skip_serializing_if = "Option::is_none")]
    values: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Texture {
    #[serde(skip_serializing_if = "Option::is_none")]
    values: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Appearance {
    #[serde(skip_serializing_if = "Option::is_none")]
    materials: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    textures: Option<Vec<Value>>,
    #[serde(rename = "vertices-texture")]
    #[serde(skip_serializing_if = "Option::is_none")]
    vertices_texture: Option<Vec<Vec<f64>>>,
    #[serde(rename = "default-theme-texture")]
    #[serde(skip_serializing_if = "Option::is_none")]
    default_theme_texture: Option<String>,
    #[serde(rename = "default-theme-material")]
    #[serde(skip_serializing_if = "Option::is_none")]
    default_theme_material: Option<String>,
}
impl Appearance {
    fn new() -> Self {
        Appearance {
            materials: None,
            textures: None,
            vertices_texture: None,
            default_theme_texture: None,
            default_theme_material: None,
        }
    }
    fn add_material(&mut self, jm: Value) -> usize {
        let re = match &mut self.materials {
            Some(x) => match x.iter().position(|e| *e == jm) {
                Some(y) => y,
                None => {
                    x.push(jm);
                    x.len()
                }
            },
            None => {
                let mut ls: Vec<Value> = Vec::new();
                ls.push(jm);
                self.materials = Some(ls);
                0
            }
        };
        re
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
    let stdin = std::io::stdin();

    let mut j: Value = json!(null);
    let mut allvertices: Vec<Vec<i64>> = Vec::new();
    for (i, line) in stdin.lock().lines().enumerate() {
        let l = line.unwrap();
        if i == 0 {
            j = serde_json::from_str(&l)?;
        } else {
            let mut cjf = serde_json::from_str(&l)?;
            // collect_add_one_cjf(&mut j, &mut cjf, &mut allvertices); //-- TODO: uncomment this
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
    let mut all_g_v: Vec<Vec<i64>> = Vec::new();
    let mut all_app: Appearance = Appearance::new();
    for (i, line) in br.lines().enumerate() {
        match &line {
            Ok(content) => {
                if i == 0 {
                    j = serde_json::from_str(&content)?;
                } else {
                    let mut cjf = serde_json::from_str(&content)?;
                    collect_add_one_cjf(&mut j, &mut cjf, &mut all_g_v, &mut all_app);
                }
            }
            Err(error) => eprintln!("Error reading line: {}", error),
        }
    }
    j["vertices"] = serde_json::to_value(&all_g_v).unwrap();
    io::stdout().write_all(&format!("{}\n", serde_json::to_string(&j).unwrap()).as_bytes())?;
    Ok(())
}

fn collect_add_one_cjf(
    j: &mut Value,
    cjf: &mut Value,
    all_g_v: &mut Vec<Vec<i64>>,
    all_app: &mut Appearance,
) {
    let offset = all_g_v.len();
    for (key, co) in cjf["CityObjects"].as_object_mut().unwrap() {
        //-- geometry
        let geoms = co["geometry"].as_array_mut();
        if geoms.is_some() {
            collect_update_cjf_geometry_offset(&mut geoms.unwrap(), offset);
        }
        //-- appearance
        // match co.pointer_mut("/appearance") {
        //     Some(x) => {
        //         let cjf_app: Appearance = serde_json::from_value(x.take()).unwrap();
        //         match &cjf_app.materials {
        //             Some(x) => {
        //                 let mut m_oldnew: HashMap<usize, usize> = HashMap::new();
        //                 for (i, m) in x.iter().enumerate() {
        //                     m_oldnew.insert(i, all_app.add_material(m.clone()));
        //                 }
        //                 //-- update the material indices
        //                 // for g in &mut geoms {
        //                 //     cat_update_material(&mut g, &mut m_oldnew);
        //                 // }
        //             }
        //             None => (),
        //         }
        //     }
        //     None => (),
        // }

        //-- update the collected json object by adding the CityObjects
        j["CityObjects"][key] = co.clone();
    }
    //-- add the new vertices
    let mut vertices: Vec<Vec<i64>> = serde_json::from_value(cjf["vertices"].take()).unwrap();
    all_g_v.append(&mut vertices);
}

// fn collect_update_cjf_geometry_material(geoms: &mut Vec<Value>, moldnew: &HashMap<usize, usize) {
//     for g in geoms {
//         // TODO : add other Geometric primitives
//         if g["type"] == "MultiSurface" || g["type"] == "CompositeSurface" {
//             let a: Vec<Option<usize>> =
//                             serde_json::from_value(g["material"].values.take().into()).unwrap();
//             let mut a2 = a.clone();
//             for (i, x) in a.iter().enumerate() {
//                 if x.is_some() {
//                     let y2 = m_oldnew.get(&x.unwrap());
//                     if y2.is_none() {
//                         let l = m_oldnew.len();
//                         m_oldnew.insert(x.unwrap(), l);
//                         a2[i] = Some(l);
//                     } else {
//                         let y2 = y2.unwrap();
//                         a2[i] = Some(*y2);
//                     }
//                 }
//             }
//             mat.values = Some(serde_json::to_value(&a2).unwrap());
//         } else if g["type"] == "Solid" {
//             for shell in g["boundaries"].as_array_mut().unwrap() {
//                 for surface in shell.as_array_mut().unwrap() {
//                     for ring in surface.as_array_mut().unwrap() {
//                         for p in ring.as_array_mut().unwrap() {
//                             let p1: i64 = p.as_i64().unwrap();
//                             *p = Value::Number((p1 + offset as i64).into());
//                         }
//                     }
//                 }
//             }
//         }
//     }
// }

fn collect_update_cjf_geometry_offset(geoms: &mut Vec<Value>, offset: usize) {
    for g in geoms {
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

fn collect_add_one_cjf_geometry(j: &mut Value, cjf: &mut Value, all_g_v: &mut Vec<Vec<i64>>) {
    let offset = all_g_v.len();
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
    let mut vertices: Vec<Vec<i64>> = serde_json::from_value(cjf["vertices"].take()).unwrap();
    all_g_v.append(&mut vertices);
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

// fn test(file: &PathBuf) -> Result<(), MyError> {
//     let f = File::open(file.canonicalize()?)?;
//     let br = BufReader::new(f);
//     let mut cjj: CityJSON = serde_json::from_reader(br)?;
//     let vs = &mut cjj.vertices;
//     vs.clear();
//     vs.push(vec![0, 2, 99]);
//     println!("{}", serde_json::to_string(&cjj).unwrap());
//     Ok(())
// }

fn cat_from_file(file: &PathBuf) -> Result<(), MyError> {
    let f = File::open(file.canonicalize()?)?;
    let br = BufReader::new(f);

    let cjj: CityJSON = serde_json::from_reader(br)?;
    cat2(&cjj)?;
    //     let vs = &mut cjj.vertices;
    //     vs.clear();
    //     vs.push(vec![0, 2, 99]);
    // cat(&mut j)?;
    Ok(())
}

fn cat2(cjj: &CityJSON) -> Result<(), MyError> {
    if cjj.thetype != "CityJSON" {
        return Err(MyError::CityJsonError(
            "Input file not CityJSON.".to_string(),
        ));
    }
    if cjj.version != "1.1" && cjj.version != "2.0" {
        return Err(MyError::CityJsonError(
            "Input file not CityJSON v1.1 nor v2.0.".to_string(),
        ));
    }

    //-- first line: the CityJSON "metadata"
    let cj1: CityJSON = cjj.get_empty_copy();
    io::stdout().write_all(&format!("{}\n", serde_json::to_string(&cj1).unwrap()).as_bytes())?;

    let cos = &cjj.city_objects;
    for (key, co) in cos {
        if co.is_toplevel() {
            let mut cjf = CityJSONFeature::new();

            let mut co2: CityObject = co.clone();

            let mut g_vi_oldnew: HashMap<usize, usize> = HashMap::new();
            let mut g_new_vi: Vec<usize> = Vec::new();
            // let m_oldnew: HashMap<usize, usize> = HashMap::new();
            // let mut t_oldnew: HashMap<usize, usize> = HashMap::new();
            // let mut t_v_oldnew: HashMap<usize, usize> = HashMap::new();
            match &mut co2.geometry {
                Some(x) => {
                    for mut g in x.iter_mut() {
                        //-- geometry/boundaries
                        cat_update_geometry_vi(&mut g, &mut g_vi_oldnew, &mut g_new_vi);
                        //-- geometry/material
                        // cat_update_material(&mut g, &mut m_oldnew);
                        // println!("== {:?}", m_oldnew);
                        //-- geometry/texture
                        // cat_update_texture(&mut g, &mut t_oldnew, &mut t_v_oldnew);
                    }
                }
                None => (),
            }
            // cjf["CityObjects"][key] = serde_json::to_value(&co2).unwrap();

            //-- TODO: to fix: children-of-children?

            //-- "slice" geometry vertices
            cjf.add_co(key.clone(), co2);
            let mut g_new_vertices: Vec<Vec<i64>> = Vec::new();
            for v in &g_new_vi {
                g_new_vertices.push(cjj.vertices[*v].clone());
            }
            // cjf["vertices"] = serde_json::to_value(&g_new_vertices).unwrap();
            io::stdout()
                .write_all(&format!("{}\n", serde_json::to_string(&cjf).unwrap()).as_bytes())?;
        }
    }
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

    let mut allappearance: Option<Appearance> = None;
    match j.pointer_mut("/appearance") {
        Some(x) => {
            allappearance = serde_json::from_value(x.take()).unwrap();
            // println!("{:?}", allappearance);
            // std::process::exit(1);
        }
        None => (),
    }

    //-- TODO: handling "default-theme-texture": "myDefaultTheme1"?

    let cos: HashMap<String, CityObject> = serde_json::from_value(j["CityObjects"].take()).unwrap();
    let g_old_vertices: Vec<Vec<i64>> = serde_json::from_value(j["vertices"].take()).unwrap();
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
            let mut t_oldnew: HashMap<usize, usize> = HashMap::new();
            let mut t_v_oldnew: HashMap<usize, usize> = HashMap::new();
            match &mut co2.geometry {
                Some(x) => {
                    for mut g in x.iter_mut() {
                        //-- geometry/boundaries
                        cat_update_geometry_vi(&mut g, &mut g_vi_oldnew, &mut g_new_vi);
                        //-- geometry/material
                        cat_update_material(&mut g, &mut m_oldnew);
                        // println!("== {:?}", m_oldnew);
                        //-- geometry/texture
                        cat_update_texture(&mut g, &mut t_oldnew, &mut t_v_oldnew);
                    }
                }
                None => (),
            }
            cjf["CityObjects"][key] = serde_json::to_value(&co2).unwrap();

            //-- TODO: to fix: children-of-children?
            //-- process all the children (only one-level lower)
            // for childkey in co.get_children_keys() {
            //     let coc = cos.get(&childkey).unwrap();
            //     let coc2: &mut CityObject = &mut coc.clone();
            //     match &mut coc2.geometry {
            //         Some(x) => {
            //             for mut g in x.iter_mut() {
            //                 cat_update_geometry_vi(&mut g, &mut g_vi_oldnew, &mut g_new_vi);
            //                 // TODO: material + children
            //             }
            //         }
            //         None => (),
            //     }
            //     cjf["CityObjects"][childkey] = serde_json::to_value(&coc2).unwrap();
            // }

            //-- "slice" geometry vertices
            let mut g_new_vertices: Vec<Vec<i64>> = Vec::new();
            for v in &g_new_vi {
                g_new_vertices.push(g_old_vertices[*v].clone());
            }
            cjf["vertices"] = serde_json::to_value(&g_new_vertices).unwrap();

            //-- "slice" materials
            if allappearance.is_some() {
                let aa: &Appearance = allappearance.as_ref().unwrap();
                // let aa = &(allappearance.unwrap());
                let mut acjf: Appearance = Appearance::new();
                acjf.default_theme_material = aa.default_theme_material.clone();
                acjf.default_theme_texture = aa.default_theme_texture.clone();
                if aa.materials.is_some() {
                    let am = aa.materials.as_ref().unwrap();
                    let mut mats2: Vec<Value> = Vec::new();
                    mats2.resize(m_oldnew.len(), json!(null));
                    for (old, new) in &m_oldnew {
                        mats2[*new] = am[*old].clone();
                    }
                    acjf.materials = Some(mats2);
                    // cjf["appearance"]["materials"] = serde_json::to_value(&mats2).unwrap();
                }
                if aa.textures.is_some() {
                    let at = aa.textures.as_ref().unwrap();
                    let mut texs2: Vec<Value> = Vec::new();
                    texs2.resize(t_oldnew.len(), json!(null));
                    for (old, new) in &t_oldnew {
                        texs2[*new] = at[*old].clone();
                    }
                    acjf.textures = Some(texs2);
                    // cjf["appearance"]["textures"] = serde_json::to_value(&texs2).unwrap();
                }
                if aa.vertices_texture.is_some() {
                    let atv = aa.vertices_texture.as_ref().unwrap();
                    let mut t_new_vertices: Vec<Vec<f64>> = Vec::new();
                    t_new_vertices.resize(t_v_oldnew.len(), vec![]);
                    for (old, new) in &t_v_oldnew {
                        t_new_vertices[*new] = atv[*old].clone();
                    }
                    acjf.vertices_texture = Some(t_new_vertices);
                    // cjf["appearance"]["vertices-texture"] =
                    // serde_json::to_value(&t_new_vertices).unwrap();
                }
                // println!("{:?}", aa);
                // println!("{:?}", acjf);
                cjf["appearance"] = serde_json::to_value(&acjf).unwrap();
            }
            // match j.pointer_mut("/appearance/materials") {
            //     Some(_x) => {
            //         let mut mats2: Vec<Value> = Vec::new();
            //         mats2.resize(m_oldnew.len(), json!(null));
            //         for (old, new) in &m_oldnew {
            //             mats2[*new] = allmats[*old].clone();
            //         }
            //         cjf["appearance"]["materials"] = serde_json::to_value(&mats2).unwrap();
            //     }
            //     None => (),
            // }

            // //-- "slice" textures
            // match j.pointer_mut("/appearance/textures") {
            //     Some(_x) => {
            //         let mut texs2: Vec<Value> = Vec::new();
            //         texs2.resize(t_oldnew.len(), json!(null));
            //         for (old, new) in &t_oldnew {
            //             texs2[*new] = alltexs[*old].clone();
            //         }
            //         cjf["appearance"]["textures"] = serde_json::to_value(&texs2).unwrap();
            //     }
            //     None => (),
            // }
            // //-- "slice" vertices-texture
            // match j.pointer_mut("/appearance/vertices-texture") {
            //     Some(_x) => {
            //         let mut t_new_vertices: Vec<Vec<f64>> = Vec::new();
            //         t_new_vertices.resize(t_v_oldnew.len(), vec![]);
            //         for (old, new) in &t_v_oldnew {
            //             t_new_vertices[*new] = t_old_vertices[*old].clone();
            //         }
            //         cjf["appearance"]["vertices-texture"] =
            //             serde_json::to_value(&t_new_vertices).unwrap();
            //     }
            //     None => (),
            // }

            //-- write to stdout
            io::stdout()
                .write_all(&format!("{}\n", serde_json::to_string(&cjf).unwrap()).as_bytes())?;
        }
    }
    Ok(())
}

fn cat_update_texture(
    g: &mut Geometry,
    t_oldnew: &mut HashMap<usize, usize>,
    t_v_oldnew: &mut HashMap<usize, usize>,
) {
    match &mut g.texture {
        Some(x) => {
            for (_key, tex) in &mut *x {
                if g.thetype == "MultiSurface" || g.thetype == "CompositeSurface" {
                    let a: Vec<Vec<Vec<Option<usize>>>> =
                        serde_json::from_value(tex.values.take().into()).unwrap();
                    let mut a2 = a.clone();
                    for (i, x) in a.iter().enumerate() {
                        for (j, y) in x.iter().enumerate() {
                            for (k, z) in y.iter().enumerate() {
                                if z.is_some() {
                                    let thevalue: usize = z.unwrap();
                                    if k == 0 {
                                        let y2 = t_oldnew.get(&thevalue);
                                        if y2.is_none() {
                                            let l = t_oldnew.len();
                                            t_oldnew.insert(thevalue, l);
                                            a2[i][j][k] = Some(l);
                                        } else {
                                            let y2 = y2.unwrap();
                                            a2[i][j][k] = Some(*y2);
                                        }
                                    } else {
                                        let y2 = t_v_oldnew.get(&thevalue);
                                        if y2.is_none() {
                                            let l = t_v_oldnew.len();
                                            t_v_oldnew.insert(thevalue, l);
                                            a2[i][j][k] = Some(l);
                                        } else {
                                            let y2 = y2.unwrap();
                                            a2[i][j][k] = Some(*y2);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    tex.values = Some(serde_json::to_value(&a2).unwrap());
                }
            }
        }
        None => (),
    }
}

fn cat_update_material(g: &mut Geometry, m_oldnew: &mut HashMap<usize, usize>) {
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

fn cat_update_geometry_vi(
    g: &mut Geometry,
    violdnew: &mut HashMap<usize, usize>,
    newvi: &mut Vec<usize>,
) {
    // TODO: GeometryInstance?
    if g.thetype == "MultiPoint" {
        let a: Vec<usize> = serde_json::from_value(g.boundaries.clone()).unwrap();
        let mut a2 = a.clone();
        for (i, x) in a.iter().enumerate() {
            let kk = violdnew.get(&x);
            if kk.is_none() {
                let l = newvi.len();
                violdnew.insert(*x, l);
                newvi.push(*x);
                a2[i] = l;
            } else {
                let kk = kk.unwrap();
                a2[i] = *kk;
            }
        }
        g.boundaries = serde_json::to_value(&a2).unwrap();
    } else if g.thetype == "MultiLineString" {
        let a: Vec<Vec<usize>> = serde_json::from_value(g.boundaries.take()).unwrap();
        let mut a2 = a.clone();
        for (i, x) in a.iter().enumerate() {
            for (j, y) in x.iter().enumerate() {
                // r.push(z);
                let kk = violdnew.get(&y);
                if kk.is_none() {
                    let l = newvi.len();
                    violdnew.insert(*y, l);
                    newvi.push(*y);
                    a2[i][j] = l;
                } else {
                    let kk = kk.unwrap();
                    a2[i][j] = *kk;
                }
            }
        }
        g.boundaries = serde_json::to_value(&a2).unwrap();
    } else if g.thetype == "MultiSurface" || g.thetype == "CompositeSurface" {
        let a: Vec<Vec<Vec<usize>>> = serde_json::from_value(g.boundaries.take()).unwrap();
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
        let a: Vec<Vec<Vec<Vec<usize>>>> = serde_json::from_value(g.boundaries.take()).unwrap();
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
    } else if g.thetype == "MultiSolid" || g.thetype == "CompositeSolid" {
        let a: Vec<Vec<Vec<Vec<Vec<usize>>>>> =
            serde_json::from_value(g.boundaries.take()).unwrap();
        let mut a2 = a.clone();
        for (i, x) in a.iter().enumerate() {
            for (j, y) in x.iter().enumerate() {
                for (k, z) in y.iter().enumerate() {
                    for (l, zz) in z.iter().enumerate() {
                        for (m, zzz) in zz.iter().enumerate() {
                            let kk = violdnew.get(&zzz);
                            if kk.is_none() {
                                let length = newvi.len();
                                violdnew.insert(*zzz, length);
                                newvi.push(*zzz);
                                a2[i][j][k][l][m] = length;
                            } else {
                                let kk = kk.unwrap();
                                a2[i][j][k][l][m] = *kk;
                            }
                        }
                    }
                }
            }
        }
        g.boundaries = serde_json::to_value(&a2).unwrap();
    }
}
