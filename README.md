
# cjseq

[![crates.io](https://img.shields.io/crates/v/cjseq.svg)](https://crates.io/crates/cjseq)

`cjseq` is a program for creating, processing, and modifying [CityJSONSeq](https://cityjson.org/cityjsonseq) files, as well as converting [CityJSON](https://cityjson.org) files to it.


## Installation

### Installing the binaries

1. Install the [Rust compiler](https://www.rust-lang.org/learn/get-started).
2. Run `cargo install cjseq`.

### Compiling the project

1. Install the [Rust compiler](https://www.rust-lang.org/learn/get-started).
2. Clone the repository:
    ```sh
    git clone https://github.com/cityjson/cjseq.git
    ```
3. Build the project:
    ```sh
    cargo build --release
    ```
4. Run the program:
    ```sh
    ./target/release/cjseq --help
    ```

## Usage

`cjseq` can take input from either stdin or a file, and it always outputs the results to stdout. 
The output can be a CityJSON object or a CityJSONSeq stream.

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
  2. the input JSON but be CityJSON schema-valid, use [cjval](https://github.com/cityjson/cjval) to validate.
