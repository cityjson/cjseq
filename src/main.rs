use serde_json::{Result, Value};
use std::path::PathBuf;

use clap::{arg, command, value_parser, Command};

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
            println!("'cat' was used");
            let re = cat(sub_matches.get_one::<PathBuf>("file"));
            println!("{:?}", re);
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

fn cat(file: Option<&PathBuf>) -> Result<()> {
    if file.is_none() {
        println!("stdin");
    } else {
        if file.unwrap().is_file() == false {
            println!("no file?!");
        } else {
            println!("=={:?}", file.unwrap().canonicalize().unwrap());
            let s1 = std::fs::read_to_string(file.unwrap()).expect("Couldn't read CityJSON file");
            let v: Value = serde_json::from_str(&s1)?;
            println!("{:?}", v);
        }
    }

    Ok(())
}
