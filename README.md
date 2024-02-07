
# cjseq: conversion CityJSON <-> CityJSONSeq

[![crates.io](https://img.shields.io/crates/v/cjseq.svg)](https://crates.io/crates/cjseq)

A program to convert [CityJSON](https://cityjson.org) datasets to [CityJSONSeq](https://cityjson.org/cityjsonseq), and vice-versa.

## Installation/compilation

### To install the binaries on your system easily

1. install the [Rust compiler](https://www.rust-lang.org/learn/get-started)
2. `cargo install cjseq`

### To compile the project (and eventually modify it)

1. install the [Rust compiler](https://www.rust-lang.org/learn/get-started)
2. `git clone https://github.com/cityjson/cjseq.git`
3. `cargo build --release` (this will ensure the binaries are compiled too)
4. `./target/release/cjseq --help`

## Usage

`cjseq` can either take its input from stdin, or from a file.
It always outputs to stdout the results (either a CityJSON object or a CityJSONSeq stream).

### CityJSON => CityJSONSeq

`cjseq cat -f myfile.city.json > myfile.city.jsonl` will convert the file `myfile.city.json` to a CityJSONSeq stream and write it to the file `myfile.city.jsonl`.

`cat myfile.city.json | cjseq cat` will output the stream to stdin.

### CityJSONSeq => CityJSON

`cat ./data/3dbag_b2.city.jsons cjseq collect > 3dbag_b2.city.json` 


## Input constraints

  1. the input CityJSON/Seq must be v1.1 or v2.0 (v1.0 will panic)
  2. the input JSON but be CityJSON schema-valid, use [cjval](https://github.com/cityjson/cjval)

