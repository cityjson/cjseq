//! One type per kind of index, so an index into one array cannot be used as
//! an index into another. In JSON each is a plain non-negative integer.

use crate::error::IndexKind;
use serde::{Deserialize, Serialize};
use std::hash::Hash;

/// What every index type can do: be built from and turned back into a
/// position, and say which array it indexes.
pub trait Index: Copy + Eq + Hash + std::fmt::Debug {
    /// The array this kind of index points into.
    const KIND: IndexKind;
    fn new(i: usize) -> Self;
    fn get(self) -> usize;
}

macro_rules! index_type {
    ($(#[$doc:meta])* $name:ident, $kind:ident) => {
        $(#[$doc])*
        #[derive(
            Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord,
        )]
        #[serde(transparent)]
        pub struct $name(pub usize);

        impl Index for $name {
            const KIND: IndexKind = IndexKind::$kind;
            fn new(i: usize) -> Self {
                $name(i)
            }
            fn get(self) -> usize {
                self.0
            }
        }
    };
}

index_type!(
    /// A position in `vertices` (or, inside a geometry template, in
    /// `vertices-templates`).
    VertexIndex,
    Vertex
);
index_type!(
    /// A position in `appearance.vertices-texture`.
    UvIndex,
    TextureVertex
);
index_type!(
    /// A position in `appearance.materials`.
    MaterialIndex,
    Material
);
index_type!(
    /// A position in `appearance.textures`.
    TextureIndex,
    Texture
);
index_type!(
    /// A position in a geometry's `semantics.surfaces`.
    SurfaceIndex,
    Surface
);
index_type!(
    /// A position in `geometry-templates.templates`.
    TemplateIndex,
    Template
);

/// Renumber `i` in first-encounter order: an index already in `map` keeps
/// its mapping, a new one gets the next free position.
pub(crate) fn remap<I: Index>(map: &mut std::collections::HashMap<I, I>, i: I) -> I {
    let next = I::new(map.len());
    *map.entry(i).or_insert(next)
}
