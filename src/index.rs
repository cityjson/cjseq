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

        impl Indices<$name> for $name {
            fn for_each_index(&mut self, f: &mut dyn FnMut(&mut $name)) {
                f(self)
            }
            fn visit_index(&self, f: &mut dyn FnMut($name)) {
                f(*self)
            }
        }

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

/// Semantics or material values of a MultiPoint, MultiLineString,
/// MultiSurface or CompositeSurface: one index per point, line or surface.
/// `None` is `null`: that primitive has none.
pub type PrimitiveValues<I> = Vec<Option<I>>;
/// Semantics or material values of a Solid: per shell, per surface. A whole
/// shell may be `null`.
pub type SolidValues<I> = Vec<Option<PrimitiveValues<I>>>;
/// Semantics or material values of a MultiSolid or CompositeSolid: per
/// solid, per shell, per surface. A whole solid may be `null`.
pub type MultiSolidValues<I> = Vec<Option<SolidValues<I>>>;

/// Visit every index of kind `I` held somewhere inside a value, in document
/// order. Implemented for each index type itself and for the containers
/// that nest them, so a value of any depth is walked the same way.
pub(crate) trait Indices<I> {
    fn for_each_index(&mut self, f: &mut dyn FnMut(&mut I));
    fn visit_index(&self, f: &mut dyn FnMut(I));
}

impl<I, T: Indices<I>> Indices<I> for Vec<T> {
    fn for_each_index(&mut self, f: &mut dyn FnMut(&mut I)) {
        for t in self {
            t.for_each_index(f);
        }
    }
    fn visit_index(&self, f: &mut dyn FnMut(I)) {
        for t in self {
            t.visit_index(f);
        }
    }
}

impl<I, T: Indices<I>, const N: usize> Indices<I> for [T; N] {
    fn for_each_index(&mut self, f: &mut dyn FnMut(&mut I)) {
        for t in self {
            t.for_each_index(f);
        }
    }
    fn visit_index(&self, f: &mut dyn FnMut(I)) {
        for t in self {
            t.visit_index(f);
        }
    }
}

impl<I, T: Indices<I>> Indices<I> for Option<T> {
    fn for_each_index(&mut self, f: &mut dyn FnMut(&mut I)) {
        if let Some(t) = self {
            t.for_each_index(f);
        }
    }
    fn visit_index(&self, f: &mut dyn FnMut(I)) {
        if let Some(t) = self {
            t.visit_index(f);
        }
    }
}

impl<I, T: Indices<I>> Indices<I> for indexmap::IndexMap<String, T> {
    fn for_each_index(&mut self, f: &mut dyn FnMut(&mut I)) {
        for t in self.values_mut() {
            t.for_each_index(f);
        }
    }
    fn visit_index(&self, f: &mut dyn FnMut(I)) {
        for t in self.values() {
            t.visit_index(f);
        }
    }
}

/// Renumber `i` in first-encounter order: an index already in `map` keeps
/// its mapping, a new one gets the next free position.
pub(crate) fn remap<I: Index>(map: &mut std::collections::HashMap<I, I>, i: I) -> I {
    let next = I::new(map.len());
    *map.entry(i).or_insert(next)
}
