# Changelog

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
