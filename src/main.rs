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
    fn new() -> Self {
        let co: HashMap<String, CityObject> = HashMap::new();
        let v: Vec<Vec<i64>> = Vec::new();
        let tr = json!({
          "transform": {
            "scale": [1.0, 1.0, 1.0],
            "translate": [0.0, 0.0, 0.0]
          },
        });
        CityJSON {
            thetype: "CityJSON".to_string(),
            version: "2.0".to_string(),
            transform: tr,
            city_objects: co,
            vertices: v,
            metadata: None,
            appearance: None,
            geometry_templates: None,
            extensions: None,
            other: json!(null),
        }
    }
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
    fn add_co(&mut self, id: String, co: CityObject) {
        self.city_objects.insert(id, co);
    }
    fn add_vertices(&mut self, mut v: Vec<Vec<i64>>) {
        self.vertices.append(&mut v);
    }
    fn add_vertices_texture(&mut self, vs: Vec<Vec<f64>>) {
        match &mut self.appearance {
            Some(x) => x.add_vertices_texture(vs),
            None => {
                let mut a: Appearance = Appearance::new();
                a.add_vertices_texture(vs);
                self.appearance = Some(a);
            }
        };
    }
    fn add_material(&mut self, jm: Value) -> usize {
        let re = match &mut self.appearance {
            Some(x) => x.add_material(jm),
            None => {
                let mut a: Appearance = Appearance::new();
                let re = a.add_material(jm);
                self.appearance = Some(a);
                re
            }
        };
        re
    }
    fn add_texture(&mut self, jm: Value) -> usize {
        let re = match &mut self.appearance {
            Some(x) => x.add_texture(jm),
            None => {
                let mut a: Appearance = Appearance::new();
                let re = a.add_texture(jm);
                self.appearance = Some(a);
                re
            }
        };
        re
    }
    fn add_one_cjf(&mut self, mut cjf: CityJSONFeature) {
        let mut g_oldnew: HashMap<usize, usize> = HashMap::new();
        let mut m_oldnew: HashMap<usize, usize> = HashMap::new();
        let mut t_oldnew: HashMap<usize, usize> = HashMap::new();
        let mut t_v_oldnew: HashMap<usize, usize> = HashMap::new();
        let g_offset = self.vertices.len();
        println!("=>{:?}--{}", cjf.id, g_offset);
        let mut t_offset = 0;
        if let Some(cjf_app) = &cjf.appearance {
            // println!("{:?}", cjf_app);
            if let Some(cjf_mat) = &cjf_app.materials {
                // println!("{:?}", cjf_mat);
                for (i, m) in cjf_mat.iter().enumerate() {
                    m_oldnew.insert(i, self.add_material(m.clone()));
                }
            }
            if let Some(cjf_tex) = &cjf_app.textures {
                for (i, m) in cjf_tex.iter().enumerate() {
                    t_oldnew.insert(i, self.add_texture(m.clone()));
                }
            }
            if let Some(cjf_v_tex) = &cjf_app.vertices_texture {
                t_offset = cjf_v_tex.len();
                self.add_vertices_texture(cjf_v_tex.clone());
            }
        }

        for (key, co) in &mut cjf.city_objects {
            //-- boundaries
            if let Some(ref mut geoms) = &mut co.geometry {
                // TODO : add other Geometric primitives
                for g in geoms.iter_mut() {
                    //-- boundaries
                    g.update_geometry_boundaries(&mut g_oldnew, g_offset);
                    //-- material
                    g.update_material(&mut m_oldnew);
                    //-- texture
                    g.update_texture(&mut t_oldnew, &mut t_v_oldnew, t_offset);
                }
            }
            //-- update the collected json object by adding the CityObjects
            self.add_co(key.to_string(), co.clone());
        }
        //-- add the new vertices
        self.add_vertices(cjf.vertices.clone());
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
impl Geometry {
    fn update_geometry_boundaries(&mut self, violdnew: &mut HashMap<usize, usize>, offset: usize) {
        // TODO: GeometryInstance?
        // let l2: usize = violdnew.len();
        if self.thetype == "MultiPoint" {
            let a: Vec<usize> = serde_json::from_value(self.boundaries.clone()).unwrap();
            let mut a2 = a.clone();
            for (i, x) in a.iter().enumerate() {
                let kk = violdnew.get(&x);
                if kk.is_none() {
                    let l = violdnew.len() + offset;
                    violdnew.insert(*x, l);
                    a2[i] = l;
                } else {
                    let kk = kk.unwrap();
                    a2[i] = *kk;
                }
            }
            self.boundaries = serde_json::to_value(&a2).unwrap();
        } else if self.thetype == "MultiLineString" {
            let a: Vec<Vec<usize>> = serde_json::from_value(self.boundaries.take()).unwrap();
            let mut a2 = a.clone();
            for (i, x) in a.iter().enumerate() {
                for (j, y) in x.iter().enumerate() {
                    // r.push(z);
                    let kk = violdnew.get(&y);
                    if kk.is_none() {
                        let l = violdnew.len() + offset;
                        violdnew.insert(*y, l);
                        a2[i][j] = l;
                    } else {
                        let kk = kk.unwrap();
                        a2[i][j] = *kk;
                    }
                }
            }
            self.boundaries = serde_json::to_value(&a2).unwrap();
        } else if self.thetype == "MultiSurface" || self.thetype == "CompositeSurface" {
            let a: Vec<Vec<Vec<usize>>> = serde_json::from_value(self.boundaries.take()).unwrap();
            let mut a2 = a.clone();
            for (i, x) in a.iter().enumerate() {
                for (j, y) in x.iter().enumerate() {
                    for (k, z) in y.iter().enumerate() {
                        // r.push(z);
                        let kk = violdnew.get(&z);
                        if kk.is_none() {
                            let l = violdnew.len() + offset;
                            violdnew.insert(*z, l);
                            a2[i][j][k] = l;
                        } else {
                            let kk = kk.unwrap();
                            a2[i][j][k] = *kk;
                        }
                    }
                }
            }
            self.boundaries = serde_json::to_value(&a2).unwrap();
        } else if self.thetype == "Solid" {
            let a: Vec<Vec<Vec<Vec<usize>>>> =
                serde_json::from_value(self.boundaries.take()).unwrap();
            let mut a2 = a.clone();
            for (i, x) in a.iter().enumerate() {
                for (j, y) in x.iter().enumerate() {
                    for (k, z) in y.iter().enumerate() {
                        for (l, zz) in z.iter().enumerate() {
                            // r.push(z);
                            let kk = violdnew.get(&zz);
                            if kk.is_none() {
                                let l = violdnew.len() + offset;
                                violdnew.insert(*zz, l);
                                a2[i][j][k][l] = l;
                            } else {
                                let kk = kk.unwrap();
                                a2[i][j][k][l] = *kk;
                            }
                        }
                    }
                }
            }
            self.boundaries = serde_json::to_value(&a2).unwrap();
        } else if self.thetype == "MultiSolid" || self.thetype == "CompositeSolid" {
            let a: Vec<Vec<Vec<Vec<Vec<usize>>>>> =
                serde_json::from_value(self.boundaries.take()).unwrap();
            let mut a2 = a.clone();
            for (i, x) in a.iter().enumerate() {
                for (j, y) in x.iter().enumerate() {
                    for (k, z) in y.iter().enumerate() {
                        for (l, zz) in z.iter().enumerate() {
                            for (m, zzz) in zz.iter().enumerate() {
                                let kk = violdnew.get(&zzz);
                                if kk.is_none() {
                                    let l = violdnew.len() + offset;
                                    violdnew.insert(*zzz, l);
                                    a2[i][j][k][l][m] = l;
                                } else {
                                    let kk = kk.unwrap();
                                    a2[i][j][k][l][m] = *kk;
                                }
                            }
                        }
                    }
                }
            }
            self.boundaries = serde_json::to_value(&a2).unwrap();
        }
    }
    fn update_material(&mut self, m_oldnew: &mut HashMap<usize, usize>) {
        match &mut self.material {
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
                    if self.thetype == "MultiSurface" || self.thetype == "CompositeSurface" {
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
                    } else if self.thetype == "Solid" {
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
                    } else if self.thetype == "MultiSolid" || self.thetype == "CompositeSolid" {
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
                self.material = Some(x.clone());
            }
            None => (),
        }
    }
    fn update_texture(
        &mut self,
        t_oldnew: &mut HashMap<usize, usize>,
        t_v_oldnew: &mut HashMap<usize, usize>,
        offset: usize,
    ) {
        match &mut self.texture {
            Some(x) => {
                for (_key, tex) in &mut *x {
                    //-- TODO: other Geometry type for textures
                    if self.thetype == "MultiSurface" || self.thetype == "CompositeSurface" {
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
                                                t_v_oldnew.insert(thevalue, l + offset);
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
                    x.len() - 1
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
    fn add_texture(&mut self, jm: Value) -> usize {
        let re = match &mut self.textures {
            Some(x) => match x.iter().position(|e| *e == jm) {
                Some(y) => y,
                None => {
                    x.push(jm);
                    x.len() - 1
                }
            },
            None => {
                let mut ls: Vec<Value> = Vec::new();
                ls.push(jm);
                self.textures = Some(ls);
                0
            }
        };
        re
    }
    fn add_vertices_texture(&mut self, mut vs: Vec<Vec<f64>>) {
        match &mut self.vertices_texture {
            Some(x) => {
                x.append(&mut vs);
            }
            None => {
                let mut ls: Vec<Vec<f64>> = Vec::new();
                ls.append(&mut vs);
                self.vertices_texture = Some(ls);
            }
        };
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
    let mut cjj: CityJSON = CityJSON::new();
    for (i, line) in stdin.lock().lines().enumerate() {
        let l = line.unwrap();
        if i == 0 {
            cjj = serde_json::from_str(&l)?;
        } else {
            let cjf: CityJSONFeature = serde_json::from_str(&l)?;
            cjj.add_one_cjf(cjf);
        }
    }
    io::stdout().write_all(&format!("{}\n", serde_json::to_string(&cjj).unwrap()).as_bytes())?;
    Ok(())
}

fn collect_from_file(file: &PathBuf) -> Result<(), MyError> {
    let f = File::open(file.canonicalize()?)?;
    let br = BufReader::new(f);
    let mut cjj: CityJSON = CityJSON::new();
    // let mut j: Value = json!(null);
    // let mut all_g_v: Vec<Vec<i64>> = Vec::new();
    // let mut all_app: Appearance = Appearance::new();
    for (i, line) in br.lines().enumerate() {
        match &line {
            Ok(l) => {
                if i == 0 {
                    cjj = serde_json::from_str(&l)?;
                } else {
                    let cjf: CityJSONFeature = serde_json::from_str(&l)?;
                    cjj.add_one_cjf(cjf);
                }
            }
            Err(error) => eprintln!("Error reading line: {}", error),
        }
    }
    io::stdout().write_all(&format!("{}\n", serde_json::to_string(&cjj).unwrap()).as_bytes())?;
    Ok(())
}

fn cat_from_stdin() -> Result<(), MyError> {
    let mut input = String::new();
    match std::io::stdin().read_to_string(&mut input) {
        Ok(_) => {
            let cjj: CityJSON = serde_json::from_str(&input)?;
            let _ = cat(&cjj)?;
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
    let cjj: CityJSON = serde_json::from_reader(br)?;
    cat(&cjj)?;
    Ok(())
}

fn cat(cjj: &CityJSON) -> Result<(), MyError> {
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

    //-- the other lines
    let cos = &cjj.city_objects;
    for (key, co) in cos {
        if co.is_toplevel() {
            let mut cjf = CityJSONFeature::new();
            let mut co2: CityObject = co.clone();
            let mut g_vi_oldnew: HashMap<usize, usize> = HashMap::new();
            let mut m_oldnew: HashMap<usize, usize> = HashMap::new();
            let mut t_oldnew: HashMap<usize, usize> = HashMap::new();
            let mut t_v_oldnew: HashMap<usize, usize> = HashMap::new();
            match &mut co2.geometry {
                Some(x) => {
                    for g in x.iter_mut() {
                        g.update_geometry_boundaries(&mut g_vi_oldnew, 0);
                        g.update_material(&mut m_oldnew);
                        g.update_texture(&mut t_oldnew, &mut t_v_oldnew, 0);
                    }
                }
                None => (),
            }
            cjf.add_co(key.clone(), co2);
            cjf.id = key.to_string();

            //-- TODO: to fix: children-of-children?
            //-- process all the children (only one-level lower)
            for childkey in co.get_children_keys() {
                let coc = cos.get(&childkey).unwrap();
                let mut coc2: CityObject = coc.clone();
                match &mut coc2.geometry {
                    Some(x) => {
                        for g in x.iter_mut() {
                            g.update_geometry_boundaries(&mut g_vi_oldnew, 0);
                            g.update_material(&mut m_oldnew);
                            g.update_texture(&mut t_oldnew, &mut t_v_oldnew, 0);
                        }
                    }
                    None => (),
                }
                cjf.add_co(childkey.clone(), coc2);
            }

            //-- "slice" geometry vertices
            let allvertices = &cjj.vertices;
            let mut g_new_vertices: Vec<Vec<i64>> = Vec::new();
            g_new_vertices.resize(g_vi_oldnew.len(), vec![]);
            for (old, new) in &g_vi_oldnew {
                g_new_vertices[*new] = allvertices[*old].clone();
            }
            cjf.vertices = g_new_vertices;

            //-- "slice" materials
            if cjj.appearance.is_some() {
                let a = cjj.appearance.as_ref().unwrap();
                let mut acjf: Appearance = Appearance::new();
                acjf.default_theme_material = a.default_theme_material.clone();
                acjf.default_theme_texture = a.default_theme_texture.clone();
                if a.materials.is_some() {
                    let am = a.materials.as_ref().unwrap();
                    let mut mats2: Vec<Value> = Vec::new();
                    mats2.resize(m_oldnew.len(), json!(null));
                    for (old, new) in &m_oldnew {
                        mats2[*new] = am[*old].clone();
                    }
                    acjf.materials = Some(mats2);
                }
                if a.textures.is_some() {
                    let at = a.textures.as_ref().unwrap();
                    let mut texs2: Vec<Value> = Vec::new();
                    texs2.resize(t_oldnew.len(), json!(null));
                    for (old, new) in &t_oldnew {
                        texs2[*new] = at[*old].clone();
                    }
                    acjf.textures = Some(texs2);
                }
                if a.vertices_texture.is_some() {
                    let atv = a.vertices_texture.as_ref().unwrap();
                    let mut t_new_vertices: Vec<Vec<f64>> = Vec::new();
                    t_new_vertices.resize(t_v_oldnew.len(), vec![]);
                    for (old, new) in &t_v_oldnew {
                        t_new_vertices[*new] = atv[*old].clone();
                    }
                    acjf.vertices_texture = Some(t_new_vertices);
                }
                cjf.appearance = Some(acjf);
            }

            io::stdout()
                .write_all(&format!("{}\n", serde_json::to_string(&cjf).unwrap()).as_bytes())?;
        }
    }
    Ok(())
}
