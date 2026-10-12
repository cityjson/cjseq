use crate::appearance::{Appearance, MaterialObject, TextureObject};
use crate::city_object::CityObject;
use crate::error::{CjseqError, IndexKind, Result};
use crate::geometry::GeometryTemplates;
use crate::metadata::{Metadata, Transform};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;

/// One vertex: integer coordinates, to be scaled and translated by the
/// document's [`Transform`].
pub type Vertex = [i64; 3];

/// Slice a palette down to the entries one feature uses, in the order that
/// feature now refers to them.
///
/// `oldnew` maps each index used by the feature onto its dense position in the
/// new palette; `update_material`/`update_texture` build it by handing out
/// consecutive new indices, so its values are exactly `0..oldnew.len()` and
/// sorting by them reproduces the palette.
fn reindex<T: Clone>(all: &[T], oldnew: &HashMap<usize, usize>, kind: IndexKind) -> Result<Vec<T>> {
    let mut pairs: Vec<(usize, usize)> = oldnew.iter().map(|(o, n)| (*n, *o)).collect();
    pairs.sort_unstable();
    pairs
        .into_iter()
        .map(|(_, old)| {
            all.get(old).cloned().ok_or(CjseqError::IndexOutOfRange {
                kind,
                index: old,
                len: all.len(),
            })
        })
        .collect()
}

/// The part of `all` that the renumbering maps `m_oldnew`, `t_oldnew` and
/// `t_v_oldnew` refer to, with the default themes carried over.
fn slice_appearance(
    all: &Appearance,
    m_oldnew: &HashMap<usize, usize>,
    t_oldnew: &HashMap<usize, usize>,
    t_v_oldnew: &HashMap<usize, usize>,
) -> Result<Appearance> {
    let mut a = Appearance::new();
    a.default_theme_material = all.default_theme_material.clone();
    a.default_theme_texture = all.default_theme_texture.clone();
    if let Some(m) = &all.materials {
        a.materials = Some(reindex(m, m_oldnew, IndexKind::Material)?);
    }
    if let Some(t) = &all.textures {
        a.textures = Some(reindex(t, t_oldnew, IndexKind::Texture)?);
    }
    if let Some(tv) = &all.vertices_texture {
        a.vertices_texture = Some(reindex(tv, t_v_oldnew, IndexKind::TextureVertex)?);
    }
    Ok(a)
}

/// The order in which [`CityJSON::get_cjfeature`] hands out features.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortingStrategy {
    /// The order the top-level City Objects appear in the document.
    AsIs,
    /// A random permutation, different on every call.
    Random,
    /// Sorted by City Object id.
    Lexicographical,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CityJSON {
    #[serde(rename = "type")]
    pub thetype: CityJSONType,
    pub version: CityJSONVersion,
    pub transform: Transform,
    #[serde(rename = "CityObjects")]
    pub city_objects: IndexMap<String, CityObject>,
    pub vertices: Vec<Vertex>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub appearance: Option<Appearance>,
    #[serde(rename = "geometry-templates")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geometry_templates: Option<GeometryTemplates>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<IndexMap<String, Extension>>,
    /// Root members the schema does not name, kept verbatim.
    #[serde(flatten)]
    pub other: Map<String, Value>,
    #[serde(skip)]
    sorted_ids: Vec<String>,
    #[serde(skip)]
    transform_correction: Option<Transform>,
}

/// Equality is over the document only: `sorted_ids` and
/// `transform_correction` are sequencing state, not CityJSON members.
impl PartialEq for CityJSON {
    fn eq(&self, other: &Self) -> bool {
        self.thetype == other.thetype
            && self.version == other.version
            && self.transform == other.transform
            && self.city_objects == other.city_objects
            && self.vertices == other.vertices
            && self.metadata == other.metadata
            && self.appearance == other.appearance
            && self.geometry_templates == other.geometry_templates
            && self.extensions == other.extensions
            && self.other == other.other
    }
}

impl Default for CityJSON {
    fn default() -> Self {
        Self::new()
    }
}

impl FromStr for CityJSON {
    type Err = CjseqError;
    /// Parse a CityJSON document. Its features are handed out in document
    /// order until [`CityJSON::sort_cjfeatures`] says otherwise.
    fn from_str(s: &str) -> Result<Self> {
        let mut cjj: CityJSON = serde_json::from_str(s)?;
        cjj.sort_cjfeatures(SortingStrategy::AsIs);
        Ok(cjj)
    }
}

impl CityJSON {
    /// Create a new CityJSON instance with default values.
    pub fn new() -> Self {
        CityJSON {
            thetype: CityJSONType::CityJSON,
            version: CityJSONVersion::V2_0,
            transform: Transform::new(),
            city_objects: IndexMap::new(),
            vertices: Vec::new(),
            metadata: None,
            appearance: None,
            geometry_templates: None,
            extensions: None,
            other: Map::new(),
            sorted_ids: vec![],
            transform_correction: None,
        }
    }
    /// Get the "first line" (aka metadata or header) of a CityJSONSeq
    pub fn get_metadata(&self) -> Self {
        let mut cj0 = CityJSON {
            thetype: self.thetype.clone(),
            version: self.version,
            transform: self.transform.clone(),
            metadata: self.metadata.clone(),
            city_objects: IndexMap::new(),
            vertices: Vec::new(),
            appearance: None,
            geometry_templates: self.geometry_templates.clone(),
            other: self.other.clone(),
            extensions: self.extensions.clone(),
            sorted_ids: vec![],
            transform_correction: None,
        };
        //-- if geometry-templates have material/textures then these need to be
        //-- added to 1st line (metadata)
        if let Some(x) = &self.geometry_templates {
            let mut gts2: GeometryTemplates = x.clone();
            let mut m_oldnew: HashMap<usize, usize> = HashMap::new();
            let mut t_oldnew: HashMap<usize, usize> = HashMap::new();
            let mut t_v_oldnew: HashMap<usize, usize> = HashMap::new();
            for g in &mut gts2.templates {
                g.update_material(&mut m_oldnew);
                g.update_texture(&mut t_oldnew, &mut t_v_oldnew);
            }
            if let Some(a) = &self.appearance {
                cj0.appearance = slice_appearance(a, &m_oldnew, &t_oldnew, &t_v_oldnew).ok();
            }
        }
        cj0
    }

    /// The `i`th feature, in the order set by [`CityJSON::sort_cjfeatures`]:
    /// a top-level City Object with all of its descendants, and only the
    /// vertices and appearance entries they use.
    ///
    /// `Ok(None)` once `i` is past the last feature. A child id that names no
    /// City Object, or an index past the end of `vertices` or of an appearance
    /// palette, is an error.
    pub fn get_cjfeature(&self, i: usize) -> Result<Option<CityJSONFeature>> {
        let Some(id) = self.sorted_ids.get(i) else {
            return Ok(None);
        };
        let Some(root) = self.city_objects.get(id) else {
            return Ok(None);
        };
        //-- the root, then every descendant depth-first; `seen` stops a
        //-- parent/child cycle from looping
        let mut members: Vec<(&String, &CityObject)> = vec![(id, root)];
        let mut seen: HashSet<&str> = HashSet::from([id.as_str()]);
        let mut next = 0;
        while next < members.len() {
            let (parent_id, parent) = members[next];
            next += 1;
            for child_id in parent.children.iter().flatten() {
                if !seen.insert(child_id.as_str()) {
                    continue;
                }
                let child = self.city_objects.get(child_id).ok_or_else(|| {
                    CjseqError::DanglingReference {
                        from: parent_id.clone(),
                        to: child_id.clone(),
                    }
                })?;
                members.push((child_id, child));
            }
        }

        let mut cjf = CityJSONFeature::new();
        cjf.id = id.clone();
        let mut g_vi_oldnew: HashMap<usize, usize> = HashMap::new();
        let mut m_oldnew: HashMap<usize, usize> = HashMap::new();
        let mut t_oldnew: HashMap<usize, usize> = HashMap::new();
        let mut t_v_oldnew: HashMap<usize, usize> = HashMap::new();
        for (key, co) in members {
            let mut co2 = co.clone();
            for g in co2.geometry.iter_mut().flatten() {
                g.update_geometry_boundaries(&mut g_vi_oldnew);
                g.update_material(&mut m_oldnew);
                g.update_texture(&mut t_oldnew, &mut t_v_oldnew);
            }
            cjf.add_co(key.clone(), co2);
        }
        cjf.vertices = reindex(&self.vertices, &g_vi_oldnew, IndexKind::Vertex)?;
        if let Some(a) = &self.appearance {
            cjf.appearance = Some(slice_appearance(a, &m_oldnew, &t_oldnew, &t_v_oldnew)?);
        }
        Ok(Some(cjf))
    }
    /// Used when many CityJSONSeq are used, the "transform" can
    /// be modified (the new value is a "correction").
    pub fn add_transform_correction(&mut self, t: Transform) {
        self.transform_correction = Some(t);
    }
    pub fn add_cjfeature(&mut self, cjf: &mut CityJSONFeature) {
        let mut m_oldnew: HashMap<usize, usize> = HashMap::new();
        let mut t_oldnew: HashMap<usize, usize> = HashMap::new();
        let mut t_v_oldnew: HashMap<usize, usize> = HashMap::new();
        let g_offset = self.vertices.len();
        if let Some(cjf_app) = &cjf.appearance {
            if let Some(cjf_mat) = &cjf_app.materials {
                for (i, m) in cjf_mat.iter().enumerate() {
                    m_oldnew.insert(i, self.add_material(m.clone()));
                }
            }
            if let Some(cjf_tex) = &cjf_app.textures {
                for (i, m) in cjf_tex.iter().enumerate() {
                    t_oldnew.insert(i, self.add_texture(m.clone()));
                }
            }
            if let Some(cjf_v_tex) = &cjf_app.vertices_texture {
                //-- the feature's texture vertices are appended verbatim, so
                //-- each one's new index is its old one shifted by however
                //-- many were already collected
                let t_v_offset = self
                    .appearance
                    .as_ref()
                    .and_then(|a| a.vertices_texture.as_ref())
                    .map_or(0, Vec::len);
                for i in 0..cjf_v_tex.len() {
                    t_v_oldnew.insert(i, t_v_offset + i);
                }
                self.add_vertices_texture(cjf_v_tex.clone());
            }
        }

        for (key, co) in &mut cjf.city_objects {
            for g in co.geometry.iter_mut().flatten() {
                g.offset_geometry_boundaries(g_offset);
                g.update_material(&mut m_oldnew);
                g.update_texture(&mut t_oldnew, &mut t_v_oldnew);
            }
            self.add_co(key.to_string(), co.clone());
        }
        self.add_vertices(&mut cjf.vertices);
        self.sorted_ids.push(cjf.id.clone());
    }
    pub fn remove_duplicate_vertices(&mut self) {
        let mut h: HashMap<Vertex, usize> = HashMap::new();
        let mut newids: HashMap<usize, usize> = HashMap::new();
        let mut newvertices: Vec<Vertex> = Vec::new();
        for (i, v) in self.vertices.iter().enumerate() {
            let new = *h.entry(*v).or_insert_with(|| {
                newvertices.push(*v);
                newvertices.len() - 1
            });
            newids.insert(i, new);
        }
        for co in self.city_objects.values_mut() {
            for g in co.geometry.iter_mut().flatten() {
                g.update_geometry_boundaries(&mut newids);
            }
        }
        self.vertices = newvertices;
    }
    /// The per-axis minimum and maximum of `vertices`, or `None` if there are
    /// none.
    fn vertex_bounds(&self) -> Option<(Vertex, Vertex)> {
        let first = *self.vertices.first()?;
        Some(
            self.vertices
                .iter()
                .fold((first, first), |(mut lo, mut hi), v| {
                    for k in 0..3 {
                        lo[k] = lo[k].min(v[k]);
                        hi[k] = hi[k].max(v[k]);
                    }
                    (lo, hi)
                }),
        )
    }
    /// Recompute `metadata.geographicalExtent` from the vertices, if the
    /// document has one. With no vertices it is left as it is.
    pub fn update_geographicalextent(&mut self) {
        let Some((mins, maxs)) = self.vertex_bounds() else {
            return;
        };
        let t = &self.transform;
        let real = |v: Vertex, k: usize| v[k] as f64 * t.scale[k] + t.translate[k];
        if let Some(ge) = self
            .metadata
            .as_mut()
            .and_then(|m| m.geographical_extent.as_mut())
        {
            *ge = [
                real(mins, 0),
                real(mins, 1),
                real(mins, 2),
                real(maxs, 0),
                real(maxs, 1),
                real(maxs, 2),
            ];
        }
    }
    /// Move `transform.translate` to the vertices' minimum corner, so the
    /// stored integers start at 0. With no vertices nothing changes.
    pub fn update_transform(&mut self) {
        let Some((mins, _)) = self.vertex_bounds() else {
            return;
        };
        for v in &mut self.vertices {
            for (c, m) in v.iter_mut().zip(mins) {
                *c -= m;
            }
        }
        let t = &mut self.transform;
        for ((tr, sc), m) in t.translate.iter_mut().zip(t.scale).zip(mins) {
            *tr += m as f64 * sc;
        }
    }
    pub fn number_of_city_objects(&self) -> usize {
        self.city_objects
            .values()
            .filter(|co| co.is_toplevel())
            .count()
    }
    /// Set the order in which [`CityJSON::get_cjfeature`] hands out features.
    pub fn sort_cjfeatures(&mut self, ss: SortingStrategy) {
        self.sorted_ids = self
            .city_objects
            .iter()
            .filter(|(_, co)| co.is_toplevel())
            .map(|(key, _)| key.clone())
            .collect();
        match ss {
            SortingStrategy::AsIs => {}
            SortingStrategy::Random => {
                use rand::seq::SliceRandom;
                self.sorted_ids.shuffle(&mut rand::thread_rng());
            }
            SortingStrategy::Lexicographical => self.sorted_ids.sort(),
        }
    }
    fn add_co(&mut self, id: String, co: CityObject) {
        self.city_objects.insert(id, co);
    }
    fn add_vertices(&mut self, vs: &mut Vec<Vertex>) {
        if let Some(c) = &self.transform_correction {
            //-- re-express each vertex in this document's transform
            let t = &self.transform;
            for v in vs.iter() {
                let mut out = [0; 3];
                for k in 0..3 {
                    out[k] = (((v[k] as f64 * c.scale[k]) + c.translate[k] - t.translate[k])
                        / t.scale[k])
                        .round() as i64;
                }
                self.vertices.push(out);
            }
        } else {
            self.vertices.append(vs);
        }
    }
    fn add_vertices_texture(&mut self, vs: Vec<[f64; 2]>) {
        self.appearance
            .get_or_insert_with(Appearance::new)
            .add_vertices_texture(vs);
    }
    fn add_material(&mut self, jm: MaterialObject) -> usize {
        self.appearance
            .get_or_insert_with(Appearance::new)
            .add_material(jm)
    }
    fn add_texture(&mut self, jm: TextureObject) -> usize {
        self.appearance
            .get_or_insert_with(Appearance::new)
            .add_texture(jm)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CityJSONFeature {
    #[serde(rename = "type")]
    pub thetype: CityJSONFeatureType,
    pub id: String,
    #[serde(rename = "CityObjects")]
    pub city_objects: IndexMap<String, CityObject>,
    pub vertices: Vec<Vertex>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub appearance: Option<Appearance>,
    /// Root members the schema does not name, kept verbatim.
    #[serde(flatten)]
    pub other: Map<String, Value>,
}

impl Default for CityJSONFeature {
    fn default() -> Self {
        Self::new()
    }
}

impl FromStr for CityJSONFeature {
    type Err = CjseqError;
    /// Parse one CityJSONFeature (one line of a CityJSONSeq).
    fn from_str(s: &str) -> Result<Self> {
        Ok(serde_json::from_str(s)?)
    }
}

impl CityJSONFeature {
    pub fn new() -> Self {
        CityJSONFeature {
            thetype: CityJSONFeatureType::CityJSONFeature,
            id: "".to_string(),
            city_objects: IndexMap::new(),
            vertices: Vec::new(),
            appearance: None,
            other: Map::new(),
        }
    }
    pub fn add_co(&mut self, id: String, co: CityObject) {
        self.city_objects.insert(id, co);
    }
    /// The mean of the feature's vertices, in the integer (untransformed)
    /// coordinates they are stored in; `None` if it has none.
    pub fn centroid(&self) -> Option<[f64; 3]> {
        if self.vertices.is_empty() {
            return None;
        }
        let mut totals = [0.0; 3];
        for v in &self.vertices {
            for k in 0..3 {
                totals[k] += v[k] as f64;
            }
        }
        Some(totals.map(|t| t / self.vertices.len() as f64))
    }
}

/// Collects a base CityJSON metadata and a vector of CityJSONFeatures
/// into a complete CityJSON object
pub fn cjseq_to_cj(mut base_cj: CityJSON, features: Vec<CityJSONFeature>) -> CityJSON {
    for mut feature in features {
        base_cj.add_cjfeature(&mut feature);
    }

    base_cj.remove_duplicate_vertices();
    base_cj.update_transform();
    base_cj.update_geographicalextent();

    base_cj
}

/// The CityJSON versions this crate reads and writes.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CityJSONVersion {
    #[serde(rename = "1.1")]
    V1_1,
    #[serde(rename = "2.0")]
    V2_0,
}

/// One entry of the `extensions` member (§ 8): where the Extension's schema
/// lives and which version of it the document uses.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Extension {
    pub url: String,
    pub version: String,
    /// Members the schema does not name, kept verbatim.
    #[serde(flatten)]
    pub other: Map<String, Value>,
}

/// The `type` member of a [`CityJSON`] object. The spec fixes its value to
/// the single string `"CityJSON"` (§ 1 CityJSON Object); unlike City Object
/// and semantic-surface types, CityJSON does not define an Extension
/// document type, so there is no `Extension` variant here -- a document
/// declaring any other `type` value fails to deserialize.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum CityJSONType {
    CityJSON,
}

/// The `type` member of a [`CityJSONFeature`] object. The spec fixes its
/// value to the single string `"CityJSONFeature"` (§ 7.2 Text sequences and
/// streaming with CityJSONFeature); as with [`CityJSONType`], there is no
/// Extension variant.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum CityJSONFeatureType {
    CityJSONFeature,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every public type in this crate derives `PartialEq`; `CityJSON` and
    /// `CityJSONFeature` were missing it.
    #[test]
    fn cityjson_feature_with_equal_fields_is_equal() {
        let input = serde_json::json!({
            "type": "CityJSONFeature",
            "id": "id-1",
            "CityObjects": {},
            "vertices": []
        });
        let a: CityJSONFeature = serde_json::from_value(input.clone()).unwrap();
        let b: CityJSONFeature = serde_json::from_value(input).unwrap();
        assert_eq!(a, b);
        let mut c = b.clone();
        c.id = "id-2".to_string();
        assert_ne!(a, c);
    }

    #[test]
    fn cityjson_with_equal_fields_is_equal() {
        let a = CityJSON::new();
        let b = CityJSON::new();
        assert_eq!(a, b);
        let mut c = CityJSON::new();
        c.version = CityJSONVersion::V1_1;
        assert_ne!(a, c);
    }

    /// `sorted_ids` and `transform_correction` are `#[serde(skip)]` internal
    /// bookkeeping, not part of the document: `sorted_ids` is populated by
    /// `from_str` iterating `city_objects` (a `HashMap`, whose iteration
    /// order is not guaranteed stable across independently-constructed
    /// maps), and `sort_cjfeatures(Random)` shuffles it further. Two
    /// `CityJSON`s parsed from the identical string represent the identical
    /// document and must compare equal regardless of what order that
    /// bookkeeping happened to land in. A fixture with a single top-level
    /// CityObject cannot exercise this -- `sorted_ids` would trivially have
    /// one, unambiguous order either way -- so this uses five.
    #[test]
    fn cityjson_parsed_twice_from_the_same_string_is_equal_regardless_of_hashmap_order() {
        let input = serde_json::json!({
            "type": "CityJSON",
            "version": "2.0",
            "transform": {"scale": [1.0, 1.0, 1.0], "translate": [0.0, 0.0, 0.0]},
            "CityObjects": {
                "id-a": {"type": "Building"},
                "id-b": {"type": "Building"},
                "id-c": {"type": "Building"},
                "id-d": {"type": "Building"},
                "id-e": {"type": "Building"}
            },
            "vertices": []
        })
        .to_string();
        let a = CityJSON::from_str(&input).unwrap();
        let b = CityJSON::from_str(&input).unwrap();
        assert_eq!(
            a, b,
            "the same document parsed twice must be equal, independent of internal \
             bookkeeping order"
        );
    }

    #[test]
    fn cityjson_type_round_trips() {
        let t: CityJSONType = serde_json::from_value(serde_json::json!("CityJSON")).unwrap();
        assert_eq!(t, CityJSONType::CityJSON);
        assert_eq!(
            serde_json::to_value(&t).unwrap(),
            serde_json::json!("CityJSON")
        );
    }

    #[test]
    fn cityjson_type_rejects_any_other_value() {
        assert!(
            serde_json::from_value::<CityJSONType>(serde_json::json!("CityJSONFeature")).is_err()
        );
        assert!(serde_json::from_value::<CityJSONType>(serde_json::json!("+CityJSON")).is_err());
    }

    #[test]
    fn cityjson_feature_type_round_trips() {
        let t: CityJSONFeatureType =
            serde_json::from_value(serde_json::json!("CityJSONFeature")).unwrap();
        assert_eq!(t, CityJSONFeatureType::CityJSONFeature);
        assert_eq!(
            serde_json::to_value(&t).unwrap(),
            serde_json::json!("CityJSONFeature")
        );
    }

    #[test]
    fn cityjson_feature_type_rejects_any_other_value() {
        assert!(
            serde_json::from_value::<CityJSONFeatureType>(serde_json::json!("CityJSON")).is_err()
        );
    }
}
