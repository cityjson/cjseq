//! `cjseq2` is a typed fork of [`cjseq`](https://github.com/cityjson/cjseq),
//! for reading, writing, and converting between CityJSON and CityJSONSeq.
//!
//! The library crate is still named `cjseq` (see `Cargo.toml`'s `[lib]`
//! section), so existing code that does `use cjseq::CityJSON` keeps working
//! unchanged. The package itself is published as `cjseq2` pending an
//! eventual merge upstream.
//!
//! Internally the crate is organised into one module per concern:
//! - `geometry` — `Geometry`, `GeometryType`, `GeometryTemplates`
//! - `semantics` — `Semantics`, `SemanticsSurface`, `SemanticsValues`
//! - `appearance` — `Appearance`, `MaterialReference`, `MaterialValues`, `TextureReference`, `TextureValues`
//! - `city_object` — `CityObject`
//! - `metadata` — `Metadata`, `PointOfContact`, `Address`, `ReferenceSystem`, `Transform`, `GeographicalExtent`
//! - `error` — `CjseqError`, `Result`
//! - `cityjson` — `CityJSON`, `CityJSONFeature`, `SortingStrategy`, and the sequencing/collect/filter logic
//!
//! Every type is re-exported at the crate root, so the module boundaries
//! above are an internal organisational detail, not part of the public
//! import paths. The one exception is the `Result<T>` alias, which stays at
//! [`error::Result`] so that a glob `use cjseq::*` cannot shadow
//! `std::result::Result`.

mod appearance;
mod city_object;
mod cityjson;
pub mod error;
mod geometry;
mod metadata;
mod semantics;

pub use appearance::*;
pub use city_object::*;
pub use cityjson::*;
//-- Deliberately *not* `pub use error::*`. That would put a `Result<T>` alias
//-- at the crate root, and `use cjseq::*` would then shadow
//-- `std::result::Result` in the importing module -- a surprise this crate has
//-- no business inflicting on its callers. `CjseqError` is re-exported by
//-- name; the alias stays reachable as `cjseq::error::Result`.
pub use error::CjseqError;
pub use geometry::*;
pub use metadata::*;
pub use semantics::*;

// WASM bindings module
#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
pub mod wasm;

// Re-export WASM functions for convenience
#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
pub use wasm::*;
