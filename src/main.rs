use cjseq::error::Result;
use cjseq::{
    CityJSON, CityObjectType, CjseqError, Collector, Line, SeqReader, SeqWriter, Transform,
};
use clap::{Parser, Subcommand, ValueEnum};
use rand::Rng;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
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
        /// Seed for `--order random`, to reproduce an order; random if absent
        #[arg(long)]
        seed: Option<u64>,
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
    /// Check a CityJSON or CityJSONSeq file for references that do not
    /// resolve; prints every problem found
    Validate {
        /// CityJSON (.json) or CityJSONSeq (.jsonl) input file; stdin if absent
        file: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();
    let result = match &cli.command {
        Commands::Cat { file, order, seed } => {
            let order = match order.clone().unwrap_or(SortingStrategy::AsIs) {
                SortingStrategy::AsIs => cjseq::SortingStrategy::AsIs,
                SortingStrategy::Random => cjseq::SortingStrategy::Random {
                    seed: seed.unwrap_or_else(rand::random),
                },
                SortingStrategy::Lexicographical => cjseq::SortingStrategy::Lexicographical,
            };
            read_all(file.as_deref()).and_then(|s| cat(&s, order))
        }
        Commands::Collect { files } => collect(files.as_deref().unwrap_or_default()),
        Commands::Filter {
            bbox,
            cotype,
            exclude,
            radius,
            random,
        } => {
            let keep = if let Some(b) = bbox {
                Filter::Bbox([b[0], b[1], b[2], b[3]])
            } else if let Some(c) = cotype {
                //-- parsed through the same type the City Objects carry, so
                //-- the comparison is enum to enum
                match serde_json::from_value::<CityObjectType>(serde_json::json!(c)) {
                    Ok(t) => Filter::CoType(t),
                    Err(e) => exit_with(e.into()),
                }
            } else if let Some(r) = radius {
                Filter::Radius(r[0], r[1], r[2])
            } else {
                Filter::Random(random.unwrap_or(1))
            };
            filter(&keep, *exclude)
        }
        Commands::Validate { file } => read_all(file.as_deref()).and_then(|s| validate(&s)),
    };
    if let Err(e) = result {
        exit_with(e);
    }
}

fn exit_with(e: CjseqError) -> ! {
    eprintln!("{e}");
    std::process::exit(1);
}

fn read_all(file: Option<&Path>) -> Result<String> {
    let mut s = String::new();
    match file {
        Some(p) => {
            File::open(p)?.read_to_string(&mut s)?;
        }
        None => {
            io::stdin().read_to_string(&mut s)?;
        }
    }
    Ok(s)
}

fn cat(input: &str, order: cjseq::SortingStrategy) -> Result<()> {
    let cj: CityJSON = input.parse()?;
    let header = cj.header()?;
    let mut features = cj.features(order).peekable();
    //-- a document that cannot be sliced fails before anything is written
    if let Some(Err(_)) = features.peek() {
        return Err(features.next().unwrap().unwrap_err());
    }
    let mut w = SeqWriter::new(BufWriter::new(io::stdout().lock()));
    w.write_header(&header)?;
    for f in features {
        w.write_feature(&f?)?;
    }
    w.into_inner().flush()?;
    Ok(())
}

/// Collect every line of every sequence into one document: the first header
/// starts it, later headers set the transform of the features after them.
fn collect(files: &[String]) -> Result<()> {
    let mut collector: Option<Collector> = None;
    let mut read = |r: &mut dyn BufRead| -> Result<()> {
        for line in SeqReader::new(r) {
            match (line?, collector.as_mut()) {
                (Line::Header(h), None) => collector = Some(Collector::new(h)),
                (Line::Header(h), Some(c)) => c.add_header(h)?,
                (Line::Feature(f), Some(c)) => c.add(f)?,
                (Line::Feature(_), None) => {
                    return Err(CjseqError::Validation(
                        "a CityJSONFeature came before any CityJSON header line".to_string(),
                    ))
                }
            }
        }
        Ok(())
    };
    if files.is_empty() {
        read(&mut io::stdin().lock())?;
    } else {
        for file in files {
            read(&mut BufReader::new(File::open(file)?))?;
        }
    }
    for w in collector.iter().flat_map(Collector::warnings) {
        eprintln!("warning: {w}");
    }
    let cj = collector.map(Collector::finish).unwrap_or_default();
    let mut out = BufWriter::new(io::stdout().lock());
    serde_json::to_writer(&mut out, &cj)?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(())
}

enum Filter {
    Bbox([f64; 4]),
    CoType(CityObjectType),
    Radius(f64, f64, f64),
    /// Keep 1 in X
    Random(u32),
}

/// Copy the sequence on stdin to stdout, keeping the header lines and the
/// features `keep` selects (or, with `exclude`, the others). Lines are copied
/// as they are, not re-serialized.
fn filter(keep: &Filter, exclude: bool) -> Result<()> {
    let mut rng = rand::thread_rng();
    let mut transform = Transform::new();
    let mut out = BufWriter::new(io::stdout().lock());
    for (i, text) in io::stdin().lock().lines().enumerate() {
        let text = text?;
        if text.trim().is_empty() {
            continue;
        }
        let at = |e: CjseqError| CjseqError::AtLine {
            line: i + 1,
            source: Box::new(e),
        };
        let selected = match text.parse::<Line>().map_err(at)? {
            Line::Header(h) => {
                transform = h.transform;
                writeln!(out, "{text}")?;
                continue;
            }
            Line::Feature(f) => {
                //-- the real-world centre, or None for a feature with no
                //-- vertices, which is nowhere and so never inside
                let centre = f
                    .centroid()
                    .map(|c| [0, 1].map(|k| c[k] * transform.scale[k] + transform.translate[k]));
                match keep {
                    Filter::Bbox([minx, miny, maxx, maxy]) => centre
                        .is_some_and(|[x, y]| x > *minx && x < *maxx && y > *miny && y < *maxy),
                    Filter::Radius(cx, cy, r) => {
                        centre.is_some_and(|[x, y]| (x - cx).powi(2) + (y - cy).powi(2) <= r * r)
                    }
                    Filter::CoType(t) => match f.city_objects.get(&f.id) {
                        Some(co) => co.thetype == *t,
                        None => {
                            return Err(at(CjseqError::DanglingReference {
                                from: "CityJSONFeature.id".to_string(),
                                to: f.id,
                            }))
                        }
                    },
                    Filter::Random(x) => rng.gen_range(1..=*x) == 1,
                }
            }
        };
        if selected != exclude {
            writeln!(out, "{text}")?;
        }
    }
    out.flush()?;
    Ok(())
}

/// Print every problem in a CityJSON document or a CityJSONSeq, and every
/// warning; fail if there is a problem.
fn validate(input: &str) -> Result<()> {
    let mut problems: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    match input.parse::<CityJSON>() {
        Ok(cj) => {
            problems.extend(cj.errors().iter().map(ToString::to_string));
            warnings.extend(cj.warnings().iter().map(ToString::to_string));
        }
        _ => {
            for line in SeqReader::new(input.as_bytes()) {
                match line {
                    Err(e) => problems.push(e.to_string()),
                    Ok(Line::Header(h)) => {
                        problems.extend(h.errors().iter().map(|e| format!("header: {e}")));
                        warnings.extend(h.warnings().iter().map(|e| format!("header: {e}")));
                    }
                    Ok(Line::Feature(f)) => {
                        let at = |e: &dyn std::fmt::Display| format!("feature {:?}: {e}", f.id);
                        problems.extend(f.errors().iter().map(|e| at(e)));
                        warnings.extend(f.warnings().iter().map(|w| at(w)));
                    }
                }
            }
        }
    }
    for w in &warnings {
        println!("warning: {w}");
    }
    for p in &problems {
        println!("{p}");
    }
    match problems.len() {
        0 => {
            println!("valid");
            Ok(())
        }
        n => Err(CjseqError::Validation(format!("{n} problem(s) found"))),
    }
}
