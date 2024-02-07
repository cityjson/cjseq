
# cjseq: for converting CityJSON <-> CityJSONSeq


## To compile

```
cargo build --release
```

## Usage

`cjseq cat` will convert a CityJSON to a CityJSONSeq

`cjseq collect` will convert a CityJSONSeq to a CityJSON

Output is always stdout, to create a file

```
cjseq cat -f delft.city.json > delft.city.jsonl
```

## Input requisites

  1. the input CityJSON/Seq must be v1.1 or v2.0 (v1.0 will panic)
  2. the input JSON but be CityJSON schema-valid, use [cjval](https://github.com/cityjson/cjval)

