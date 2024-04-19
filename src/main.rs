use crate::cityjson::Appearance;
use crate::cityjson::CityJSON;
use crate::cityjson::CityJSONFeature;
use crate::cityjson::CityObject;
use crate::cityjson::GeometryTemplates;
use crate::cityjson::Transform;
use serde_json::{json, Value};

extern crate clap;

use rand::Rng;
use std::fmt;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::{self, Read, Write};
use std::path::PathBuf;

use std::collections::HashMap;

mod cityjson;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about = "Create/process/modify CityJSONSeq files", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// CityJSONSeq ==> CityJSON
    Cat {
        /// CityJSONSeq input file
        #[arg(short, long)]
        file: Option<PathBuf>,
    },
    /// CityJSON ==> CityJSONSeq
    Collect {
        /// CityJSON input file
        #[arg(short, long)]
        file: Option<PathBuf>,
    },
    /// Filter a CityJSONSeq
    Filter {
        /// bbox: minx miny maxx maxy
        #[arg(long, value_delimiter = ' ', num_args = 4, group = "exclusive")]
        bbox: Option<Vec<f64>>,
        /// Keep only the CityObjects of this type
        #[arg(long, group = "exclusive")]
        cotype: Option<String>,
        // Excludes the selection, thus delete the selected city object(s).
        #[arg(long)]
        exclude: bool,
        /// x y radius
        #[arg(long, value_delimiter = ' ', num_args = 3, group = "exclusive")]
        radius: Option<Vec<f64>>,
        /// 1/X chances of a given feature be kept
        #[arg(long, value_parser = clap::value_parser!(u32).range(1..), group = "exclusive")]
        random: Option<u32>,
    },
}

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

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        //-- cat
        Commands::Cat { file } => match file {
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
        //-- collect
        Commands::Collect { file } => match file {
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
        //-- filter
        Commands::Filter {
            bbox,
            cotype,
            exclude,
            radius,
            random,
        } => {
            if bbox.is_some() {
                if let Err(e) = filter_bbox(*exclude, &bbox.clone().unwrap()) {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
            if cotype.is_some() {
                if let Err(e) = filter_cotype(*exclude, cotype.clone().unwrap()) {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
            if radius.is_some() {
                let p: Vec<f64> = radius.clone().unwrap();
                if let Err(e) = filter_radius(*exclude, p[0], p[1], p[2]) {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
            if random.is_some() {
                if let Err(e) = filter_random(*exclude, random.unwrap()) {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
        }
    }
}

fn filter_random(exclude: bool, rand_factor: u32) -> Result<(), MyError> {
    let stdin = std::io::stdin();
    let mut rng = rand::thread_rng();
    for (i, line) in stdin.lock().lines().enumerate() {
        let mut w: bool = false;
        let l = line.unwrap();
        if i == 0 {
            io::stdout().write_all(&format!("{}\n", l).as_bytes())?;
        } else {
            let r: u32 = rng.gen_range(1..=rand_factor);
            if r == 1 {
                w = true;
            }
            if (w == true && !exclude) || (w == false && exclude) {
                io::stdout().write_all(&format!("{}\n", l).as_bytes())?;
            }
        }
    }
    Ok(())
}

fn filter_cotype(exclude: bool, cotype: String) -> Result<(), MyError> {
    let stdin = std::io::stdin();
    for (i, line) in stdin.lock().lines().enumerate() {
        let mut w: bool = false;
        let l = line.unwrap();
        if i == 0 {
            io::stdout().write_all(&format!("{}\n", l).as_bytes())?;
        } else {
            let cjf: CityJSONFeature = serde_json::from_str(&l)?;
            if cjf.city_objects[&cjf.id].thetype == cotype {
                w = true;
            }
            if (w == true && !exclude) || (w == false && exclude) {
                io::stdout().write_all(&format!("{}\n", l).as_bytes())?;
            }
        }
    }
    Ok(())
}

fn filter_bbox(exclude: bool, bbox: &Vec<f64>) -> Result<(), MyError> {
    let stdin = std::io::stdin();
    let mut transform: Transform = Transform::new();
    for (i, line) in stdin.lock().lines().enumerate() {
        let mut w: bool = false;
        let l = line.unwrap();
        if i == 0 {
            io::stdout().write_all(&format!("{}\n", l).as_bytes())?;
            let cj: CityJSON = serde_json::from_str(&l)?;
            transform = cj.transform;
        } else {
            let cjf: CityJSONFeature = serde_json::from_str(&l)?;
            let ci = cjf.centroid();
            let cx = (ci[0] * transform.scale[0]) + transform.translate[0];
            let cy = (ci[1] * transform.scale[1]) + transform.translate[1];
            if (cx > bbox[0]) && (cx < bbox[2]) && (cy > bbox[1]) && (cy < bbox[3]) {
                w = true;
            }
            if (w == true && !exclude) || (w == false && exclude) {
                io::stdout().write_all(&format!("{}\n", l).as_bytes())?;
            }
        }
    }
    Ok(())
}

fn filter_radius(exclude: bool, x: f64, y: f64, r: f64) -> Result<(), MyError> {
    let stdin = std::io::stdin();
    let mut transform: Transform = Transform::new();
    for (i, line) in stdin.lock().lines().enumerate() {
        let mut w: bool = false;
        let l = line.unwrap();
        if i == 0 {
            io::stdout().write_all(&format!("{}\n", l).as_bytes())?;
            let cj: CityJSON = serde_json::from_str(&l)?;
            transform = cj.transform;
        } else {
            let cjf: CityJSONFeature = serde_json::from_str(&l)?;
            let ci = cjf.centroid();
            let cx = (ci[0] * transform.scale[0]) + transform.translate[0];
            let cy = (ci[1] * transform.scale[1]) + transform.translate[1];
            let d2 = (cx - x).powf(2.0) + (cy - y).powf(2.0);
            if d2 <= (r * r) {
                w = true;
            }
            if (w == true && !exclude) || (w == false && exclude) {
                io::stdout().write_all(&format!("{}\n", l).as_bytes())?;
            }
        }
    }
    Ok(())
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
    cjj.retransform();
    cjj.remove_duplicate_vertices();
    io::stdout().write_all(&format!("{}\n", serde_json::to_string(&cjj).unwrap()).as_bytes())?;
    Ok(())
}

fn collect_from_file(file: &PathBuf) -> Result<(), MyError> {
    let f = File::open(file.canonicalize()?)?;
    let br = BufReader::new(f);
    let mut cjj: CityJSON = CityJSON::new();
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
    cjj.remove_duplicate_vertices();
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
            eprintln!("Error: {}", error);
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
    let mut cj1: CityJSON = cjj.get_empty_copy();
    //-- if geometry-templates have material/textures then these need to be added to 1st line
    match &cjj.geometry_templates {
        Some(x) => {
            let mut gts2: GeometryTemplates = x.clone();
            let mut m_oldnew: HashMap<usize, usize> = HashMap::new();
            let mut t_oldnew: HashMap<usize, usize> = HashMap::new();
            let mut t_v_oldnew: HashMap<usize, usize> = HashMap::new();
            for g in &mut gts2.templates {
                g.update_material(&mut m_oldnew);
                g.update_texture(&mut t_oldnew, &mut t_v_oldnew, 0);
            }
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
                cj1.appearance = Some(acjf);
            }
        }
        None => (),
    }
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
                        g.update_geometry_boundaries(&mut g_vi_oldnew);
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
                            g.update_geometry_boundaries(&mut g_vi_oldnew);
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
