
# cjseq

[![crates.io](https://img.shields.io/crates/v/cjseq.svg)](https://crates.io/crates/cjseq)

`cjseq` is a Rust library and binary for creating, processing, and modifying [CityJSONSeq](https://cityjson.org/cityjsonseq) files, as well as converting to and from [CityJSON](https://cityjson.org).

Its types follow the CityJSON 2.0 schema: a geometry's boundaries and its semantics, material and texture values are nested exactly as deeply as its type requires, each kind of index has its own type, and strings the schema constrains (LoD, City Object types, versions) are typed. What the types cannot express, `validate()` checks.

## Installation

### Installing the binary

1. Install the [Rust compiler](https://www.rust-lang.org/learn/get-started)
2. Run `cargo install cjseq`
3. Then a binary called `cjseq` is installed system-wide

### Using the library

```sh
cargo add cjseq
```

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

By default the features are written in the order of the input; `--order lexicographical` sorts them by id, `--order random` shuffles them.

### Convert CityJSONSeq to CityJSON

The operator "collect" converts a CityJSONSeq stream to a CityJSON file:

```sh
cat ./data/3dbag_b2.city.jsonl | cjseq collect > 3dbag_b2.city.json
```

```sh
cjseq collect ./data/3dbag_b2.city.jsonl > 3dbag_b2.city.json
```

Several sequences can be collected into one file, with a file list or [globbing](https://en.wikipedia.org/wiki/Glob_(programming)):

```sh
cjseq collect ./data/*.city.jsonl > hugefile.city.json
```

When their transforms differ, the features of each sequence are re-expressed in the first one's transform.

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

### Validate

`validate` reads a CityJSON file or a CityJSONSeq and prints every reference that does not resolve: indices past the end of `vertices` or of an appearance palette, semantics or material values that do not match their geometry, children and parents that do not name each other back, and so on.

```sh
cjseq validate myfile.city.jsonl
```

It also prints warnings, without failing, for what the spec advises against but a valid file may still do: a core City Object with an Extension City Object as a child (§ 8.7 rule 3).

It complements, and does not replace, schema validation with [cjval](https://github.com/cityjson/cjval).

## Input constraints

  1. the input must be CityJSON v1.1 or v2.0; anything else is reported as an error.
  2. input the schema forbids (a geometry nested to the wrong depth, a member a closed object does not name, a misspelt type) is reported as an error, with the line it is on for a CityJSONSeq.

## The library

```rust
use cjseq::{CityJSON, Collector, Line, SeqReader, SeqWriter, SortingStrategy};
use std::io::BufReader;

fn main() -> cjseq::error::Result<()> {
    //-- CityJSON -> CityJSONSeq
    let cj: CityJSON = std::fs::read_to_string("data/3dbag_b2.city.json")?.parse()?;
    cj.validate()?;
    let mut out = SeqWriter::new(Vec::new());
    out.write_header(&cj.header()?)?;
    for feature in cj.features(SortingStrategy::AsIs) {
        out.write_feature(&feature?)?;
    }

    //-- CityJSONSeq -> CityJSON
    let file = std::fs::File::open("data/3dbag_b2.city.jsonl")?;
    let mut collector: Option<Collector> = None;
    for line in SeqReader::new(BufReader::new(file)) {
        match line? {
            Line::Header(h) => collector = Some(Collector::new(h)),
            Line::Feature(f) => collector.as_mut().expect("header first").add(f)?,
        }
    }
    let cj = collector.expect("a header").finish();
    println!("{} features", cj.number_of_city_objects());
    Ok(())
}
```

## WASM bindings

`cjseq` can be used in JavaScript/TypeScript applications via WASM bindings.

```sh
cargo install wasm-pack
```

Build the WASM bindings, which are behind the `wasm` cargo feature:

```sh
wasm-pack build --release --target web --out-dir js -- --features wasm
```
