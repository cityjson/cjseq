//! `cjseq` reads, writes, and converts between CityJSON and CityJSONSeq,
//! with the CityJSON schema's structure carried in its types.
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
mod index;
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
pub use error::{CjseqError, IndexKind};
pub use geometry::*;
pub use index::{
    Index, MaterialIndex, SurfaceIndex, TemplateIndex, TextureIndex, UvIndex, VertexIndex,
};
pub use metadata::*;
pub use semantics::*;

// WASM bindings module
#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
pub mod wasm;

// Re-export WASM functions for convenience
#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
pub use wasm::*;
