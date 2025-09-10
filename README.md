
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

`cjseq` takes input from either a file or the standard input (stdin, if no file path is given as argument), and it always outputs the results to the standard output (stdout). 
The output can be a CityJSON object or a CityJSONSeq stream.

### Convert CityJSON to CityJSONSeq

The operator "cat" converts a CityJSON file to a CityJSONSeq stream:

```sh
cjseq cat myfile.city.json > myfile.city.jsonl
```

Alternatively, to use stdin as input:

```sh
cat myfile.city.json | cjseq cat
```

### Convert CityJSONSeq to CityJSON

The operator "collect" converts a CityJSONSeq stream to a CityJSON file:

```sh
cat ./data/3dbag_b2.city.jsonl | cjseq collect > 3dbag_b2.city.json
```

```sh
cjseq collect ./data/3dbag_b2.city.jsonl > 3dbag_b2.city.json
```

Notice that [globbing](https://en.wikipedia.org/wiki/Glob_(programming)) works for the `collect` command:

```sh
cat ./data/*.city.jsonl | cjseq collect > hugefile.city.json
```

### Filter CityJSONSeq

An input stream of CityJSONSeq can be filtered with the following operators:

```sh
--bbox <minx> <miny> <maxx> <maxy>
          Bounding box filter
--cotype <COTYPE>
    Keep only the CityObjects of this type
--exclude
    Excludes the selection, thus remove the selected city object(s)
--radius <x> <y> <radius>
    Circle filter: centre + radius
--random <X>
    1/X chances of a given feature being kept
```

As an example:

```sh
cat myfile.city.jsonl | cjseq filter --bbox 85007 446179 85168 446290 > mysubset.city.jsonl
```

## Input constraints

  1. the input CityJSON/Seq must be v1.1 or v2.0 (v1.0 will panic).
  2. the input JSON must be CityJSON schema-valid, use [cjval](https://github.com/cityjson/cjval) to validate.
