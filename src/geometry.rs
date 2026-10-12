use crate::appearance::{
    MaterialReference, TextureReference, TexturedRing, TexturedShell, TexturedSurface,
};
use crate::error::{CjseqError, Result};
use crate::index::{
    remap, Indices, MaterialIndex, MultiSolidValues, PrimitiveValues, SolidValues, SurfaceIndex,
    TemplateIndex, TextureIndex, UvIndex, VertexIndex,
};
use crate::semantics::{Semantics, SemanticsSurface};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// A closed sequence of vertex indices bounding a face.
pub type Ring = Vec<VertexIndex>;
/// A face: the exterior ring first, then any interior rings.
pub type Surface = Vec<Ring>;
/// A closed collection of surfaces bounding a volume.
pub type Shell = Vec<Surface>;

/// The tag of a [`Geometry`], for callers that need the kind without matching
/// on the whole enum.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryType {
    MultiPoint,
    MultiLineString,
    MultiSurface,
    CompositeSurface,
    Solid,
    MultiSolid,
    CompositeSolid,
    GeometryInstance,
}

impl GeometryType {
    /// The nesting depth `boundaries` must have for this geometry type: the
    /// number of array levels between the outermost array and a vertex index.
    pub fn boundary_depth(self) -> usize {
        match self {
            GeometryType::MultiPoint | GeometryType::GeometryInstance => 1,
            GeometryType::MultiLineString => 2,
            GeometryType::MultiSurface | GeometryType::CompositeSurface => 3,
            GeometryType::Solid => 4,
            GeometryType::MultiSolid | GeometryType::CompositeSolid => 5,
        }
    }
}

/// A level of detail (§ 3.2): `"0"` to `"3"`, optionally refined with one more
/// digit, `"0.0"` to `"3.3"` -- the twenty values `geomprimitives.schema.json`
/// enumerates. `"2"` and `"2.0"` are distinct values and stay distinct.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(try_from = "String", into = "String")]
pub struct Lod {
    level: u8,
    refinement: Option<u8>,
}

impl Lod {
    /// `level` and `refinement` must each be 0 to 3.
    pub fn new(level: u8, refinement: Option<u8>) -> Result<Lod> {
        if level > 3 || refinement.is_some_and(|r| r > 3) {
            return Err(CjseqError::Validation(format!(
                "LoD {level}.{refinement:?} is out of range: both parts are 0 to 3"
            )));
        }
        Ok(Lod { level, refinement })
    }
    /// The LoD level, 0 to 3.
    pub fn level(&self) -> u8 {
        self.level
    }
    /// The refinement after the dot, if the value has one.
    pub fn refinement(&self) -> Option<u8> {
        self.refinement
    }
}

impl std::fmt::Display for Lod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.refinement {
            Some(r) => write!(f, "{}.{r}", self.level),
            None => write!(f, "{}", self.level),
        }
    }
}

impl std::str::FromStr for Lod {
    type Err = CjseqError;
    fn from_str(s: &str) -> Result<Lod> {
        let digit = |c: &str| match c.as_bytes() {
            [d @ b'0'..=b'3'] => Some(d - b'0'),
            _ => None,
        };
        let parsed = match s.split_once('.') {
            None => digit(s).map(|l| (l, None)),
            Some((l, r)) => digit(l).zip(digit(r)).map(|(l, r)| (l, Some(r))),
        };
        let (level, refinement) = parsed.ok_or_else(|| {
            CjseqError::Validation(format!(
                "{s:?} is not a LoD: expected \"0\" to \"3\" or \"0.0\" to \"3.3\""
            ))
        })?;
        Lod::new(level, refinement)
    }
}

impl TryFrom<String> for Lod {
    type Error = CjseqError;
    fn try_from(s: String) -> Result<Lod> {
        s.parse()
    }
}

impl From<Lod> for String {
    fn from(l: Lod) -> String {
        l.to_string()
    }
}

/// Per-theme appearance references of a geometry, keyed by theme name.
pub type Themes<T> = IndexMap<String, T>;

/// The number of array levels between `v` and the deepest non-array value
/// inside it. A non-array is 0 deep; `[]` is 1 deep.
fn nesting_depth(v: &Value) -> usize {
    match v {
        Value::Array(a) => 1 + a.iter().map(nesting_depth).max().unwrap_or(0),
        _ => 0,
    }
}

/// A CityJSON geometry object.
///
/// The nesting depth of `boundaries` is part of the type: a `MultiSurface`
/// whose boundaries are nested one level too deep does not deserialize, where
/// previously it decoded happily into an untyped `serde_json::Value` and left
/// every consumer to guess the depth.
///
/// So is the nesting depth of semantics, material and texture `values`: each
/// variant names its own value types, so values nested to the wrong depth for
/// their geometry do not deserialize either.
///
/// Each variant has exactly the members the schema names for it, and the
/// schema closes every geometry object, so any other member is rejected. A
/// MultiPoint or MultiLineString carries no material or texture.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum Geometry {
    MultiPoint {
        lod: Lod,
        boundaries: Ring,
        #[serde(skip_serializing_if = "Option::is_none")]
        semantics: Option<Semantics>,
    },
    MultiLineString {
        lod: Lod,
        boundaries: Vec<Ring>,
        #[serde(skip_serializing_if = "Option::is_none")]
        semantics: Option<Semantics>,
    },
    MultiSurface {
        lod: Lod,
        boundaries: Vec<Surface>,
        #[serde(skip_serializing_if = "Option::is_none")]
        semantics: Option<Semantics<PrimitiveValues<SurfaceIndex>>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        material: Option<Themes<MaterialReference<PrimitiveValues<MaterialIndex>>>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        texture: Option<Themes<TextureReference<Vec<TexturedSurface>>>>,
    },
    CompositeSurface {
        lod: Lod,
        boundaries: Vec<Surface>,
        #[serde(skip_serializing_if = "Option::is_none")]
        semantics: Option<Semantics<PrimitiveValues<SurfaceIndex>>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        material: Option<Themes<MaterialReference<PrimitiveValues<MaterialIndex>>>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        texture: Option<Themes<TextureReference<Vec<TexturedSurface>>>>,
    },
    Solid {
        lod: Lod,
        boundaries: Vec<Shell>,
        #[serde(skip_serializing_if = "Option::is_none")]
        semantics: Option<Semantics<SolidValues<SurfaceIndex>>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        material: Option<Themes<MaterialReference<SolidValues<MaterialIndex>>>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        texture: Option<Themes<TextureReference<Vec<TexturedShell>>>>,
    },
    MultiSolid {
        lod: Lod,
        boundaries: Vec<Vec<Shell>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        semantics: Option<Semantics<MultiSolidValues<SurfaceIndex>>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        material: Option<Themes<MaterialReference<MultiSolidValues<MaterialIndex>>>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        texture: Option<Themes<TextureReference<Vec<Vec<TexturedShell>>>>>,
    },
    CompositeSolid {
        lod: Lod,
        boundaries: Vec<Vec<Shell>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        semantics: Option<Semantics<MultiSolidValues<SurfaceIndex>>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        material: Option<Themes<MaterialReference<MultiSolidValues<MaterialIndex>>>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        texture: Option<Themes<TextureReference<Vec<Vec<TexturedShell>>>>>,
    },
    GeometryInstance {
        /// The one vertex the template is placed at.
        boundaries: [VertexIndex; 1],
        template: TemplateIndex,
        #[serde(rename = "transformationMatrix")]
        transformation_matrix: [f64; 16],
    },
}

impl Geometry {
    /// Parse a geometry, reporting a wrong `boundaries` depth in terms of the
    /// geometry type rather than in terms of serde's innards.
    ///
    /// `serde_json::from_value::<Geometry>` works just as well and is what
    /// `#[derive(Deserialize)]` gives every containing type; the only thing
    /// this adds is a readable error. Deserializing a deeply nested `Vec`
    /// otherwise fails with "invalid type: integer, expected a sequence",
    /// which says neither which geometry was at fault nor what depth it
    /// wanted.
    pub fn from_json_value(v: Value) -> Result<Geometry> {
        //-- `&Value` is itself a Deserializer, so the happy path borrows `v`
        //-- instead of deep-copying every boundary just to keep `v` alive for
        //-- the error branch below.
        let err = match Geometry::deserialize(&v) {
            Ok(g) => return Ok(g),
            Err(e) => e,
        };
        //-- can the failure be pinned on the depth of `boundaries`?
        let thetype = v
            .get("type")
            .and_then(|t| GeometryType::deserialize(t).ok());
        if let (Some(thetype), Some(boundaries)) = (thetype, v.get("boundaries")) {
            let expected = thetype.boundary_depth();
            let found = nesting_depth(boundaries);
            if found != expected {
                return Err(CjseqError::GeometryDepth {
                    geometry_type: thetype,
                    expected,
                    found,
                });
            }
        }
        Err(CjseqError::Json(err))
    }

    pub fn geometry_type(&self) -> GeometryType {
        match self {
            Geometry::MultiPoint { .. } => GeometryType::MultiPoint,
            Geometry::MultiLineString { .. } => GeometryType::MultiLineString,
            Geometry::MultiSurface { .. } => GeometryType::MultiSurface,
            Geometry::CompositeSurface { .. } => GeometryType::CompositeSurface,
            Geometry::Solid { .. } => GeometryType::Solid,
            Geometry::MultiSolid { .. } => GeometryType::MultiSolid,
            Geometry::CompositeSolid { .. } => GeometryType::CompositeSolid,
            Geometry::GeometryInstance { .. } => GeometryType::GeometryInstance,
        }
    }

    /// The lod of this geometry; a [`Geometry::GeometryInstance`] has none.
    pub fn lod(&self) -> Option<Lod> {
        match self {
            Geometry::MultiPoint { lod, .. }
            | Geometry::MultiLineString { lod, .. }
            | Geometry::MultiSurface { lod, .. }
            | Geometry::CompositeSurface { lod, .. }
            | Geometry::Solid { lod, .. }
            | Geometry::MultiSolid { lod, .. }
            | Geometry::CompositeSolid { lod, .. } => Some(*lod),
            Geometry::GeometryInstance { .. } => None,
        }
    }

    /// The semantic surfaces, whatever the geometry's depth; the `values`
    /// that refer to them are on each variant.
    pub fn surfaces(&self) -> Option<&[SemanticsSurface]> {
        match self {
            Geometry::MultiPoint { semantics, .. }
            | Geometry::MultiLineString { semantics, .. }
            | Geometry::MultiSurface { semantics, .. }
            | Geometry::CompositeSurface { semantics, .. } => {
                semantics.as_ref().map(|s| &s.surfaces[..])
            }
            Geometry::Solid { semantics, .. } => semantics.as_ref().map(|s| &s.surfaces[..]),
            Geometry::MultiSolid { semantics, .. } | Geometry::CompositeSolid { semantics, .. } => {
                semantics.as_ref().map(|s| &s.surfaces[..])
            }
            Geometry::GeometryInstance { .. } => None,
        }
    }

    /// Visit every vertex index in `boundaries`, in document order.
    pub(crate) fn for_each_vertex(&mut self, f: &mut dyn FnMut(&mut VertexIndex)) {
        match self {
            Geometry::MultiPoint { boundaries, .. } => boundaries.for_each_index(f),
            Geometry::MultiLineString { boundaries, .. } => boundaries.for_each_index(f),
            Geometry::MultiSurface { boundaries, .. }
            | Geometry::CompositeSurface { boundaries, .. } => boundaries.for_each_index(f),
            Geometry::Solid { boundaries, .. } => boundaries.for_each_index(f),
            Geometry::MultiSolid { boundaries, .. }
            | Geometry::CompositeSolid { boundaries, .. } => boundaries.for_each_index(f),
            Geometry::GeometryInstance { boundaries, .. } => boundaries.for_each_index(f),
        }
    }

    /// As [`Geometry::for_each_vertex`], read-only.
    pub(crate) fn visit_vertices(&self, f: &mut dyn FnMut(VertexIndex)) {
        match self {
            Geometry::MultiPoint { boundaries, .. } => boundaries.visit_index(f),
            Geometry::MultiLineString { boundaries, .. } => boundaries.visit_index(f),
            Geometry::MultiSurface { boundaries, .. }
            | Geometry::CompositeSurface { boundaries, .. } => boundaries.visit_index(f),
            Geometry::Solid { boundaries, .. } => boundaries.visit_index(f),
            Geometry::MultiSolid { boundaries, .. }
            | Geometry::CompositeSolid { boundaries, .. } => boundaries.visit_index(f),
            Geometry::GeometryInstance { boundaries, .. } => boundaries.visit_index(f),
        }
    }

    /// Visit every material index (`value` and `values`, every theme).
    pub(crate) fn for_each_material(&mut self, f: &mut dyn FnMut(&mut MaterialIndex)) {
        match self {
            Geometry::MultiSurface { material, .. }
            | Geometry::CompositeSurface { material, .. } => material.for_each_index(f),
            Geometry::Solid { material, .. } => material.for_each_index(f),
            Geometry::MultiSolid { material, .. } | Geometry::CompositeSolid { material, .. } => {
                material.for_each_index(f)
            }
            _ => {}
        }
    }

    /// Visit every texture index, or every texture-vertex index, in the
    /// texture rings of every theme.
    pub(crate) fn for_each_texture<I>(&mut self, f: &mut dyn FnMut(&mut I))
    where
        TexturedRing: Indices<I>,
    {
        match self {
            Geometry::MultiSurface { texture, .. } | Geometry::CompositeSurface { texture, .. } => {
                texture.for_each_index(f)
            }
            Geometry::Solid { texture, .. } => texture.for_each_index(f),
            Geometry::MultiSolid { texture, .. } | Geometry::CompositeSolid { texture, .. } => {
                texture.for_each_index(f)
            }
            _ => {}
        }
    }

    pub(crate) fn update_geometry_boundaries(
        &mut self,
        violdnew: &mut HashMap<VertexIndex, VertexIndex>,
    ) {
        self.for_each_vertex(&mut |vi| *vi = remap(violdnew, *vi));
    }

    pub(crate) fn offset_geometry_boundaries(&mut self, offset: usize) {
        self.for_each_vertex(&mut |vi| vi.0 += offset);
    }

    /// Renumber every material index against `m_oldnew`, first-encounter
    /// order.
    pub(crate) fn update_material(&mut self, m_oldnew: &mut HashMap<MaterialIndex, MaterialIndex>) {
        self.for_each_material(&mut |m| *m = remap(m_oldnew, *m));
    }

    /// Renumber every texture index against `t_oldnew` and every texture
    /// vertex against `t_v_oldnew`, both in first-encounter order. An index
    /// already in a map keeps the value the map gives it, so a caller that
    /// knows the target numbering up front (as `Collector::add`
    /// does) fills the map first.
    pub(crate) fn update_texture(
        &mut self,
        t_oldnew: &mut HashMap<TextureIndex, TextureIndex>,
        t_v_oldnew: &mut HashMap<UvIndex, UvIndex>,
    ) {
        self.for_each_texture(&mut |t: &mut TextureIndex| *t = remap(t_oldnew, *t));
        self.for_each_texture(&mut |uv: &mut UvIndex| *uv = remap(t_v_oldnew, *uv));
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeometryTemplates {
    pub templates: Vec<Geometry>,
    /// Real coordinates, not transformed integers: templates are placed by
    /// their instances' transformation matrices.
    #[serde(rename = "vertices-templates")]
    pub vertices_templates: Vec<[f64; 3]>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(v: serde_json::Value) -> std::result::Result<Geometry, serde_json::Error> {
        serde_json::from_value(v)
    }

    #[test]
    fn each_variant_has_its_own_depth() {
        let cases = [
            (GeometryType::MultiPoint, serde_json::json!([0, 1])),
            (GeometryType::MultiLineString, serde_json::json!([[0, 1]])),
            (GeometryType::MultiSurface, serde_json::json!([[[0, 1, 2]]])),
            (
                GeometryType::CompositeSurface,
                serde_json::json!([[[0, 1, 2]]]),
            ),
            (GeometryType::Solid, serde_json::json!([[[[0, 1, 2]]]])),
            (
                GeometryType::MultiSolid,
                serde_json::json!([[[[[0, 1, 2]]]]]),
            ),
            (
                GeometryType::CompositeSolid,
                serde_json::json!([[[[[0, 1, 2]]]]]),
            ),
        ];
        for (thetype, boundaries) in cases {
            let name = serde_json::to_value(thetype).unwrap();
            let g = parse(serde_json::json!({
                "type": name, "lod": "1", "boundaries": boundaries.clone()
            }))
            .unwrap_or_else(|e| panic!("{name} must accept its own depth: {e}"));
            assert_eq!(g.geometry_type(), thetype);
            assert_eq!(g.lod().map(|l| l.to_string()).as_deref(), Some("1"));
            //-- and it must serialize back to exactly the boundaries it was given
            assert_eq!(
                serde_json::to_value(&g).unwrap()["boundaries"],
                boundaries,
                "{name} must round-trip its boundaries"
            );

            //-- one level too deep must be rejected
            let too_deep = serde_json::json!([boundaries]);
            assert!(
                parse(serde_json::json!({
                    "type": name, "lod": "1", "boundaries": too_deep
                }))
                .is_err(),
                "{name} must reject boundaries one level too deep"
            );

            //-- and one level too shallow must be rejected too
            let too_shallow = boundaries[0].clone();
            assert!(
                parse(serde_json::json!({
                    "type": name, "lod": "1", "boundaries": too_shallow
                }))
                .is_err(),
                "{name} must reject boundaries one level too shallow"
            );
        }
    }

    #[test]
    fn geometry_instance_roundtrips() {
        let input = serde_json::json!({
            "type": "GeometryInstance",
            "boundaries": [372],
            "template": 0,
            "transformationMatrix": [
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 1.0
            ]
        });
        let g: Geometry = serde_json::from_value(input.clone()).unwrap();
        assert_eq!(g.geometry_type(), GeometryType::GeometryInstance);
        assert!(g.surfaces().is_none());
        assert_eq!(serde_json::to_value(&g).unwrap(), input);
    }

    #[test]
    fn offset_walks_every_depth() {
        let mut g: Geometry = serde_json::from_value(serde_json::json!({
            "type": "CompositeSolid", "lod": "2",
            "boundaries": [[[[[0, 1]]]], [[[[2, 3]]]]]
        }))
        .unwrap();
        g.offset_geometry_boundaries(10);
        assert_eq!(
            serde_json::to_value(&g).unwrap()["boundaries"],
            serde_json::json!([[[[[10, 11]]]], [[[[12, 13]]]]])
        );
    }

    #[test]
    fn update_boundaries_compacts_indices() {
        let mut g: Geometry = serde_json::from_value(serde_json::json!({
            "type": "MultiSurface", "lod": "2",
            "boundaries": [[[7, 9, 7]], [[9, 4, 4]]]
        }))
        .unwrap();
        let mut map: HashMap<VertexIndex, VertexIndex> = HashMap::new();
        g.update_geometry_boundaries(&mut map);
        assert_eq!(
            serde_json::to_value(&g).unwrap()["boundaries"],
            serde_json::json!([[[0, 1, 0]], [[1, 2, 2]]])
        );
        assert_eq!(map.len(), 3);
    }

    /// The same first-encounter renumbering, but at the deepest traversal the
    /// enum has, so the flattening order of the 5-level variants is pinned too.
    /// Expected values produced by the pre-rewrite implementation at a97bc2d.
    #[test]
    fn update_boundaries_compacts_indices_at_max_depth() {
        let mut g: Geometry = serde_json::from_value(serde_json::json!({
            "type": "CompositeSolid", "lod": "2",
            "boundaries": [[[[[7, 9, 7]]]], [[[[9, 4, 4]]]]]
        }))
        .unwrap();
        let mut map: HashMap<VertexIndex, VertexIndex> = HashMap::new();
        g.update_geometry_boundaries(&mut map);
        assert_eq!(
            serde_json::to_value(&g).unwrap()["boundaries"],
            serde_json::json!([[[[[0, 1, 0]]]], [[[[1, 2, 2]]]]])
        );
        let mut entries: Vec<(usize, usize)> = map.into_iter().map(|(a, b)| (a.0, b.0)).collect();
        entries.sort();
        assert_eq!(entries, vec![(4, 2), (7, 0), (9, 1)]);
    }

    /// `update_texture` renumbers the first entry of each innermost array
    /// against the texture map and the rest against the texture-vertex map,
    /// both in first-encounter order, and a repeated index maps to the same
    /// new index every time.
    #[test]
    fn update_texture_compacts_textures_and_texture_vertices() {
        let mut g: Geometry = serde_json::from_value(serde_json::json!({
            "type": "MultiSurface", "lod": "2",
            "boundaries": [[[0, 1, 2, 3]], [[4, 5, 6, 7]], [[8, 9, 10, 11]]],
            "texture": {"winter": {"values": [
                [[5, 20, 21, 22, 20]],
                [[5, 22, 21, 30, 22]],
                [[null]]
            ]}}
        }))
        .unwrap();
        let mut t_oldnew: HashMap<TextureIndex, TextureIndex> = HashMap::new();
        let mut t_v_oldnew: HashMap<UvIndex, UvIndex> = HashMap::new();
        g.update_texture(&mut t_oldnew, &mut t_v_oldnew);

        let values = serde_json::to_value(&g).unwrap()["texture"]["winter"]["values"].clone();
        assert_eq!(
            values,
            serde_json::json!([[[0, 0, 1, 2, 0]], [[0, 2, 1, 3, 2]], [[null]]])
        );
        let mut tv: Vec<(usize, usize)> = t_v_oldnew.into_iter().map(|(a, b)| (a.0, b.0)).collect();
        tv.sort();
        assert_eq!(tv, vec![(20, 0), (21, 1), (22, 2), (30, 3)]);
        let t: Vec<(usize, usize)> = t_oldnew.into_iter().map(|(a, b)| (a.0, b.0)).collect();
        assert_eq!(t, vec![(5, 0)]);
    }

    /// An index the caller has already mapped keeps that mapping; this is how
    /// collecting shifts a feature's texture vertices past those already
    /// collected.
    #[test]
    fn update_texture_honours_a_prefilled_map() {
        let mut g: Geometry = serde_json::from_value(serde_json::json!({
            "type": "MultiSurface", "lod": "2",
            "boundaries": [[[0, 1, 2]]],
            "texture": {"t": {"values": [[[0, 2, 0, 1]]]}}
        }))
        .unwrap();
        let mut t_oldnew = HashMap::from([(TextureIndex(0), TextureIndex(3))]);
        let mut t_v_oldnew: HashMap<UvIndex, UvIndex> =
            (0..3).map(|i| (UvIndex(i), UvIndex(10 + i))).collect();
        g.update_texture(&mut t_oldnew, &mut t_v_oldnew);
        assert_eq!(
            serde_json::to_value(&g).unwrap()["texture"]["t"]["values"],
            serde_json::json!([[[3, 12, 10, 11]]])
        );
    }

    /// `update_material` renumbers `material.values` at the depth the geometry
    /// type implies, and `null` holes (surfaces with no material) must survive
    /// untouched. Expected values produced by the pre-rewrite implementation at
    /// a97bc2d on this exact input.
    #[test]
    fn update_material_preserves_null_holes() {
        let mut g: Geometry = serde_json::from_value(serde_json::json!({
            "type": "Solid", "lod": "2",
            "boundaries": [[[[0, 1, 2, 3]], [[4, 5, 6, 7]], [[8, 9, 10, 11]]]],
            "material": {"irradiation": {"values": [[0, null, 2], [2, null, 0]]}}
        }))
        .unwrap();
        let mut m_oldnew: HashMap<MaterialIndex, MaterialIndex> = HashMap::new();
        g.update_material(&mut m_oldnew);

        //-- as above: typed `values`, oracle expectations unchanged.
        let values = serde_json::to_value(&g).unwrap()["material"]["irradiation"]["values"].clone();
        assert_eq!(
            values,
            serde_json::json!([[0, null, 1], [1, null, 0]]),
            "null holes must survive and indices must compact in first-encounter order"
        );

        let mut m: Vec<(usize, usize)> = m_oldnew.into_iter().map(|(a, b)| (a.0, b.0)).collect();
        m.sort();
        assert_eq!(m, vec![(0, 0), (2, 1)]);
    }

    /// A raw serde error on a deeply nested `Vec` reads "invalid type:
    /// integer, expected a sequence", which names neither the geometry nor the
    /// depth it wanted. `from_json_value` must say both.
    #[test]
    fn wrong_depth_reports_which_geometry_and_which_depth() {
        let err = Geometry::from_json_value(serde_json::json!({
            "type": "MultiSurface", "lod": "2",
            "boundaries": [[[[0, 1, 2]]]]
        }))
        .unwrap_err();

        match &err {
            CjseqError::GeometryDepth {
                geometry_type,
                expected,
                found,
            } => {
                assert_eq!(*geometry_type, GeometryType::MultiSurface);
                assert_eq!(*expected, 3, "a MultiSurface nests boundaries 3 deep");
                assert_eq!(
                    *found, 4,
                    "the depth actually found must be reported as a number, not prose"
                );
            }
            other => panic!("expected a GeometryDepth error, got {other:?}"),
        }

        let msg = err.to_string();
        assert!(
            msg.contains("MultiSurface") && msg.contains('3'),
            "the message must name the geometry and the depth it expected, got {msg:?}"
        );
    }

    /// A geometry whose boundaries are the right depth but which is malformed
    /// some other way is not misreported as a depth problem.
    #[test]
    fn a_non_depth_failure_stays_a_json_error() {
        let err = Geometry::from_json_value(serde_json::json!({
            "type": "MultiSurface", "lod": "2",
            "boundaries": [[["not an index"]]]
        }))
        .unwrap_err();
        assert!(
            matches!(err, CjseqError::Json(_)),
            "expected a Json error, got {err:?}"
        );
    }

    #[test]
    fn from_json_value_accepts_a_good_geometry() {
        let g = Geometry::from_json_value(serde_json::json!({
            "type": "Solid", "lod": "2", "boundaries": [[[[0, 1, 2]]]]
        }))
        .unwrap();
        assert_eq!(g.geometry_type(), GeometryType::Solid);
    }

    #[test]
    fn absent_members_are_not_serialized_as_null() {
        let g: Geometry = serde_json::from_value(serde_json::json!({
            "type": "MultiSurface", "lod": "1", "boundaries": [[[0, 1, 2]]]
        }))
        .unwrap();
        assert_eq!(
            serde_json::to_value(&g).unwrap(),
            serde_json::json!({"type": "MultiSurface", "lod": "1", "boundaries": [[[0, 1, 2]]]})
        );
    }
}
