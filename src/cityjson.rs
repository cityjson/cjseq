use crate::appearance::{Appearance, MaterialObject, TextureObject};
use crate::city_object::CityObject;
use crate::error::{CjseqError, Result};
use crate::geometry::GeometryTemplates;
use crate::metadata::{Metadata, Transform};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::str::FromStr;

/// Slice a document-level appearance palette down to the entries one feature
/// uses, in the order that feature now refers to them.
///
/// `oldnew` maps each index used by the feature onto its dense position in the
/// new palette; `update_material`/`update_texture` build it by handing out
/// consecutive new indices, so its values are exactly `0..oldnew.len()` and
/// sorting by them reproduces the palette.
///
/// This replaces a `resize(len, json!(null))`-then-overwrite loop, which only
/// worked because every slot was subsequently overwritten: the `null` filler
/// was never a valid palette entry, and now that the palette is typed it is
/// not even representable.
fn reindex<T: Clone>(all: &[T], oldnew: &HashMap<usize, usize>) -> Vec<T> {
    let mut pairs: Vec<(usize, usize)> = oldnew.iter().map(|(o, n)| (*n, *o)).collect();
    pairs.sort_unstable();
    pairs.into_iter().map(|(_, old)| all[old].clone()).collect()
}

#[derive(Clone)]
pub enum SortingStrategy {
    Random,
    Lexicographical,
    Morton,  //-- TODO implement Morton sorting
    Hilbert, //-- TODO implement Hilbert sorting
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CityJSON {
    #[serde(rename = "type")]
    pub thetype: CityJSONType,
    pub version: String,
    pub transform: Transform,
    #[serde(rename = "CityObjects")]
    pub city_objects: HashMap<String, CityObject>,
    pub vertices: Vec<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub appearance: Option<Appearance>,
    #[serde(rename = "geometry-templates")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geometry_templates: Option<GeometryTemplates>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Value>,
    #[serde(flatten)]
    pub other: serde_json::Value,
    #[serde(skip)]
    sorted_ids: Vec<String>,
    #[serde(skip)]
    transform_correction: Option<Transform>,
}

/// Hand-written rather than derived: `sorted_ids` and `transform_correction`
/// are `#[serde(skip)]` internal bookkeeping, not part of the CityJSON
/// document. `sorted_ids` in particular is populated by iterating
/// `city_objects` (a `HashMap`, whose iteration order is not guaranteed
/// stable across independently-constructed maps) and can be further
/// reordered by `sort_cjfeatures(SortingStrategy::Random)`, so a derived
/// `PartialEq` that included it would make `==` spuriously `false` for two
/// values representing the identical document. Excluding it here is what
/// makes equality mean "same document", which is the only sense of equality
/// this crate (or its callers) should ever want. This does not violate the
/// "never hand-write" constraint, which is about `Serialize`/`Deserialize`
/// -- a hand-written `PartialEq` cannot corrupt output.
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
    /// Parse a CityJSON document.
    fn from_str(s: &str) -> Result<Self> {
        let mut cjj: CityJSON = serde_json::from_str(s)?;
        //-- check if CO exists, then add them to the sorted_ids
        for (key, co) in &cjj.city_objects {
            if co.is_toplevel() {
                cjj.sorted_ids.push(key.clone());
            }
        }
        Ok(cjj)
    }
}

impl CityJSON {
    /// Create a new CityJSON instance with default values.
    pub fn new() -> Self {
        let co: HashMap<String, CityObject> = HashMap::new();
        let v: Vec<Vec<i64>> = Vec::new();
        let tr = Transform::new();
        CityJSON {
            thetype: CityJSONType::CityJSON,
            version: "2.0".to_string(),
            transform: tr,
            city_objects: co,
            vertices: v,
            metadata: None,
            appearance: None,
            geometry_templates: None,
            extensions: None,
            other: json!(null),
            sorted_ids: vec![],
            transform_correction: None,
        }
    }
    /// Get the "first line" (aka metadata or header) of a CityJSONSeq
    pub fn get_metadata(&self) -> Self {
        let co: HashMap<String, CityObject> = HashMap::new();
        let v: Vec<Vec<i64>> = Vec::new();
        let mut cj0 = CityJSON {
            thetype: self.thetype.clone(),
            version: self.version.clone(),
            transform: self.transform.clone(),
            metadata: self.metadata.clone(),
            city_objects: co,
            vertices: v,
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
                g.update_texture(&mut t_oldnew, &mut t_v_oldnew, 0);
            }
            //-- "slice" materials
            if let Some(a) = &self.appearance {
                let mut acjf: Appearance = Appearance::new();
                acjf.default_theme_material = a.default_theme_material.clone();
                acjf.default_theme_texture = a.default_theme_texture.clone();
                if let Some(am) = a.materials.as_ref() {
                    acjf.materials = Some(reindex(am, &m_oldnew));
                }
                if let Some(at) = a.textures.as_ref() {
                    acjf.textures = Some(reindex(at, &t_oldnew));
                }
                if let Some(atv) = a.vertices_texture.as_ref() {
                    acjf.vertices_texture = Some(reindex(atv, &t_v_oldnew));
                }
                cj0.appearance = Some(acjf);
            }
        }
        cj0
    }
    /// Getter for the features in a CityJSON dataset.
    /// Starts at 0, and return Option::None if the index is out of bounds.
    pub fn get_cjfeature(&self, i: usize) -> Option<CityJSONFeature> {
        let i2 = self.sorted_ids.get(i);
        i2?;
        let obj = self.city_objects.get(i2.unwrap());
        obj?;
        let co = obj.unwrap();
        //-- the other lines
        let mut cjf = CityJSONFeature::new();
        let mut co2: CityObject = co.clone();
        let mut g_vi_oldnew: HashMap<usize, usize> = HashMap::new();
        let mut m_oldnew: HashMap<usize, usize> = HashMap::new();
        let mut t_oldnew: HashMap<usize, usize> = HashMap::new();
        let mut t_v_oldnew: HashMap<usize, usize> = HashMap::new();
        if let Some(x) = &mut co2.geometry {
            for g in x.iter_mut() {
                g.update_geometry_boundaries(&mut g_vi_oldnew);
                g.update_material(&mut m_oldnew);
                g.update_texture(&mut t_oldnew, &mut t_v_oldnew, 0);
            }
        }
        cjf.add_co(self.sorted_ids[i].clone(), co2);
        cjf.id = self.sorted_ids[i].to_string();
        //-- TODO: to fix: children-of-children?
        //-- process all the children (only one-level lower)
        for childkey in co.get_children_keys() {
            let coc = self.city_objects.get(&childkey).unwrap();
            let mut coc2: CityObject = coc.clone();
            if let Some(x) = &mut coc2.geometry {
                for g in x.iter_mut() {
                    g.update_geometry_boundaries(&mut g_vi_oldnew);
                    g.update_material(&mut m_oldnew);
                    g.update_texture(&mut t_oldnew, &mut t_v_oldnew, 0);
                }
            }
            cjf.add_co(childkey.clone(), coc2);
        }
        //-- "slice" geometry vertices
        let allvertices = &self.vertices;
        let mut g_new_vertices: Vec<Vec<i64>> = Vec::new();
        g_new_vertices.resize(g_vi_oldnew.len(), vec![]);
        for (old, new) in &g_vi_oldnew {
            g_new_vertices[*new] = allvertices[*old].clone();
        }
        cjf.vertices = g_new_vertices;
        //-- "slice" materials
        if let Some(a) = &self.appearance {
            let mut acjf: Appearance = Appearance::new();
            acjf.default_theme_material = a.default_theme_material.clone();
            acjf.default_theme_texture = a.default_theme_texture.clone();
            if let Some(am) = a.materials.as_ref() {
                acjf.materials = Some(reindex(am, &m_oldnew));
            }
            if let Some(at) = a.textures.as_ref() {
                acjf.textures = Some(reindex(at, &t_oldnew));
            }
            if let Some(atv) = a.vertices_texture.as_ref() {
                acjf.vertices_texture = Some(reindex(atv, &t_v_oldnew));
            }
            cjf.appearance = Some(acjf);
        }
        Some(cjf)
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
        let mut t_offset = 0;
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
                t_offset = cjf_v_tex.len();
                self.add_vertices_texture(cjf_v_tex.clone());
            }
        }

        for (key, co) in &mut cjf.city_objects {
            //-- boundaries
            if let Some(ref mut geoms) = &mut co.geometry {
                for g in geoms.iter_mut() {
                    //-- boundaries
                    g.offset_geometry_boundaries(g_offset);
                    // g.update_geometry_boundaries(&mut g_oldnew, g_offset);
                    //-- material
                    g.update_material(&mut m_oldnew);
                    //-- texture
                    g.update_texture(&mut t_oldnew, &mut t_v_oldnew, t_offset);
                }
            }
            //-- update the collected json object by adding the CityObjects
            self.add_co(key.to_string(), co.clone());
        }
        //-- add the new vertices
        self.add_vertices(&mut cjf.vertices);
        //-- add the CO id to the list
        self.sorted_ids.push(cjf.id.clone());
    }
    pub fn remove_duplicate_vertices(&mut self) {
        // let totalinput = self.vertices.len();
        let mut h: HashMap<String, usize> = HashMap::new();
        let mut newids: HashMap<usize, usize> = HashMap::new();
        let mut newvertices: Vec<Vec<i64>> = Vec::new();
        for (i, v) in self.vertices.iter().enumerate() {
            // println!("{:?}", v);
            let k = format!("{} {} {}", v[0], v[1], v[2]);
            match h.get(&k) {
                Some(x) => {
                    let _ = newids.insert(i, *x);
                }
                None => {
                    newids.insert(i, newvertices.len());
                    h.insert(k.clone(), newvertices.len());
                    newvertices.push(v.clone());
                }
            }
        }
        //-- update indices
        let cos = &mut self.city_objects;
        for (_key, co) in cos.iter_mut() {
            if let Some(x) = &mut co.geometry {
                for g in x.iter_mut() {
                    g.update_geometry_boundaries(&mut newids);
                }
            }
        }
        //-- replace the vertices, innit?
        self.vertices = newvertices;
    }
    pub fn update_geographicalextent(&mut self) {
        if let Some(m) = &mut self.metadata {
            if let Some(ref mut ge) = m.geographical_extent {
                let mut mins: Vec<i64> = vec![i64::MAX, i64::MAX, i64::MAX];
                let mut maxs: Vec<i64> = vec![i64::MIN, i64::MIN, i64::MIN];
                for v in &self.vertices {
                    for i in 0..3 {
                        if v[i] < mins[i] {
                            mins[i] = v[i];
                        }
                        if v[i] > maxs[i] {
                            maxs[i] = v[i];
                        }
                    }
                }
                *ge = [
                    mins[0] as f64 * self.transform.scale[0] + self.transform.translate[0],
                    mins[1] as f64 * self.transform.scale[1] + self.transform.translate[1],
                    mins[2] as f64 * self.transform.scale[2] + self.transform.translate[2],
                    maxs[0] as f64 * self.transform.scale[0] + self.transform.translate[0],
                    maxs[1] as f64 * self.transform.scale[1] + self.transform.translate[1],
                    maxs[2] as f64 * self.transform.scale[2] + self.transform.translate[2],
                ];
            }
        }
    }
    pub fn update_transform(&mut self) {
        let mut newvertices: Vec<Vec<i64>> = Vec::new();
        let mut mins: Vec<i64> = vec![i64::MAX, i64::MAX, i64::MAX];
        //-- find min-xyz
        for v in &self.vertices {
            for i in 0..3 {
                if v[i] < mins[i] {
                    mins[i] = v[i];
                }
            }
        }
        //-- subtract the mins from each vertex
        for v in &self.vertices {
            let v: Vec<i64> = vec![v[0] - mins[0], v[1] - mins[1], v[2] - mins[2]];
            newvertices.push(v);
        }
        //-- replace the vertices, innit?
        self.vertices = newvertices;
        //-- update the transform/translate
        let ttx = (mins[0] as f64 * self.transform.scale[0]) + self.transform.translate[0];
        let tty = (mins[1] as f64 * self.transform.scale[1]) + self.transform.translate[1];
        let ttz = (mins[2] as f64 * self.transform.scale[2]) + self.transform.translate[2];
        self.transform.translate = vec![ttx, tty, ttz];
    }
    pub fn number_of_city_objects(&self) -> usize {
        let mut total: usize = 0;
        for co in self.city_objects.values() {
            if co.is_toplevel() {
                total += 1;
            }
        }
        total
    }
    /// When getting the CityJSONFeatures, this controls the order in which
    /// they are returned. By default they are returned in the order they were added.
    pub fn sort_cjfeatures(&mut self, ss: SortingStrategy) {
        self.sorted_ids.clear();
        match ss {
            SortingStrategy::Random => {
                for (key, co) in &self.city_objects {
                    if co.is_toplevel() {
                        self.sorted_ids.push(key.clone());
                    }
                }
            }
            SortingStrategy::Lexicographical => {
                for (key, co) in &self.city_objects {
                    if co.is_toplevel() {
                        self.sorted_ids.push(key.clone());
                    }
                }
                self.sorted_ids.sort();
            }
            _ => todo!(),
        }
    }
    fn add_co(&mut self, id: String, co: CityObject) {
        self.city_objects.insert(id.clone(), co);
    }
    fn add_vertices(&mut self, vs: &mut Vec<Vec<i64>>) {
        if let Some(c) = &self.transform_correction {
            //-- the transform correction needs to be applied
            for v in vs {
                let cx: i64 = (((v[0] as f64 * c.scale[0]) + c.translate[0]
                    - self.transform.translate[0])
                    / self.transform.scale[0])
                    .round() as i64;
                let cy: i64 = (((v[1] as f64 * c.scale[1]) + c.translate[1]
                    - self.transform.translate[1])
                    / self.transform.scale[1])
                    .round() as i64;
                let cz: i64 = (((v[2] as f64 * c.scale[2]) + c.translate[2]
                    - self.transform.translate[2])
                    / self.transform.scale[2])
                    .round() as i64;
                self.vertices.push(vec![cx, cy, cz]);
            }
        } else {
            self.vertices.append(vs);
        }
    }
    fn add_vertices_texture(&mut self, vs: Vec<[f64; 2]>) {
        match &mut self.appearance {
            Some(x) => x.add_vertices_texture(vs),
            None => {
                let mut a: Appearance = Appearance::new();
                a.add_vertices_texture(vs);
                self.appearance = Some(a);
            }
        };
    }
    fn add_material(&mut self, jm: MaterialObject) -> usize {
        match &mut self.appearance {
            Some(x) => x.add_material(jm),
            None => {
                let mut a: Appearance = Appearance::new();
                let re = a.add_material(jm);
                self.appearance = Some(a);
                re
            }
        }
    }
    fn add_texture(&mut self, jm: TextureObject) -> usize {
        match &mut self.appearance {
            Some(x) => x.add_texture(jm),
            None => {
                let mut a: Appearance = Appearance::new();
                let re = a.add_texture(jm);
                self.appearance = Some(a);
                re
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CityJSONFeature {
    #[serde(rename = "type")]
    pub thetype: CityJSONFeatureType,
    pub id: String,
    #[serde(rename = "CityObjects")]
    pub city_objects: HashMap<String, CityObject>,
    pub vertices: Vec<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub appearance: Option<Appearance>,
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
        let co: HashMap<String, CityObject> = HashMap::new();
        let v: Vec<Vec<i64>> = Vec::new();
        CityJSONFeature {
            thetype: CityJSONFeatureType::CityJSONFeature,
            id: "".to_string(),
            city_objects: co,
            vertices: v,
            appearance: None,
        }
    }
    pub fn add_co(&mut self, id: String, co: CityObject) {
        self.city_objects.insert(id, co);
    }
    pub fn centroid(&self) -> Vec<f64> {
        let mut totals: Vec<f64> = vec![0., 0., 0.];
        for v in &self.vertices {
            for (t, c) in totals.iter_mut().zip(v) {
                *t += *c as f64;
            }
        }
        for t in &mut totals {
            *t /= self.vertices.len() as f64;
        }
        totals
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
        c.version = "1.1".to_string();
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
