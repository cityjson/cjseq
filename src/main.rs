use cjseq2::CityJSON;
use cjseq2::CityJSONFeature;

extern crate clap;
use clap::{Parser, Subcommand, ValueEnum};

use cjseq2::error::{CjseqError, Result};
use rand::Rng;
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

#[derive(Clone, ValueEnum)]
pub enum SortingStrategy {
    Random,
    Alphabetical,
}

#[derive(Subcommand)]
enum Commands {
    /// CityJSON ==> CityJSONSeq
    Cat {
        /// CityJSON input file
        #[arg(short, long)]
        file: Option<PathBuf>,
        /// Sorting for the cat output
        #[arg(short, long, value_enum)]
        order: Option<SortingStrategy>,
    },
    /// CityJSONSeq ==> CityJSON
    Collect {
        /// CityJSONSeq input file
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

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        //-- cat
        Commands::Cat { file, order } => {
            let o2 = match order.clone().unwrap_or(SortingStrategy::Random) {
                SortingStrategy::Random => cjseq2::SortingStrategy::Random,
                SortingStrategy::Alphabetical => cjseq2::SortingStrategy::Alphabetical,
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

fn filter_random(exclude: bool, rand_factor: u32) -> Result<()> {
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

fn filter_cotype(exclude: bool, cotype: String) -> Result<()> {
    let stdin = std::io::stdin();
    for (i, line) in stdin.lock().lines().enumerate() {
        let mut w: bool = false;
        let l = line.unwrap();
        if i == 0 {
            io::stdout().write_all(&format!("{}\n", l).as_bytes())?;
        } else {
            let cjf: CityJSONFeature = CityJSONFeature::from_str(&l)?;
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

fn filter_bbox(exclude: bool, bbox: &Vec<f64>) -> Result<()> {
    let stdin = std::io::stdin();
    let mut cj: CityJSON = CityJSON::new();
    for (i, line) in stdin.lock().lines().enumerate() {
        let mut w: bool = false;
        let l = line.unwrap();
        if i == 0 {
            io::stdout().write_all(&format!("{}\n", l).as_bytes())?;
            cj = CityJSON::from_str(&l)?;
        } else {
            let cjf: CityJSONFeature = CityJSONFeature::from_str(&l)?;
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

fn filter_radius(exclude: bool, x: f64, y: f64, r: f64) -> Result<()> {
    let stdin = std::io::stdin();
    let mut cj: CityJSON = CityJSON::new();
    for (i, line) in stdin.lock().lines().enumerate() {
        let mut w: bool = false;
        let l = line.unwrap();
        if i == 0 {
            io::stdout().write_all(&format!("{}\n", l).as_bytes())?;
            cj = CityJSON::from_str(&l)?;
        } else {
            let cjf: CityJSONFeature = CityJSONFeature::from_str(&l)?;
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

fn collect_from_stdin() -> Result<()> {
    let stdin = std::io::stdin();
    let mut cjj = CityJSON::new();
    for (i, line) in stdin.lock().lines().enumerate() {
        let l = line.unwrap();
        if i == 0 {
            cjj = CityJSON::from_str(&l)?;
        } else {
            let mut cjf = CityJSONFeature::from_str(&l)?;
            cjj.add_cjfeature(&mut cjf);
        }
    }
    cjj.remove_duplicate_vertices();
    cjj.update_transform();
    io::stdout().write_all(&format!("{}\n", serde_json::to_string(&cjj).unwrap()).as_bytes())?;
    Ok(())
}

fn collect_from_file(file: &PathBuf) -> Result<()> {
    let f = File::open(file.canonicalize()?)?;
    let br = BufReader::new(f);
    let mut cjj: CityJSON = CityJSON::new();
    for (i, line) in br.lines().enumerate() {
        match &line {
            Ok(l) => {
                if i == 0 {
                    cjj = CityJSON::from_str(&l)?;
                } else {
                    let mut cjf: CityJSONFeature = CityJSONFeature::from_str(&l)?;
                    cjj.add_cjfeature(&mut cjf);
                }
            }
            Err(error) => eprintln!("Error reading line: {}", error),
        }
    }
    cjj.remove_duplicate_vertices();
    cjj.update_transform();
    io::stdout().write_all(&format!("{}\n", serde_json::to_string(&cjj).unwrap()).as_bytes())?;
    Ok(())
}

fn cat_from_stdin(order: cjseq2::SortingStrategy) -> Result<()> {
    let mut input = String::new();
    match std::io::stdin().read_to_string(&mut input) {
        Ok(_) => {
            let mut cjj: CityJSON = CityJSON::from_str(&input)?;
            let _ = cat(&mut cjj, order)?;
        }
        Err(error) => {
            eprintln!("Error: {}", error);
        }
    }
    Ok(())
}

fn cat_from_file(file: &PathBuf, order: cjseq2::SortingStrategy) -> Result<()> {
    let f = File::open(file.canonicalize()?)?;
    let mut br = BufReader::new(f);
    let mut json_content = String::new();
    br.read_to_string(&mut json_content)?;
    let mut cjj: CityJSON = CityJSON::from_str(&json_content)?;
    cat(&mut cjj, order)?;
    Ok(())
}

fn cat(cjj: &mut CityJSON, order: cjseq2::SortingStrategy) -> Result<()> {
    if cjj.thetype != "CityJSON" {
        return Err(CjseqError::CityJsonError(
            "Input file not CityJSON.".to_string(),
        ));
    }
    if cjj.version != "1.1" && cjj.version != "2.0" {
        return Err(CjseqError::CityJsonError(
            "Input file not CityJSON v1.1 nor v2.0.".to_string(),
        ));
    }
    cjj.sort_cjfeatures(order);
    //-- first line: the CityJSON "metadata"
    let cj1 = cjj.get_metadata();
    io::stdout().write_all(&format!("{}\n", serde_json::to_string(&cj1).unwrap()).as_bytes())?;
    //-- the other lines for each CityJSONSeq
    let mut i: usize = 0;
    while let Some(cjf) = cjj.get_cjfeature(i) {
        i += 1;
        io::stdout()
            .write_all(&format!("{}\n", serde_json::to_string(&cjf).unwrap()).as_bytes())?;
    }
    Ok(())
}
