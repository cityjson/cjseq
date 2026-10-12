# Changelog

## [0.5.0] - unreleased
### Modified (breaking)
- The CityJSON schema's structure is carried in the types:
  - geometry boundaries, and semantics, material and texture `values`, are nested exactly as deeply as their geometry type requires; `NestedArray` and the untagged value enums are gone, and each `Geometry` variant names its own value shapes (`PrimitiveValues`, `SolidValues`, `MultiSolidValues`);
  - one index type per kind of index (`VertexIndex`, `UvIndex`, `MaterialIndex`, `TextureIndex`, `SurfaceIndex`, `TemplateIndex`);
  - vertices are `[i64; 3]`, `transform` scale/translate `[f64; 3]`, `vertices-templates` `[f64; 3]`, a GeometryInstance's boundaries `[VertexIndex; 1]`, texture vertices `[f64; 2]`, extents `[f64; 6]`;
  - `Lod` holds one of the schema's twenty LoD values and is required on every primitive; `version` is a `CityJSONVersion` (1.1 or 2.0); `attributes` is a JSON object; `extensions` maps a name to an `Extension { url, version }`; `contactType` and `role` are enums; a texture ring is `TexturedRing::Untextured` or `Textured { texture, uvs }`;
  - City Object and semantic surface types are enums with an `Extension` case, which must start with `+`.
- Objects the schema closes (geometries, `transform`, `appearance`, `geometry-templates`, materials, textures) reject members it does not name; every other object keeps them in an `other: serde_json::Map`.
- `CityObjects` and the material/texture theme maps are `IndexMap`s, and object member order is preserved, so output follows input order and is deterministic.
- The document model and CityJSONSeq sequencing are split. `CityJSON` holds only the document; `header()` (was `get_metadata`) and `features(order)` slice it into a sequence, `feature(id)` slices one feature, and `Collector` (`new`, `add`, `add_header`, `finish`) replaces `add_cjfeature`/`add_transform_correction`. `SeqReader`/`SeqWriter` read and write a sequence line by line, as `Line::Header`/`Line::Feature`. `get_cjfeature`, `sort_cjfeatures` and the hidden sorting state are gone; `cjseq_to_cj` returns a `Result`.
- `SortingStrategy` is `AsIs` (input order, now the CLI default), `Random { seed }` (a shuffle reproducible from its seed; `cjseq cat --seed`) or `Lexicographical`; the unimplemented `Morton`/`Hilbert` are gone.
- A City Object no feature reaches (its parent is missing, or does not list it as a child) is a `NotInAnyFeature` error from `features()` instead of being left out of `cat`'s output.
- Warnings are a `Warning` type of their own, not `CjseqError` variants.
- The public enums are `#[non_exhaustive]` (except `Line`).
- `clap`, `rand` and `getrandom` are optional, behind a default `cli` feature that the binary needs; the library depends only on `serde`, `serde_json` and `indexmap`. `rust-version` is 1.85.
- Errors are a `CjseqError` with structured variants (`IndexOutOfRange`, `DanglingReference`, `HierarchyMismatch`, `ValueCount`, `MisplacedMember`, located by `InCityObject`, `InTemplate`, `AtLine`); `Result<T>` lives at `cjseq::error::Result`. Malformed input is an error, never a panic.
- The library is split into per-concern modules; every type is re-exported at the crate root. `CityJSON` and `CityJSONFeature` implement `FromStr` and `Default`.
- wasm-bindgen bindings are behind an off-by-default `wasm` cargo feature, so library consumers building for wasm32 no longer inherit them.
### Added
- `validate()`/`errors()` on `CityJSON` and `CityJSONFeature`: every vertex, template, semantic surface, material, texture and texture-vertex index is in range, value arrays match the boundaries they describe, the parent/child hierarchy agrees with itself, and a feature carries no document-only members. The CLI has a `validate` subcommand.
- `validate()` also checks the schemas' Extension name patterns: `+` then a capital letter for a City Object type, `+` then word characters for a semantic surface type, and `X.Y` or `X.Y.Z` for an Extension's version (`CjseqError::Pattern`).
- `warnings()` on `CityJSON` and `CityJSONFeature` reports what the spec advises against but a valid file may still do: a core City Object with an Extension child (§ 8.7 rule 3), which the Noise Extension example published with the spec does. `cjseq validate` prints warnings without failing.
- `Collector::warnings()`: CityJSONFeature lines may carry root members the schema does not name, and collecting drops them, one `DroppedMember` warning each; `cjseq collect` prints them to stderr.
- `Display` for `CityJSONVersion` (`1.1`, `2.0`).
- Features keep every descendant of their top-level City Object, not only its children.
- `CityObject::new`, `children_roles`, `Geometry::surfaces`, `PartialEq` on the public types.
- Round-trip, validation and sequencing tests over real CityJSONSeq fixtures; CI runs fmt, clippy with warnings denied, the tests and the wasm build.
### Fixed
- `collect` corrupted the texture coordinates of every textured feature after the first.
- Address `location`s (Building, BuildingPart, BuildingUnit, Bridge, BridgePart) were never renumbered: `cat` and `collect` left them pointing at the wrong vertices. They are now a typed `CityObjectAddress`, renumbered and validated like any geometry.
- `cat | collect` lost `default-theme-material`/`-texture` when the document had no geometry templates.
- Collecting several sequences kept only a later header's transform; its geometry templates (and their appearance), extensions and root members are now merged, with GeometryInstances shifted to match, and conflicts reported.
- `collect` turned an index past the end of a feature's own vertices or palettes into a valid-looking index into another feature's data; it is now an error. A City Object met in more than one feature (a member of two CityObjectGroups travels in both groups' features) is collected once when both copies are identical, and is a `DuplicateId` error instead of a silent overwrite when they are not; equal vertices and texture vertices are merged as features are added, so identical copies compare equal.
- The header written by `cat` cut the appearance down to what the geometry templates use but left the templates' indices unrenumbered, so they pointed at the wrong materials and textures.
- Metadata, address, `pointOfContact`, semantics, appearance and feature members the schema permits are kept instead of dropped or rejected; every `referenceSystem` the schema's URL pattern permits is accepted.
- Coordinates round-trip exactly (`serde_json`'s `float_roundtrip`).
- An empty document no longer gets a nonsense `translate` from `collect`, and an empty feature no longer has a NaN centroid.
### Performance
Against 0.4.0, release builds, median of 3 runs on one machine:
- a 65 MB, 15,000-feature CityJSONSeq (`tests/data/small.city.jsonl` repeated): `collect` 0.95 s to 0.47 s, `cat` 0.57 s to 0.44 s, `filter --radius` 0.40 s to 0.25 s; `collect` gives the same objects, vertices, transform and real coordinates;
- 80,000 features with one distinct material each: `collect` 68.3 s to 0.23 s, as materials and textures are merged through a hash map instead of a linear scan.

## [0.4.0] - 2025-09-12
### Modified
- cjseq is now a library + has one binary. Some the functionalities that were in the `main.rs` are now in the `lib.rs` and the binary basically just adds functions to filter and calls the functions of `lib.rs`. This was done also to easily create a Python package.
- it's now possible to control how the features are ordered in a CityJSONSeq output: random (default) or lexicographical (Morton + Hilbert sorting planned for the future)
- collect sub-command works with globbing (so multiple files can be input)
- WASM compilation added

## [0.3.1] - 2024-06-17
### Modified
- better readme + better help for subcommands
- Add retransform ops when collecting. For large dataset if subset is small area the translate is too far away. This scales the dataset better and smaller coordinates too, viewers will be centered on the data too.

## [0.3.0] - 2024-04-18
### Modified
- changed many names 
### Added
- new subcommand: filter

## [0.2.0] - 2024-02-09
### Modified
- fix bug with wrong indexing with BuildingPart and collect
### Added
- remove_duplicate_vertices() added for collect


## [0.1.0] - 2024-02-07
### Added
- first release in draft
- no unit test yet, except me testing manually with some files
- everything (incl. Geometry Templates) are supported
