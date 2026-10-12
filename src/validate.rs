//! The checks serde cannot express: every index is in range, every value
//! array has as many entries as the geometry it describes, and the City
//! Object hierarchy agrees with itself.

use crate::appearance::{Appearance, MaterialReference, TextureReference, TexturedRing};
use crate::city_object::CityObject;
use crate::cityjson::{CityJSON, CityJSONFeature};
use crate::error::{CjseqError, Result};
use crate::geometry::{Geometry, Shell, Surface, Themes};
use crate::index::{
    Index, Indices, MaterialIndex, MultiSolidValues, PrimitiveValues, SolidValues, SurfaceIndex,
    TextureIndex, UvIndex,
};
use crate::semantics::Semantics;
use indexmap::IndexMap;

/// The lengths that the indices of one geometry are checked against.
struct Lengths {
    vertices: usize,
    /// `None` where templates are not known, as in a lone CityJSONFeature.
    templates: Option<usize>,
    materials: usize,
    textures: usize,
    uvs: usize,
}

impl Lengths {
    fn new(vertices: usize, templates: Option<usize>, appearance: Option<&Appearance>) -> Self {
        let len = |v: Option<usize>| v.unwrap_or(0);
        Lengths {
            vertices,
            templates,
            materials: len(appearance.and_then(|a| a.materials.as_ref().map(Vec::len))),
            textures: len(appearance.and_then(|a| a.textures.as_ref().map(Vec::len))),
            uvs: len(appearance.and_then(|a| a.vertices_texture.as_ref().map(Vec::len))),
        }
    }
}

fn in_range<I: Index>(i: I, len: usize, errs: &mut Vec<CjseqError>) {
    if i.get() >= len {
        errs.push(CjseqError::IndexOutOfRange {
            kind: I::KIND,
            index: i.get(),
            len,
        });
    }
}

fn count(what: &'static str, expected: usize, found: usize, errs: &mut Vec<CjseqError>) {
    if expected != found {
        errs.push(CjseqError::ValueCount {
            what,
            expected,
            found,
        });
    }
}

/// How a value array lines up with the boundaries it describes.
trait Shape<B: ?Sized> {
    fn check_shape(&self, what: &'static str, boundaries: &B, errs: &mut Vec<CjseqError>);
}

//-- one impl per index type rather than generic over it: generic impls for
//-- `Vec<Option<I>>` and `Vec<Option<Vec<Option<I>>>>` would overlap
macro_rules! value_shapes {
    ($($i:ty),*) => {$(
        impl<B> Shape<[B]> for PrimitiveValues<$i> {
            fn check_shape(&self, what: &'static str, b: &[B], errs: &mut Vec<CjseqError>) {
                count(what, b.len(), self.len(), errs);
            }
        }

        impl Shape<[Shell]> for SolidValues<$i> {
            fn check_shape(&self, what: &'static str, b: &[Shell], errs: &mut Vec<CjseqError>) {
                count(what, b.len(), self.len(), errs);
                for (v, shell) in self.iter().zip(b) {
                    if let Some(v) = v {
                        count(what, shell.len(), v.len(), errs);
                    }
                }
            }
        }

        impl Shape<[Vec<Shell>]> for MultiSolidValues<$i> {
            fn check_shape(&self, what: &'static str, b: &[Vec<Shell>], errs: &mut Vec<CjseqError>) {
                count(what, b.len(), self.len(), errs);
                for (v, solid) in self.iter().zip(b) {
                    if let Some(v) = v {
                        v.check_shape(what, &solid[..], errs);
                    }
                }
            }
        }
    )*};
}
value_shapes!(SurfaceIndex, MaterialIndex);

const TEXTURE: &str = "texture values";

impl Shape<[Surface]> for Vec<Vec<TexturedRing>> {
    fn check_shape(&self, what: &'static str, b: &[Surface], errs: &mut Vec<CjseqError>) {
        count(what, b.len(), self.len(), errs);
        for (rings, surface) in self.iter().zip(b) {
            count(what, surface.len(), rings.len(), errs);
            for (ring, vertices) in rings.iter().zip(surface) {
                if let TexturedRing::Textured { uvs, .. } = ring {
                    count("texture vertices", vertices.len(), uvs.len(), errs);
                }
            }
        }
    }
}

impl Shape<[Shell]> for Vec<Vec<Vec<TexturedRing>>> {
    fn check_shape(&self, what: &'static str, b: &[Shell], errs: &mut Vec<CjseqError>) {
        count(what, b.len(), self.len(), errs);
        for (t, shell) in self.iter().zip(b) {
            t.check_shape(what, &shell[..], errs);
        }
    }
}

impl Shape<[Vec<Shell>]> for Vec<Vec<Vec<Vec<TexturedRing>>>> {
    fn check_shape(&self, what: &'static str, b: &[Vec<Shell>], errs: &mut Vec<CjseqError>) {
        count(what, b.len(), self.len(), errs);
        for (t, solid) in self.iter().zip(b) {
            t.check_shape(what, &solid[..], errs);
        }
    }
}

fn semantics<V, B: ?Sized>(s: &Option<Semantics<V>>, b: &B, errs: &mut Vec<CjseqError>)
where
    V: Indices<SurfaceIndex> + Shape<B>,
{
    let Some(s) = s else { return };
    let n = s.surfaces.len();
    s.visit_index(&mut |i| in_range(i, n, errs));
    if let Some(v) = &s.values {
        v.check_shape("semantics values", b, errs);
    }
}

fn material<V, B: ?Sized>(
    m: &Option<Themes<MaterialReference<V>>>,
    b: &B,
    l: &Lengths,
    errs: &mut Vec<CjseqError>,
) where
    V: Indices<MaterialIndex> + Shape<B>,
{
    let Some(m) = m else { return };
    for (theme, r) in m {
        if r.value.is_some() == r.values.is_some() {
            errs.push(CjseqError::Validation(format!(
                "material theme {theme:?} must have exactly one of \"value\" and \"values\""
            )));
        }
        r.visit_index(&mut |i| in_range(i, l.materials, errs));
        if let Some(Some(v)) = &r.values {
            v.check_shape("material values", b, errs);
        }
    }
}

fn texture<V, B: ?Sized>(
    t: &Option<Themes<TextureReference<V>>>,
    b: &B,
    l: &Lengths,
    errs: &mut Vec<CjseqError>,
) where
    V: Indices<TextureIndex> + Indices<UvIndex> + Shape<B>,
{
    let Some(t) = t else { return };
    for r in t.values() {
        Indices::<TextureIndex>::visit_index(r, &mut |i| in_range(i, l.textures, errs));
        Indices::<UvIndex>::visit_index(r, &mut |i| in_range(i, l.uvs, errs));
        if let Some(v) = &r.values {
            v.check_shape(TEXTURE, b, errs);
        }
    }
}

fn geometry(g: &Geometry, l: &Lengths, errs: &mut Vec<CjseqError>) {
    g.visit_vertices(&mut |v| in_range(v, l.vertices, errs));
    match g {
        Geometry::MultiPoint {
            boundaries,
            semantics: s,
            ..
        } => semantics(s, &boundaries[..], errs),
        Geometry::MultiLineString {
            boundaries,
            semantics: s,
            ..
        } => semantics(s, &boundaries[..], errs),
        Geometry::MultiSurface {
            boundaries,
            semantics: s,
            material: m,
            texture: t,
            ..
        }
        | Geometry::CompositeSurface {
            boundaries,
            semantics: s,
            material: m,
            texture: t,
            ..
        } => {
            semantics(s, &boundaries[..], errs);
            material(m, &boundaries[..], l, errs);
            texture(t, &boundaries[..], l, errs);
        }
        Geometry::Solid {
            boundaries,
            semantics: s,
            material: m,
            texture: t,
            ..
        } => {
            semantics(s, &boundaries[..], errs);
            material(m, &boundaries[..], l, errs);
            texture(t, &boundaries[..], l, errs);
        }
        Geometry::MultiSolid {
            boundaries,
            semantics: s,
            material: m,
            texture: t,
            ..
        }
        | Geometry::CompositeSolid {
            boundaries,
            semantics: s,
            material: m,
            texture: t,
            ..
        } => {
            semantics(s, &boundaries[..], errs);
            material(m, &boundaries[..], l, errs);
            texture(t, &boundaries[..], l, errs);
        }
        Geometry::GeometryInstance { template, .. } => {
            if let Some(n) = l.templates {
                in_range(*template, n, errs);
            }
        }
    }
}

/// Every child and parent named exists, and names the other back.
fn hierarchy(cos: &IndexMap<String, CityObject>, errs: &mut Vec<CjseqError>) {
    let names = |list: &Option<Vec<String>>, id: &str| {
        list.as_ref().is_some_and(|l| l.iter().any(|x| x == id))
    };
    for (id, co) in cos {
        for child in co.children.iter().flatten() {
            match cos.get(child) {
                None => errs.push(CjseqError::DanglingReference {
                    from: id.clone(),
                    to: child.clone(),
                }),
                Some(c) if !names(&c.parents, id) => errs.push(CjseqError::HierarchyMismatch {
                    parent: id.clone(),
                    child: child.clone(),
                }),
                _ => {}
            }
        }
        for parent in co.parents.iter().flatten() {
            match cos.get(parent) {
                None => errs.push(CjseqError::DanglingReference {
                    from: id.clone(),
                    to: parent.clone(),
                }),
                Some(p) if !names(&p.children, id) => errs.push(CjseqError::HierarchyMismatch {
                    parent: parent.clone(),
                    child: id.clone(),
                }),
                _ => {}
            }
        }
    }
}

fn city_objects(cos: &IndexMap<String, CityObject>, l: &Lengths, errs: &mut Vec<CjseqError>) {
    hierarchy(cos, errs);
    for (id, co) in cos {
        let mut inner = Vec::new();
        for g in co.geometry.iter().flatten() {
            geometry(g, l, &mut inner);
        }
        errs.extend(inner.into_iter().map(|e| CjseqError::InCityObject {
            id: id.clone(),
            source: Box::new(e),
        }));
    }
}

fn first(errs: Vec<CjseqError>) -> Result<()> {
    match errs.into_iter().next() {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// The root members a CityJSONFeature must not carry: they belong to the
/// document (`cityjsonfeature.schema.json`'s `not`).
const DOCUMENT_ONLY: [&str; 5] = [
    "transform",
    "version",
    "metadata",
    "geometry-templates",
    "extensions",
];

impl CityJSON {
    /// Every reference the document makes that serde cannot check, with the
    /// City Object or template each problem is in. Empty if the document is
    /// valid.
    pub fn errors(&self) -> Vec<CjseqError> {
        let mut errs = Vec::new();
        let app = self.appearance.as_ref();
        if let Some(gt) = &self.geometry_templates {
            let l = Lengths::new(gt.vertices_templates.len(), None, app);
            for (index, g) in gt.templates.iter().enumerate() {
                let mut inner = Vec::new();
                geometry(g, &l, &mut inner);
                errs.extend(inner.into_iter().map(|e| CjseqError::InTemplate {
                    index,
                    source: Box::new(e),
                }));
            }
        }
        let templates = self
            .geometry_templates
            .as_ref()
            .map_or(0, |t| t.templates.len());
        let l = Lengths::new(self.vertices.len(), Some(templates), app);
        city_objects(&self.city_objects, &l, &mut errs);
        errs
    }

    /// The first of [`CityJSON::errors`], if any.
    pub fn validate(&self) -> Result<()> {
        first(self.errors())
    }
}

impl CityJSONFeature {
    /// As [`CityJSON::errors`], for what a feature can check on its own: a
    /// GeometryInstance's template lives in the document's header, so it is
    /// not checked here.
    pub fn errors(&self) -> Vec<CjseqError> {
        let mut errs: Vec<CjseqError> = DOCUMENT_ONLY
            .iter()
            .filter(|m| self.other.contains_key(**m))
            .map(|m| CjseqError::MisplacedMember {
                member: m.to_string(),
            })
            .collect();
        if !self.city_objects.contains_key(&self.id) {
            errs.push(CjseqError::DanglingReference {
                from: "CityJSONFeature.id".to_string(),
                to: self.id.clone(),
            });
        }
        let l = Lengths::new(self.vertices.len(), None, self.appearance.as_ref());
        city_objects(&self.city_objects, &l, &mut errs);
        errs
    }

    /// The first of [`CityJSONFeature::errors`], if any.
    pub fn validate(&self) -> Result<()> {
        first(self.errors())
    }
}
