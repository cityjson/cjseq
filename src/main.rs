use cjseq::error::Result;
use cjseq::CityJSON;
use cjseq::CityJSONFeature;
use cjseq::CityJSONType;
use cjseq::CityObjectType;
use cjseq::CjseqError;
use cjseq::Transform;
use std::str::FromStr;

extern crate clap;
use clap::{Parser, Subcommand, ValueEnum};
use serde_json::Value;

use rand::Rng;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(version, about = "Create/process/modify CityJSONSeq files", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, ValueEnum)]
pub enum SortingStrategy {
    /// The order of the input
    AsIs,
    Random,
    Lexicographical,
}

#[derive(Subcommand)]
enum Commands {
    /// CityJSON ==> CityJSONSeq
    Cat {
        /// CityJSON input file
        file: Option<PathBuf>,
        /// Sorting for the cat output
        #[arg(short, long, value_enum)]
        order: Option<SortingStrategy>,
    },
    /// CityJSONSeq ==> CityJSON
    Collect {
        /// CityJSONSeq input file(s) (globbing works)
        files: Option<Vec<String>>,
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

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        //-- cat
        Commands::Cat { file, order } => {
            let o2 = match order.clone().unwrap_or(SortingStrategy::AsIs) {
                SortingStrategy::AsIs => cjseq::SortingStrategy::AsIs,
                SortingStrategy::Random => cjseq::SortingStrategy::Random,
                SortingStrategy::Lexicographical => cjseq::SortingStrategy::Lexicographical,
            };
            match file {
                Some(x) => {
                    if let Err(e) = cat_from_file(x, o2) {
                        eprintln!("{e}");
                        std::process::exit(1);
                    }
                }
                None => {
                    if let Err(e) = cat_from_stdin(o2) {
                        eprintln!("{e}");
                        std::process::exit(1);
                    }
                }
            }
        }
        //-- collect
        Commands::Collect { files } => match files {
            Some(x) => {
                if let Err(e) = collect_from_files(x) {
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
            if bbox.is_none() && cotype.is_none() && radius.is_none() && random.is_none() {
                if let Err(e) = filter_random(*exclude, 1) {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
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

fn filter_random(exclude: bool, rand_factor: u32) -> Result<()> {
    let stdin = std::io::stdin();
    let mut rng = rand::thread_rng();
    for line in stdin.lock().lines() {
        let mut w: bool = false;
        if let Ok(l) = line {
            let j: Value = serde_json::from_str(&l)?;
            if j.get("type").unwrap() == "CityJSON" {
                io::stdout().write_all(format!("{}\n", l).as_bytes())?;
            } else {
                let r: u32 = rng.gen_range(1..=rand_factor);
                if r == 1 {
                    w = true;
                }
                if (w && !exclude) || (!w && exclude) {
                    io::stdout().write_all(format!("{}\n", l).as_bytes())?;
                }
            }
        }
    }
    Ok(())
}

fn filter_cotype(exclude: bool, cotype: String) -> Result<()> {
    // The CLI accepts any type name the user types (a core type such as
    // "Building", or an Extension type such as "+NoiseBuilding"); parse it
    // through the same enum used for the CityObjects being filtered so the
    // comparison below is enum-to-enum, not string-to-string.
    let cotype: CityObjectType = serde_json::from_value(serde_json::Value::String(cotype))?;
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let mut w: bool = false;
        if let Ok(l) = line {
            let j: Value = serde_json::from_str(&l)?;
            if j.get("type").unwrap() == "CityJSON" {
                io::stdout().write_all(format!("{}\n", l).as_bytes())?;
            } else {
                let cjf: CityJSONFeature = CityJSONFeature::from_str(&l)?;
                if cjf.city_objects[&cjf.id].get_type() == cotype {
                    w = true;
                }
                if (w && !exclude) || (!w && exclude) {
                    io::stdout().write_all(format!("{}\n", l).as_bytes())?;
                }
            }
        }
    }

    Ok(())
}

fn filter_bbox(exclude: bool, bbox: &[f64]) -> Result<()> {
    let stdin = std::io::stdin();
    let mut cj: CityJSON = CityJSON::new();
    for line in stdin.lock().lines() {
        let mut w: bool = false;
        if let Ok(l) = line {
            let j: Value = serde_json::from_str(&l)?;
            if j.get("type").unwrap() == "CityJSON" {
                io::stdout().write_all(format!("{}\n", l).as_bytes())?;
                cj = CityJSON::from_str(&l)?;
            } else {
                let cjf: CityJSONFeature = CityJSONFeature::from_str(&l)?;
                //-- a feature with no vertices is nowhere, so never inside
                if let Some(ci) = cjf.centroid() {
                    let cx = (ci[0] * cj.transform.scale[0]) + cj.transform.translate[0];
                    let cy = (ci[1] * cj.transform.scale[1]) + cj.transform.translate[1];
                    if (cx > bbox[0]) && (cx < bbox[2]) && (cy > bbox[1]) && (cy < bbox[3]) {
                        w = true;
                    }
                }
                if (w && !exclude) || (!w && exclude) {
                    io::stdout().write_all(format!("{}\n", l).as_bytes())?;
                }
            }
        }
    }
    Ok(())
}

fn filter_radius(exclude: bool, x: f64, y: f64, r: f64) -> Result<()> {
    let stdin = std::io::stdin();
    let mut cj: CityJSON = CityJSON::new();
    for line in stdin.lock().lines() {
        let mut w: bool = false;
        if let Ok(l) = line {
            let j: Value = serde_json::from_str(&l)?;
            if j.get("type").unwrap() == "CityJSON" {
                io::stdout().write_all(format!("{}\n", l).as_bytes())?;
                cj = CityJSON::from_str(&l)?;
            } else {
                let cjf: CityJSONFeature = CityJSONFeature::from_str(&l)?;
                //-- a feature with no vertices is nowhere, so never inside
                if let Some(ci) = cjf.centroid() {
                    let cx = (ci[0] * cj.transform.scale[0]) + cj.transform.translate[0];
                    let cy = (ci[1] * cj.transform.scale[1]) + cj.transform.translate[1];
                    let d2 = (cx - x).powf(2.0) + (cy - y).powf(2.0);
                    if d2 <= (r * r) {
                        w = true;
                    }
                }
                if (w && !exclude) || (!w && exclude) {
                    io::stdout().write_all(format!("{}\n", l).as_bytes())?;
                }
            }
        }
    }
    Ok(())
}

fn collect_from_stdin() -> Result<()> {
    let stdin = std::io::stdin();
    let mut cjj = CityJSON::new();
    let mut cjj_init: bool = false;
    for json_line in stdin.lock().lines().map_while(std::result::Result::ok) {
        let j: Value = serde_json::from_str(&json_line)?;
        if j.get("type").unwrap() == "CityJSON" {
            if !cjj_init {
                cjj = CityJSON::from_str(&json_line)?;
                cjj_init = true;
            } else {
                let t: Transform = serde_json::from_value(j.get("transform").unwrap().clone())?;
                cjj.add_transform_correction(t);
            }
        } else {
            let mut cjf = CityJSONFeature::from_str(&json_line)?;
            cjj.add_cjfeature(&mut cjf);
        }
    }
    cjj.remove_duplicate_vertices();
    cjj.update_transform();
    cjj.update_geographicalextent();
    io::stdout().write_all(format!("{}\n", serde_json::to_string(&cjj).unwrap()).as_bytes())?;
    Ok(())
}

fn collect_from_files(files: &[String]) -> Result<()> {
    let mut cjj = CityJSON::new();
    let mut cjj_init: bool = false;
    for file in files {
        let path = PathBuf::from(file);
        let f = File::open(path.canonicalize()?)?;
        let br = BufReader::new(f);
        for (i, line) in br.lines().enumerate() {
            match &line {
                Ok(l) => {
                    if i == 0 {
                        if !cjj_init {
                            cjj = CityJSON::from_str(l)?;
                            cjj_init = true;
                        } else {
                            let cjj2 = CityJSON::from_str(l)?;
                            let t: Transform = cjj2.transform;
                            cjj.add_transform_correction(t);
                        }
                    } else {
                        let mut cjf: CityJSONFeature = CityJSONFeature::from_str(l)?;
                        cjj.add_cjfeature(&mut cjf);
                    }
                }
                Err(error) => eprintln!("Error reading line: {}", error),
            }
        }
    }
    cjj.remove_duplicate_vertices();
    cjj.update_transform();
    cjj.update_geographicalextent();
    io::stdout().write_all(format!("{}\n", serde_json::to_string(&cjj).unwrap()).as_bytes())?;
    Ok(())
}

fn cat_from_stdin(order: cjseq::SortingStrategy) -> Result<()> {
    let mut input = String::new();
    match std::io::stdin().read_to_string(&mut input) {
        Ok(_) => {
            let mut cjj: CityJSON = CityJSON::from_str(&input)?;
            cat(&mut cjj, order)?;
        }
        Err(error) => {
            eprintln!("Error: {}", error);
        }
    }
    Ok(())
}

fn cat_from_file(file: &Path, order: cjseq::SortingStrategy) -> Result<()> {
    let f = File::open(file.canonicalize()?)?;
    let mut br = BufReader::new(f);
    let mut json_content = String::new();
    br.read_to_string(&mut json_content)?;
    let mut cjj: CityJSON = CityJSON::from_str(&json_content)?;
    cat(&mut cjj, order)?;
    Ok(())
}

fn cat(cjj: &mut CityJSON, order: cjseq::SortingStrategy) -> Result<()> {
    if cjj.thetype != CityJSONType::CityJSON {
        return Err(CjseqError::Validation(
            "Input file not CityJSON.".to_string(),
        ));
    }
    cjj.sort_cjfeatures(order);
    //-- first line: the CityJSON "metadata"
    let cj1 = cjj.get_metadata()?;
    io::stdout().write_all(format!("{}\n", serde_json::to_string(&cj1).unwrap()).as_bytes())?;
    //-- the other lines for each CityJSONSeq
    let mut i: usize = 0;
    while let Some(cjf) = cjj.get_cjfeature(i)? {
        i += 1;
        io::stdout().write_all(format!("{}\n", serde_json::to_string(&cjf).unwrap()).as_bytes())?;
    }
    Ok(())
}
