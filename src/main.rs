use cjseq::CityJSON;
use cjseq::CityJSONFeature;

extern crate clap;
use clap::{Parser, Subcommand};

use rand::Rng;
use std::fmt;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::{self, Read, Write};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about = "Create/process/modify CityJSONSeq files", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// CityJSON ==> CityJSONSeq
    Cat {
        /// CityJSONSeq input file
        #[arg(short, long)]
        file: Option<PathBuf>,
    },
    /// CityJSONSeq ==> CityJSON
    Collect {
        /// CityJSON input file
        #[arg(short, long)]
        file: Option<PathBuf>,
    },
    /// Filter a CityJSONSeq
    Filter {
        /// Bounding box filter
        #[arg(long, value_names = &["minx", "miny", "maxx", "maxy"], value_delimiter = ' ', num_args = 4, group = "exclusive")]
        bbox: Option<Vec<f64>>,
        /// Keep only the CityObjects of this type
        #[arg(long, group = "exclusive")]
        cotype: Option<String>,
        /// Excludes the selection, thus delete the selected city object(s)
        #[arg(long)]
        exclude: bool,
        /// Circle filter: centre + radius
        #[arg(
            long,
            value_names = &["x", "y", "radius"],
            value_delimiter = ' ',
            num_args = 3,
            group = "exclusive"
        )]
        radius: Option<Vec<f64>>,
        /// 1/X chances of a given feature being kept
        #[arg(long, value_name = "X", value_parser = clap::value_parser!(u32).range(1..), group = "exclusive")]
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
            if cjf.city_objects[&cjf.id].get_type() == cotype {
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
    let mut cj: CityJSON = CityJSON::new();
    for (i, line) in stdin.lock().lines().enumerate() {
        let mut w: bool = false;
        let l = line.unwrap();
        if i == 0 {
            io::stdout().write_all(&format!("{}\n", l).as_bytes())?;
            cj = serde_json::from_str(&l)?;
        } else {
            let cjf: CityJSONFeature = serde_json::from_str(&l)?;
            let ci = cjf.centroid();
            let cx = (ci[0] * cj.transform.scale[0]) + cj.transform.translate[0];
            let cy = (ci[1] * cj.transform.scale[1]) + cj.transform.translate[1];
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
    let mut cj: CityJSON = CityJSON::new();
    for (i, line) in stdin.lock().lines().enumerate() {
        let mut w: bool = false;
        let l = line.unwrap();
        if i == 0 {
            io::stdout().write_all(&format!("{}\n", l).as_bytes())?;
            cj = serde_json::from_str(&l)?;
        } else {
            let cjf: CityJSONFeature = serde_json::from_str(&l)?;
            let ci = cjf.centroid();
            let cx = (ci[0] * cj.transform.scale[0]) + cj.transform.translate[0];
            let cy = (ci[1] * cj.transform.scale[1]) + cj.transform.translate[1];
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
    let mut cjj = CityJSON::new();
    for (i, line) in stdin.lock().lines().enumerate() {
        let l = line.unwrap();
        if i == 0 {
            cjj = CityJSON::from_str(&l)?;
        } else {
            let cjf = CityJSONFeature::from_str(&l)?;
            cjj.add_one_cjf(cjf);
        }
    }
    cjj.remove_duplicate_vertices();
    cjj.retransform();
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
    cjj.retransform();
    io::stdout().write_all(&format!("{}\n", serde_json::to_string(&cjj).unwrap()).as_bytes())?;
    Ok(())
}

fn cat_from_stdin() -> Result<(), MyError> {
    let mut input = String::new();
    match std::io::stdin().read_to_string(&mut input) {
        Ok(_) => {
            let mut cjj: CityJSON = CityJSON::from_str(&input)?;
            let _ = cat(&mut cjj)?;
        }
        Err(error) => {
            eprintln!("Error: {}", error);
        }
    }
    Ok(())
}

fn cat_from_file(file: &PathBuf) -> Result<(), MyError> {
    let f = File::open(file.canonicalize()?)?;
    let mut br = BufReader::new(f);
    let mut json_content = String::new();
    br.read_to_string(&mut json_content)?;
    let mut cjj: CityJSON = CityJSON::from_str(&json_content)?;
    cat(&mut cjj)?;
    Ok(())
}

fn cat(cjj: &mut CityJSON) -> Result<(), MyError> {
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
    cjj.sort_features(cjseq::SortingStrategy::Alphabetical);
    //-- first line: the CityJSON "metadata"
    let cj1 = cjj.cat_metadata();
    io::stdout().write_all(&format!("{}\n", serde_json::to_string(&cj1).unwrap()).as_bytes())?;
    //-- the other lines for each CityJSONSeq
    let mut i: usize = 0;
    while let Some(cjf) = cjj.cat_feature(i) {
        i += 1;
        io::stdout()
            .write_all(&format!("{}\n", serde_json::to_string(&cjf).unwrap()).as_bytes())?;
    }
    Ok(())
}
