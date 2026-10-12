use crate::appearance::{MaterialReference, TextureReference};
use crate::error::{CjseqError, Result};
use crate::semantics::Semantics;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// A closed sequence of vertex indices bounding a face.
pub type Ring = Vec<usize>;
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

/// The members every geometry may carry that are neither its type, its lod,
/// nor its boundaries. The appearance values are depth-typed, keyed by theme.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct GeometryCommon {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantics: Option<Semantics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub material: Option<HashMap<String, MaterialReference>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture: Option<HashMap<String, TextureReference>>,
}

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
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type")]
pub enum Geometry {
    MultiPoint {
        #[serde(skip_serializing_if = "Option::is_none")]
        lod: Option<String>,
        boundaries: Ring,
        #[serde(flatten)]
        common: GeometryCommon,
    },
    MultiLineString {
        #[serde(skip_serializing_if = "Option::is_none")]
        lod: Option<String>,
        boundaries: Vec<Ring>,
        #[serde(flatten)]
        common: GeometryCommon,
    },
    MultiSurface {
        #[serde(skip_serializing_if = "Option::is_none")]
        lod: Option<String>,
        boundaries: Vec<Surface>,
        #[serde(flatten)]
        common: GeometryCommon,
    },
    CompositeSurface {
        #[serde(skip_serializing_if = "Option::is_none")]
        lod: Option<String>,
        boundaries: Vec<Surface>,
        #[serde(flatten)]
        common: GeometryCommon,
    },
    Solid {
        #[serde(skip_serializing_if = "Option::is_none")]
        lod: Option<String>,
        boundaries: Vec<Shell>,
        #[serde(flatten)]
        common: GeometryCommon,
    },
    MultiSolid {
        #[serde(skip_serializing_if = "Option::is_none")]
        lod: Option<String>,
        boundaries: Vec<Vec<Shell>>,
        #[serde(flatten)]
        common: GeometryCommon,
    },
    CompositeSolid {
        #[serde(skip_serializing_if = "Option::is_none")]
        lod: Option<String>,
        boundaries: Vec<Vec<Shell>>,
        #[serde(flatten)]
        common: GeometryCommon,
    },
    GeometryInstance {
        boundaries: Ring,
        template: usize,
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
    pub fn lod(&self) -> Option<&str> {
        match self {
            Geometry::MultiPoint { lod, .. }
            | Geometry::MultiLineString { lod, .. }
            | Geometry::MultiSurface { lod, .. }
            | Geometry::CompositeSurface { lod, .. }
            | Geometry::Solid { lod, .. }
            | Geometry::MultiSolid { lod, .. }
            | Geometry::CompositeSolid { lod, .. } => lod.as_deref(),
            Geometry::GeometryInstance { .. } => None,
        }
    }

    /// The semantics/material/texture members, if this variant has them.
    /// A [`Geometry::GeometryInstance`] carries none: its appearance lives on
    /// the template it refers to.
    pub fn common(&self) -> Option<&GeometryCommon> {
        match self {
            Geometry::MultiPoint { common, .. }
            | Geometry::MultiLineString { common, .. }
            | Geometry::MultiSurface { common, .. }
            | Geometry::CompositeSurface { common, .. }
            | Geometry::Solid { common, .. }
            | Geometry::MultiSolid { common, .. }
            | Geometry::CompositeSolid { common, .. } => Some(common),
            Geometry::GeometryInstance { .. } => None,
        }
    }

    pub fn common_mut(&mut self) -> Option<&mut GeometryCommon> {
        match self {
            Geometry::MultiPoint { common, .. }
            | Geometry::MultiLineString { common, .. }
            | Geometry::MultiSurface { common, .. }
            | Geometry::CompositeSurface { common, .. }
            | Geometry::Solid { common, .. }
            | Geometry::MultiSolid { common, .. }
            | Geometry::CompositeSolid { common, .. } => Some(common),
            Geometry::GeometryInstance { .. } => None,
        }
    }

    /// Every vertex index in `boundaries`, in document order, whatever the
    /// nesting depth of this variant. The depth is known statically per
    /// variant, so no runtime inspection of the boundaries is needed.
    fn boundary_indices_mut(&mut self) -> Box<dyn Iterator<Item = &mut usize> + '_> {
        match self {
            Geometry::MultiPoint { boundaries, .. }
            | Geometry::GeometryInstance { boundaries, .. } => Box::new(boundaries.iter_mut()),
            Geometry::MultiLineString { boundaries, .. } => {
                Box::new(boundaries.iter_mut().flatten())
            }
            Geometry::MultiSurface { boundaries, .. }
            | Geometry::CompositeSurface { boundaries, .. } => {
                Box::new(boundaries.iter_mut().flatten().flatten())
            }
            Geometry::Solid { boundaries, .. } => {
                Box::new(boundaries.iter_mut().flatten().flatten().flatten())
            }
            Geometry::MultiSolid { boundaries, .. }
            | Geometry::CompositeSolid { boundaries, .. } => Box::new(
                boundaries
                    .iter_mut()
                    .flatten()
                    .flatten()
                    .flatten()
                    .flatten(),
            ),
        }
    }

    pub(crate) fn update_geometry_boundaries(&mut self, violdnew: &mut HashMap<usize, usize>) {
        for vi in self.boundary_indices_mut() {
            let next = violdnew.len();
            *vi = *violdnew.entry(*vi).or_insert(next);
        }
    }

    pub(crate) fn offset_geometry_boundaries(&mut self, offset: usize) {
        for vi in self.boundary_indices_mut() {
            *vi += offset;
        }
    }

    /// Renumber every material index against `m_oldnew`, first-encounter
    /// order. The depth of `values` comes from its variant, so the geometry
    /// type is never consulted.
    pub(crate) fn update_material(&mut self, m_oldnew: &mut HashMap<usize, usize>) {
        let Some(common) = self.common_mut() else {
            return;
        };
        let Some(materials) = common.material.as_mut() else {
            return;
        };
        for mat in materials.values_mut() {
            //-- material.value colours the whole object
            if let Some(thevalue) = mat.value {
                let l = m_oldnew.len();
                mat.value = Some(*m_oldnew.entry(thevalue).or_insert(l));
                continue;
            }
            //-- else it's material.values, one index per surface. The outer
            //-- Option is present-vs-absent, the inner null-vs-array; there is
            //-- nothing to renumber unless both are Some.
            let Some(values) = mat.values.as_mut().and_then(|v| v.as_mut()) else {
                continue;
            };
            for x in values.indices_mut() {
                let l = m_oldnew.len();
                *x = *m_oldnew.entry(*x).or_insert(l);
            }
        }
    }

    /// Renumber every texture index against `t_oldnew` and every texture
    /// vertex against `t_v_oldnew`, both in first-encounter order. An index
    /// already in a map keeps the value the map gives it, so a caller that
    /// knows the target numbering up front (as `CityJSON::add_cjfeature`
    /// does) fills the map first.
    pub(crate) fn update_texture(
        &mut self,
        t_oldnew: &mut HashMap<usize, usize>,
        t_v_oldnew: &mut HashMap<usize, usize>,
    ) {
        let Some(common) = self.common_mut() else {
            return;
        };
        let Some(textures) = common.texture.as_mut() else {
            return;
        };
        //-- the first index of the innermost array is a texture, the rest are
        //-- texture vertices; they are renumbered against different maps.
        let mut remap = |is_texture: bool, thevalue: usize| -> usize {
            let map = if is_texture {
                &mut *t_oldnew
            } else {
                &mut *t_v_oldnew
            };
            let l = map.len();
            *map.entry(thevalue).or_insert(l)
        };
        for tex in textures.values_mut() {
            //-- a theme may legally carry no `values` at all
            let Some(values) = tex.values.as_mut() else {
                continue;
            };
            for ring in values.rings_mut() {
                for (k, z) in ring.iter_mut().enumerate() {
                    if let Some(thevalue) = *z {
                        *z = Some(remap(k == 0, thevalue));
                    }
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GeometryTemplates {
    pub templates: Vec<Geometry>,
    #[serde(rename = "vertices-templates")]
    pub vertices_templates: Value,
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
            assert_eq!(g.lod(), Some("1"));
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
        assert!(g.common().is_none());
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
        let mut map: HashMap<usize, usize> = HashMap::new();
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
        let mut map: HashMap<usize, usize> = HashMap::new();
        g.update_geometry_boundaries(&mut map);
        assert_eq!(
            serde_json::to_value(&g).unwrap()["boundaries"],
            serde_json::json!([[[[[0, 1, 0]]]], [[[[1, 2, 2]]]]])
        );
        let mut entries: Vec<(usize, usize)> = map.into_iter().collect();
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
        let mut t_oldnew: HashMap<usize, usize> = HashMap::new();
        let mut t_v_oldnew: HashMap<usize, usize> = HashMap::new();
        g.update_texture(&mut t_oldnew, &mut t_v_oldnew);

        let values =
            serde_json::to_value(&g.common().unwrap().texture.as_ref().unwrap()["winter"].values)
                .unwrap();
        assert_eq!(
            values,
            serde_json::json!([[[0, 0, 1, 2, 0]], [[0, 2, 1, 3, 2]], [[null]]])
        );
        let mut tv: Vec<(usize, usize)> = t_v_oldnew.into_iter().collect();
        tv.sort();
        assert_eq!(tv, vec![(20, 0), (21, 1), (22, 2), (30, 3)]);
        let t: Vec<(usize, usize)> = t_oldnew.into_iter().collect();
        assert_eq!(t, vec![(5, 0)]);
    }

    /// An index the caller has already mapped keeps that mapping; this is how
    /// collecting shifts a feature's texture vertices past those already
    /// collected.
    #[test]
    fn update_texture_honours_a_prefilled_map() {
        let mut g: Geometry = serde_json::from_value(serde_json::json!({
            "type": "MultiSurface",
            "boundaries": [[[0, 1, 2]]],
            "texture": {"t": {"values": [[[0, 2, 0, 1]]]}}
        }))
        .unwrap();
        let mut t_oldnew: HashMap<usize, usize> = HashMap::from([(0, 3)]);
        let mut t_v_oldnew: HashMap<usize, usize> = (0..3).map(|i| (i, 10 + i)).collect();
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
        let mut m_oldnew: HashMap<usize, usize> = HashMap::new();
        g.update_material(&mut m_oldnew);

        //-- as above: typed `values`, oracle expectations unchanged.
        let values = serde_json::to_value(
            g.common().unwrap().material.as_ref().unwrap()["irradiation"]
                .values
                .as_ref()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            values,
            serde_json::json!([[0, null, 1], [1, null, 0]]),
            "null holes must survive and indices must compact in first-encounter order"
        );

        let mut m: Vec<(usize, usize)> = m_oldnew.into_iter().collect();
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
            "type": "MultiSurface", "boundaries": [[[0, 1, 2]]]
        }))
        .unwrap();
        assert_eq!(
            serde_json::to_value(&g).unwrap(),
            serde_json::json!({"type": "MultiSurface", "boundaries": [[[0, 1, 2]]]})
        );
    }
}
