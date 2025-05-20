
# cjseq

[![crates.io](https://img.shields.io/crates/v/cjseq.svg)](https://crates.io/crates/cjseq)

`cjseq` is a Rust libray+binary for creating, processing, and modifying [CityJSONSeq](https://cityjson.org/cityjsonseq) files, as well as converting to/from [CityJSON](https://cityjson.org).

## Installation

### Installing the binary

1. Install the [Rust compiler](https://www.rust-lang.org/learn/get-started)
2. Run `cargo install cjseq`
3. Then a binary called `cjseq` is installed system-wide

### Installing the library

1. Install the [Rust compiler](https://www.rust-lang.org/learn/get-started)
2. Run `cargo install cjseq`

### Compiling the project

1. Install the [Rust compiler](https://www.rust-lang.org/learn/get-started)
2. Clone the repository: `git clone https://github.com/cityjson/cjseq.git`
3. Build the project: `cargo build --release`
4. Run the program: `./target/release/cjseq --help`

## Usage

`cjseq` can take input from either stdin or a file, and it always outputs the results to stdout. 
The output can be a CityJSON object or a CityJSONSeq stream.

### OBJ Conversion

The library also provides functionality to convert CityJSON or CityJSONSeq files to OBJ format, which can be used for 3D visualization in many software packages.

Here's an example of how to use the OBJ conversion in your Rust code:

```rust
use cjseq::{CityJSON, conv::obj};
use std::fs::File;
use std::io::Read;

fn main() -> std::io::Result<()> {
    // Read a CityJSON file
    let mut file = File::open("your_file.city.json")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    
    // Parse into CityJSON
    let city_json = CityJSON::from_str(&contents).unwrap();
    
    // Convert to OBJ and save to file
    obj::to_obj_file(&city_json, "output.obj")?;
    
    // For CityJSONSeq files, use:
    // obj::jsonseq_file_to_obj("input.city.jsonl", "output.obj")?;
    
    Ok(())
}
```

### Convert CityJSON to CityJSONSeq

Convert a CityJSON file to a CityJSONSeq stream:

```sh
cjseq cat -f myfile.city.json > myfile.city.jsonl
```

Alternatively use stdin:

```sh
cat myfile.city.json | cjseq cat` will output the stream to stdin.
```

### Convert CityJSONSeq to CityJSON

Convert a CityJSONSeq stream to a CityJSON file:

```sh
cat ./data/3dbag_b2.city.jsonl | cjseq collect > 3dbag_b2.city.json
```

### Filter CityJSONSeq

`cat myfile.city.jsonl | cjseq filter --bbox 85007 446179 85168 446290 > mysubset.city.jsonl`

## Input constraints

  1. the input CityJSON/Seq must be v1.1 or v2.0 (v1.0 will panic).
  2. the input JSON must be CityJSON schema-valid, use [cjval](https://github.com/cityjson/cjval) to validate.
