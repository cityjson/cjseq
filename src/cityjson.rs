use crate::appearance::Appearance;
use crate::city_object::CityObject;
use crate::error::{CjseqError, Result};
use crate::geometry::GeometryTemplates;
use crate::index::{Index, MaterialIndex, TextureIndex, UvIndex, VertexIndex};
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
fn reindex<T: Clone, I: Index>(all: &[T], oldnew: &HashMap<I, I>) -> Result<Vec<T>> {
    let mut pairs: Vec<(usize, usize)> = oldnew.iter().map(|(o, n)| (n.get(), o.get())).collect();
    pairs.sort_unstable();
    pairs
        .into_iter()
        .map(|(_, old)| {
            all.get(old).cloned().ok_or(CjseqError::IndexOutOfRange {
                kind: I::KIND,
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
    m_oldnew: &HashMap<MaterialIndex, MaterialIndex>,
    t_oldnew: &HashMap<TextureIndex, TextureIndex>,
    t_v_oldnew: &HashMap<UvIndex, UvIndex>,
) -> Result<Appearance> {
    let mut a = Appearance::new();
    a.default_theme_material = all.default_theme_material.clone();
    a.default_theme_texture = all.default_theme_texture.clone();
    if let Some(m) = &all.materials {
        a.materials = Some(reindex(m, m_oldnew)?);
    }
    if let Some(t) = &all.textures {
        a.textures = Some(reindex(t, t_oldnew)?);
    }
    if let Some(tv) = &all.vertices_texture {
        a.vertices_texture = Some(reindex(tv, t_v_oldnew)?);
    }
    Ok(a)
}

/// The order in which [`CityJSON::features`] hands out features.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortingStrategy {
    /// The order the top-level City Objects appear in the document.
    AsIs,
    /// A random permutation, different on every call.
    Random,
    /// Sorted by City Object id.
    Lexicographical,
}

/// A CityJSON document. It holds the document and nothing else; splitting it
/// into a CityJSONSeq is [`CityJSON::header`] and [`CityJSON::features`], and
/// the reverse is [`crate::Collector`].
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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
}

impl Default for CityJSON {
    fn default() -> Self {
        Self::new()
    }
}

impl FromStr for CityJSON {
    type Err = CjseqError;
    /// Parse a CityJSON document.
    fn from_str(s: &str) -> Result<Self> {
        Ok(serde_json::from_str(s)?)
    }
}

impl CityJSON {
    /// An empty CityJSON 2.0 document with the identity transform.
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
        }
    }

    /// The first line of a CityJSONSeq: the document without its City
    /// Objects and vertices, its appearance cut down to what the geometry
    /// templates use. An appearance index in a template that is out of range
    /// is an error.
    pub fn header(&self) -> Result<Self> {
        let mut cj0 = CityJSON {
            city_objects: IndexMap::new(),
            vertices: Vec::new(),
            appearance: None,
            ..self.clone_without_objects()
        };
        if let Some(x) = &self.geometry_templates {
            let mut gts2: GeometryTemplates = x.clone();
            let mut m_oldnew: HashMap<MaterialIndex, MaterialIndex> = HashMap::new();
            let mut t_oldnew: HashMap<TextureIndex, TextureIndex> = HashMap::new();
            let mut t_v_oldnew: HashMap<UvIndex, UvIndex> = HashMap::new();
            for g in &mut gts2.templates {
                g.update_material(&mut m_oldnew);
                g.update_texture(&mut t_oldnew, &mut t_v_oldnew);
            }
            if let Some(a) = &self.appearance {
                cj0.appearance = Some(slice_appearance(a, &m_oldnew, &t_oldnew, &t_v_oldnew)?);
            }
            //-- the templates now index the sliced appearance
            cj0.geometry_templates = Some(gts2);
        }
        Ok(cj0)
    }

    /// Every member but the City Objects, vertices and appearance, which
    /// `header` rebuilds.
    fn clone_without_objects(&self) -> Self {
        CityJSON {
            thetype: self.thetype.clone(),
            version: self.version,
            transform: self.transform.clone(),
            city_objects: IndexMap::new(),
            vertices: Vec::new(),
            metadata: self.metadata.clone(),
            appearance: None,
            geometry_templates: self.geometry_templates.clone(),
            extensions: self.extensions.clone(),
            other: self.other.clone(),
        }
    }

    /// The ids of the top-level City Objects, in `order`.
    fn feature_ids(&self, order: SortingStrategy) -> Vec<&String> {
        let mut ids: Vec<&String> = self
            .city_objects
            .iter()
            .filter(|(_, co)| co.is_toplevel())
            .map(|(id, _)| id)
            .collect();
        match order {
            SortingStrategy::AsIs => {}
            SortingStrategy::Random => {
                use rand::seq::SliceRandom;
                ids.shuffle(&mut rand::thread_rng());
            }
            SortingStrategy::Lexicographical => ids.sort(),
        }
        ids
    }

    /// The document's features, one per top-level City Object, in `order`;
    /// see [`CityJSON::feature`].
    pub fn features(
        &self,
        order: SortingStrategy,
    ) -> impl Iterator<Item = Result<CityJSONFeature>> + '_ {
        self.feature_ids(order)
            .into_iter()
            .map(move |id| self.slice(id, &self.city_objects[id]))
    }

    /// The feature rooted at City Object `id`: it with all of its
    /// descendants, and only the vertices and appearance entries they use,
    /// renumbered. `Ok(None)` if there is no such City Object.
    ///
    /// A child id that names no City Object, or an index past the end of
    /// `vertices` or of an appearance palette, is an error.
    pub fn feature(&self, id: &str) -> Result<Option<CityJSONFeature>> {
        match self.city_objects.get_key_value(id) {
            Some((id, root)) => self.slice(id, root).map(Some),
            None => Ok(None),
        }
    }

    fn slice(&self, id: &String, root: &CityObject) -> Result<CityJSONFeature> {
        //-- the root, then every descendant breadth-first; `seen` stops a
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
        let mut g_vi_oldnew: HashMap<VertexIndex, VertexIndex> = HashMap::new();
        let mut m_oldnew: HashMap<MaterialIndex, MaterialIndex> = HashMap::new();
        let mut t_oldnew: HashMap<TextureIndex, TextureIndex> = HashMap::new();
        let mut t_v_oldnew: HashMap<UvIndex, UvIndex> = HashMap::new();
        for (key, co) in members {
            let mut co2 = co.clone();
            for g in co2.geometry.iter_mut().flatten() {
                g.update_geometry_boundaries(&mut g_vi_oldnew);
                g.update_material(&mut m_oldnew);
                g.update_texture(&mut t_oldnew, &mut t_v_oldnew);
            }
            cjf.add_co(key.clone(), co2);
        }
        cjf.vertices = reindex(&self.vertices, &g_vi_oldnew)?;
        if let Some(a) = &self.appearance {
            cjf.appearance = Some(slice_appearance(a, &m_oldnew, &t_oldnew, &t_v_oldnew)?);
        }
        Ok(cjf)
    }

    /// Merge identical vertices, renumbering every geometry to match.
    pub fn remove_duplicate_vertices(&mut self) {
        let mut h: HashMap<Vertex, usize> = HashMap::new();
        let mut newids: HashMap<VertexIndex, VertexIndex> = HashMap::new();
        let mut newvertices: Vec<Vertex> = Vec::new();
        for (i, v) in self.vertices.iter().enumerate() {
            let new = *h.entry(*v).or_insert_with(|| {
                newvertices.push(*v);
                newvertices.len() - 1
            });
            newids.insert(VertexIndex(i), VertexIndex(new));
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
    /// The number of top-level City Objects, ie of features.
    pub fn number_of_city_objects(&self) -> usize {
        self.city_objects
            .values()
            .filter(|co| co.is_toplevel())
            .count()
    }
}

/// Collects a CityJSONSeq back into one CityJSON document: start from the
/// header, [`Collector::add`] each feature, then [`Collector::finish`].
///
/// Features from several sequences can be collected into one document: hand
/// each further sequence's header to [`Collector::add_header`], and the
/// features that follow are re-expressed in the first header's transform.
#[derive(Debug, Clone)]
pub struct Collector {
    doc: CityJSON,
    /// The transform the features being added are expressed in, when it is
    /// not the document's.
    correction: Option<Transform>,
}

impl Collector {
    pub fn new(header: CityJSON) -> Self {
        Collector {
            doc: header,
            correction: None,
        }
    }

    /// The header of a further sequence: the features added after it are in
    /// its transform.
    pub fn add_header(&mut self, header: CityJSON) -> Result<()> {
        self.correction = (header.transform != self.doc.transform).then_some(header.transform);
        Ok(())
    }

    /// Add one feature: its City Objects, its vertices (re-expressed if a
    /// later header changed the transform) and its appearance, merged into
    /// the document's palettes.
    pub fn add(&mut self, mut cjf: CityJSONFeature) -> Result<()> {
        if !cjf.city_objects.contains_key(&cjf.id) {
            return Err(CjseqError::DanglingReference {
                from: "CityJSONFeature.id".to_string(),
                to: cjf.id,
            });
        }
        let doc = &mut self.doc;
        let mut m_oldnew: HashMap<MaterialIndex, MaterialIndex> = HashMap::new();
        let mut t_oldnew: HashMap<TextureIndex, TextureIndex> = HashMap::new();
        let mut t_v_oldnew: HashMap<UvIndex, UvIndex> = HashMap::new();
        let g_offset = doc.vertices.len();
        if let Some(cjf_app) = cjf.appearance.take() {
            let app = doc.appearance.get_or_insert_with(Appearance::new);
            for (i, m) in cjf_app.materials.into_iter().flatten().enumerate() {
                m_oldnew.insert(MaterialIndex(i), MaterialIndex(app.add_material(m)));
            }
            for (i, t) in cjf_app.textures.into_iter().flatten().enumerate() {
                t_oldnew.insert(TextureIndex(i), TextureIndex(app.add_texture(t)));
            }
            if let Some(uvs) = cjf_app.vertices_texture {
                //-- the feature's texture vertices are appended verbatim, so
                //-- each one's new index is its old one shifted by however
                //-- many were already collected
                let t_v_offset = app.vertices_texture.as_ref().map_or(0, Vec::len);
                for i in 0..uvs.len() {
                    t_v_oldnew.insert(UvIndex(i), UvIndex(t_v_offset + i));
                }
                app.add_vertices_texture(uvs);
            }
        }
        for (key, mut co) in cjf.city_objects {
            for g in co.geometry.iter_mut().flatten() {
                g.offset_geometry_boundaries(g_offset);
                g.update_material(&mut m_oldnew);
                g.update_texture(&mut t_oldnew, &mut t_v_oldnew);
            }
            doc.city_objects.insert(key, co);
        }
        match &self.correction {
            None => doc.vertices.extend(cjf.vertices),
            Some(c) => {
                //-- re-express each vertex in the document's transform
                let t = &doc.transform;
                doc.vertices.extend(cjf.vertices.iter().map(|v| {
                    [0, 1, 2].map(|k| {
                        (((v[k] as f64 * c.scale[k]) + c.translate[k] - t.translate[k])
                            / t.scale[k])
                            .round() as i64
                    })
                }));
            }
        }
        Ok(())
    }

    /// The collected document, with duplicate vertices merged, the transform
    /// moved to the vertices' minimum corner and the geographical extent
    /// recomputed.
    pub fn finish(mut self) -> CityJSON {
        self.doc.remove_duplicate_vertices();
        self.doc.update_transform();
        self.doc.update_geographicalextent();
        self.doc
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

/// Collect a header and its features into one CityJSON document; see
/// [`Collector`].
pub fn cjseq_to_cj(header: CityJSON, features: Vec<CityJSONFeature>) -> Result<CityJSON> {
    let mut c = Collector::new(header);
    for f in features {
        c.add(f)?;
    }
    Ok(c.finish())
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
