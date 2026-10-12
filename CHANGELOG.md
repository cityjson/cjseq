# Changelog

## [0.5.0] - unreleased
### Modified (breaking)
- The library is split into per-concern modules (`geometry`, `semantics`, `appearance`, `city_object`, `metadata`, `error`, `cityjson`); every type is still re-exported at the crate root.
- Geometry boundaries, semantics values and appearance values carry their nesting depth in their type; `NestedArray` is removed. Null sub-arrays are accepted where the CityJSON schema allows them, and an explicit `null` material `values` stays distinct from an absent one.
- CityObject and semantic-surface types are typed enums, with support for extension types (`+Name`).
- Materials and textures in the appearance library are typed structs; members the schema does not name are rejected.
- `CityObject.geographicalExtent` is `[f64; 6]`, texture vertices are `[f64; 2]`.
- Errors are a `CjseqError` type; `Result<T>` lives at `cjseq::error::Result` and is no longer re-exported at the root.
- wasm-bindgen bindings are behind an off-by-default `wasm` cargo feature, so library consumers building for wasm32 no longer inherit them.
### Added
- `CityObject::new`, `children_roles` on `CityObject`, `PartialEq`/`Eq` on the public structs.
- Round-trip tests over real CityJSONSeq fixtures.
### Fixed
- Metadata, address, `pointOfContact`, semantics and appearance members that the schema permits are kept instead of dropped or rejected.
- Every `referenceSystem` the schema's URL pattern permits is accepted.

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
