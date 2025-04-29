/*!
# CityJSONSeq

Create, process, modify, and convert CityJSONSeq files.

## Cross-platform Support

This crate supports both native platforms and WebAssembly (WASM):

- **Native platforms**: Uses `reqwest` for HTTP requests.
- **WASM targets**: Uses `gloo-net` for HTTP requests.

When compiling for WASM, note that some functionality works differently:

- The synchronous `fetch_from_url` method returns a placeholder in WASM environments.
- Use the `fetch_from_url_async` method when working with WASM targets.

## Example

```rust,no_run
use cjseq::{CityJSON, CityJSONFeature, Extension, ExtensionFile};
use std::collections::HashMap;

// To process a CityJSON file:
let json_str = std::fs::read_to_string("path/to/file.json").unwrap();
let city_json = CityJSON::from_str(&json_str).unwrap();

// To fetch extension data (native platforms):
#[cfg(not(target_arch = "wasm32"))]
{
    // Make sure to create a long-lived reference to the extension
    let extensions = &city_json.extensions.unwrap();
    let extension = extensions.get("my_extension").unwrap();
    let extension_file = extension.fetch_extension_file("MyExt".to_string()).unwrap();
}

// To fetch extension data (WASM):
#[cfg(target_arch = "wasm32")]
async {
    if let Some(extensions) = &city_json.extensions {
        if let Some(extension) = extensions.get("my_extension") {
            let extension_file = ExtensionFile::fetch_from_url_async(
                "MyExt".to_string(),
                extension.url.clone(),
                extension.version.clone()
            ).await.unwrap();
        }
    }
}
```
*/

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Number, Value};
use std::collections::HashMap;

// Re-export the error module
pub mod error;
pub use error::*;

const DEFAULT_CRS_BASE_URL: &str = "https://www.opengis.net/def/crs";

#[derive(Clone)]
pub enum SortingStrategy {
    Random,
    Alphabetical,
    Morton, //-- TODO implement Morton sorting
    Hilbert,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CityJSON {
    #[serde(rename = "type")]
    pub thetype: String,
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
    pub extensions: Option<HashMap<String, Extension>>,
    #[serde(flatten)]
    pub other: serde_json::Value,
    #[serde(skip)]
    sorted_ids: Vec<String>,
}
impl CityJSON {
    pub fn new() -> Self {
        let co: HashMap<String, CityObject> = HashMap::new();
        let v: Vec<Vec<i64>> = Vec::new();
        let tr = Transform::new();
        CityJSON {
            thetype: "CityJSON".to_string(),
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
        }
    }
    pub fn from_str(s: &str) -> Result<Self> {
        let mut raw_value: serde_json::Value = serde_json::from_str(s)?;

        // Handle extensions - support both old Value-typed extensions and new HashMap<String, Extension>
        if let Some(extensions_value) = raw_value.get_mut("extensions") {
            if !extensions_value.is_object() {
                // If extensions isn't an object, set it to null so it's skipped
                *extensions_value = json!(null);
            }
        }

        let mut cjj: CityJSON = serde_json::from_value(raw_value)?;

        //-- check if CO exists, then add them to the sorted_ids
        for (key, co) in &cjj.city_objects {
            if co.is_toplevel() {
                cjj.sorted_ids.push(key.clone());
            }
        }
        Ok(cjj)
    }
    pub fn get_metadata(&self) -> Self {
        //-- first line: the CityJSON "metadata"
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
        };
        //-- if geometry-templates have material/textures then these need to be
        //-- added to 1st line (metadata)
        match &self.geometry_templates {
            Some(x) => {
                let mut gts2: GeometryTemplates = x.clone();
                let mut m_oldnew: HashMap<usize, usize> = HashMap::new();
                let mut t_oldnew: HashMap<usize, usize> = HashMap::new();
                let mut t_v_oldnew: HashMap<usize, usize> = HashMap::new();
                for g in &mut gts2.templates {
                    g.update_material(&mut m_oldnew);
                    g.update_texture(&mut t_oldnew, &mut t_v_oldnew, 0);
                }
                //-- "slice" materials
                if self.appearance.is_some() {
                    let a = self.appearance.as_ref().unwrap();
                    let mut acjf: Appearance = Appearance::new();
                    acjf.default_theme_material = a.default_theme_material.clone();
                    acjf.default_theme_texture = a.default_theme_texture.clone();
                    if a.materials.is_some() {
                        let am = a.materials.as_ref().unwrap();
                        let mut mats2: Vec<MaterialObject> = Vec::new();
                        mats2.resize_with(m_oldnew.len(), Default::default);
                        for (old, new) in &m_oldnew {
                            mats2[*new] = am[*old].clone();
                        }
                        acjf.materials = Some(mats2);
                    }
                    if a.textures.is_some() {
                        let at = a.textures.as_ref().unwrap();
                        let mut texs2: Vec<TextureObject> = Vec::new();
                        texs2.resize(t_oldnew.len(), Default::default());
                        for (old, new) in &t_oldnew {
                            texs2[*new] = at[*old].clone();
                        }
                        acjf.textures = Some(texs2);
                    }
                    if a.vertices_texture.is_some() {
                        let atv = a.vertices_texture.as_ref().unwrap();
                        let mut t_new_vertices: Vec<[f64; 2]> = Vec::new();
                        t_new_vertices.resize(t_v_oldnew.len(), [0.0, 0.0]);
                        for (old, new) in &t_v_oldnew {
                            t_new_vertices[*new] = atv[*old].clone();
                        }
                        acjf.vertices_texture = Some(t_new_vertices);
                    }
                    cj0.appearance = Some(acjf);
                }
            }
            None => (),
        }
        cj0
    }
    pub fn get_cjfeature(&self, i: usize) -> Option<CityJSONFeature> {
        let i2 = self.sorted_ids.get(i);
        if i2.is_none() {
            return None;
        }
        let obj = self.city_objects.get(i2.unwrap());
        if obj.is_none() {
            return None;
        }
        let co = obj.unwrap();
        //-- the other lines
        let mut cjf = CityJSONFeature::new();
        let mut co2: CityObject = co.clone();
        let mut g_vi_oldnew: HashMap<usize, usize> = HashMap::new();
        let mut m_oldnew: HashMap<usize, usize> = HashMap::new();
        let mut t_oldnew: HashMap<usize, usize> = HashMap::new();
        let mut t_v_oldnew: HashMap<usize, usize> = HashMap::new();
        match &mut co2.geometry {
            Some(x) => {
                for g in x.iter_mut() {
                    g.update_geometry_boundaries(&mut g_vi_oldnew);
                    g.update_material(&mut m_oldnew);
                    g.update_texture(&mut t_oldnew, &mut t_v_oldnew, 0);
                }
            }
            None => (),
        }
        cjf.add_co(self.sorted_ids[i].clone(), co2);
        cjf.id = self.sorted_ids[i].to_string();
        //-- TODO: to fix: children-of-children?
        //-- process all the children (only one-level lower)
        for childkey in co.get_children_keys() {
            let coc = self.city_objects.get(&childkey).unwrap();
            let mut coc2: CityObject = coc.clone();
            match &mut coc2.geometry {
                Some(x) => {
                    for g in x.iter_mut() {
                        g.update_geometry_boundaries(&mut g_vi_oldnew);
                        g.update_material(&mut m_oldnew);
                        g.update_texture(&mut t_oldnew, &mut t_v_oldnew, 0);
                    }
                }
                None => (),
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
        if self.appearance.is_some() {
            let a = self.appearance.as_ref().unwrap();
            let mut acjf: Appearance = Appearance::new();
            acjf.default_theme_material = a.default_theme_material.clone();
            acjf.default_theme_texture = a.default_theme_texture.clone();
            if a.materials.is_some() {
                let am = a.materials.as_ref().unwrap();
                let mut mats2: Vec<MaterialObject> = Vec::new();
                mats2.resize(m_oldnew.len(), Default::default());
                for (old, new) in &m_oldnew {
                    mats2[*new] = am[*old].clone();
                }
                acjf.materials = Some(mats2);
            }
            if a.textures.is_some() {
                let at = a.textures.as_ref().unwrap();
                let mut texs2: Vec<TextureObject> = Vec::new();
                texs2.resize(t_oldnew.len(), Default::default());
                for (old, new) in &t_oldnew {
                    texs2[*new] = at[*old].clone();
                }
                acjf.textures = Some(texs2);
            }
            if a.vertices_texture.is_some() {
                let atv = a.vertices_texture.as_ref().unwrap();
                let mut t_new_vertices: Vec<[f64; 2]> = Vec::new();
                t_new_vertices.resize(t_v_oldnew.len(), [0.0, 0.0]);
                for (old, new) in &t_v_oldnew {
                    t_new_vertices[*new] = atv[*old].clone();
                }
                acjf.vertices_texture = Some(t_new_vertices);
            }
            cjf.appearance = Some(acjf);
        }
        Some(cjf)
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
            match &mut co.geometry {
                Some(x) => {
                    for g in x.iter_mut() {
                        g.update_geometry_boundaries(&mut newids);
                    }
                }
                None => (),
            }
        }
        //-- replace the vertices, innit?
        self.vertices = newvertices;
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
        for (_key, co) in &self.city_objects {
            if co.is_toplevel() {
                total += 1;
            }
        }
        total
    }
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
            SortingStrategy::Alphabetical => {
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
    fn add_vertices(&mut self, v: &mut Vec<Vec<i64>>) {
        self.vertices.append(v);
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
    pub fn add_material(&mut self, jm: MaterialObject) -> usize {
        let re = match &mut self.appearance {
            Some(x) => x.add_material(jm),
            None => {
                let mut a: Appearance = Appearance::new();
                let re = a.add_material(jm);
                self.appearance = Some(a);
                re
            }
        };
        re
    }
    fn add_texture(&mut self, jm: TextureObject) -> usize {
        let re = match &mut self.appearance {
            Some(x) => x.add_texture(jm),
            None => {
                let mut a: Appearance = Appearance::new();
                let re = a.add_texture(jm);
                self.appearance = Some(a);
                re
            }
        };
        re
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CityJSONFeature {
    #[serde(rename = "type")]
    pub thetype: String,
    pub id: String,
    #[serde(rename = "CityObjects")]
    pub city_objects: HashMap<String, CityObject>,
    pub vertices: Vec<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub appearance: Option<Appearance>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<HashMap<String, Extension>>,
}
impl CityJSONFeature {
    pub fn new() -> Self {
        let co: HashMap<String, CityObject> = HashMap::new();
        let v: Vec<Vec<i64>> = Vec::new();
        CityJSONFeature {
            thetype: "CityJSONFeature".to_string(),
            id: "".to_string(),
            city_objects: co,
            vertices: v,
            appearance: None,
            extensions: None,
        }
    }
    pub fn from_str(s: &str) -> Result<Self> {
        let cjf: CityJSONFeature = serde_json::from_str(&s)?;
        Ok(cjf)
    }
    pub fn add_co(&mut self, id: String, co: CityObject) {
        self.city_objects.insert(id, co);
    }
    pub fn centroid(&self) -> Vec<f64> {
        let mut totals: Vec<f64> = vec![0., 0., 0.];
        for v in &self.vertices {
            for i in 0..3 {
                totals[i] += v[i] as f64;
            }
        }
        for i in 0..3 {
            totals[i] /= self.vertices.len() as f64;
        }
        return totals;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CityObject {
    #[serde(rename = "type")]
    pub thetype: String,
    #[serde(rename = "geographicalExtent")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geographical_extent: Option<GeographicalExtent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geometry: Option<Vec<Geometry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children_roles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parents: Option<Vec<String>>,
    #[serde(flatten)]
    other: serde_json::Value,
}

impl CityObject {
    pub fn new(
        thetype: String,
        geographical_extent: Option<GeographicalExtent>,
        attributes: Option<Value>,
        geometry: Option<Vec<Geometry>>,
        children: Option<Vec<String>>,
        children_roles: Option<Vec<String>>,
        parents: Option<Vec<String>>,
        other: Option<Value>,
    ) -> Self {
        CityObject {
            thetype,
            geographical_extent,
            attributes,
            geometry,
            children,
            children_roles,
            parents,
            other: other.unwrap_or(Value::Null),
        }
    }
    pub fn get_type(&self) -> String {
        self.thetype.clone()
    }

    // Check if this city object is from an extension (type starts with +)
    pub fn is_extension_type(&self) -> bool {
        self.thetype.starts_with("+")
    }

    // If this is an extension type (starts with +), return the extension name
    pub fn get_extension_type(&self) -> Option<String> {
        if self.is_extension_type() {
            // Remove the + prefix and get the part before any further + or . characters
            let type_name = self.thetype.trim_start_matches('+');
            match type_name.find(|c| c == '+' || c == '.') {
                Some(idx) => Some(type_name[..idx].to_string()),
                None => Some(type_name.to_string()),
            }
        } else {
            None
        }
    }

    fn is_toplevel(&self) -> bool {
        match &self.parents {
            Some(x) => {
                if x.is_empty() {
                    return true;
                } else {
                    return false;
                }
            }
            None => return true,
        }
    }
    fn get_children_keys(&self) -> Vec<String> {
        let mut re: Vec<String> = Vec::new();
        match &self.children {
            Some(x) => {
                for each in x {
                    re.push(each.to_string());
                }
            }
            None => (),
        }
        re
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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

pub trait JsonIndex: Clone + PartialEq + Eq + std::fmt::Debug {
    /// Attempt to parse a `T` from the given `Value`.
    /// Return `None` if parsing fails or if you want to skip certain cases.
    fn from_value(v: &Value) -> Option<Self>;

    /// Convert a `T` into a JSON `Value`.
    fn to_value(&self) -> Value;
}

/// Implement `JsonIndex` for a plain `u32`.
/// - `null` is ignored (returns `None`).
/// - Numeric values are cast to `u32` (watch out for possible negative or large values).
impl JsonIndex for u32 {
    fn from_value(v: &Value) -> Option<Self> {
        if let Some(u) = v.as_u64() {
            Some(u as u32)
        } else if let Some(i) = v.as_i64() {
            // You may want to check if `i` is negative, or out of u32 range.
            Some(i as u32)
        } else {
            None
        }
    }

    fn to_value(&self) -> Value {
        Value::Number(Number::from(*self as u64))
    }
}

/// Implement `JsonIndex` for an `Option<u32>`.
/// - `null` becomes `None`.
/// - Numeric values become `Some(...)`.
impl JsonIndex for Option<u32> {
    fn from_value(v: &Value) -> Option<Self> {
        if v.is_null() {
            // JSON null -> None
            Some(None)
        } else if let Some(u) = v.as_u64() {
            Some(Some(u as u32))
        } else if let Some(i) = v.as_i64() {
            Some(Some(i as u32))
        } else {
            None
        }
    }

    fn to_value(&self) -> Value {
        match self {
            Some(x) => Value::Number(Number::from(*x as u64)),
            None => Value::Null,
        }
    }
}

impl JsonIndex for usize {
    fn from_value(v: &Value) -> Option<Self> {
        if let Some(u) = v.as_u64() {
            Some(u as usize)
        } else if let Some(i) = v.as_i64() {
            Some(i as usize)
        } else {
            None
        }
    }

    fn to_value(&self) -> Value {
        Value::Number(Number::from(*self as u64))
    }
}

impl JsonIndex for Option<usize> {
    fn from_value(v: &Value) -> Option<Self> {
        if v.is_null() {
            Some(None)
        } else if let Some(u) = v.as_u64() {
            Some(Some(u as usize))
        } else if let Some(i) = v.as_i64() {
            Some(Some(i as usize))
        } else {
            None
        }
    }

    fn to_value(&self) -> Value {
        Value::Array(self.iter().map(|x| x.to_value()).collect())
    }
}

/// Our nested structure, generic over `T`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NestedArray<T> {
    Indices(Vec<T>),
    Nested(Vec<NestedArray<T>>),
}

/// For convenience, define `Boundaries` as `NestedArray<u32>` (no null allowed).
pub type Boundaries = NestedArray<u32>;
/// For Semantics, define `SemanticsValues` as `NestedArray<Option<u32>>` (null allowed).
pub type SemanticsValues = NestedArray<Option<u32>>;

// ---------------------------------------------------------------------------
// Custom Serialize/Deserialize for `NestedArray<T>`
// where `T: JsonIndex` defines how to go from/to JSON numbers or null.
// ---------------------------------------------------------------------------
impl<T> Serialize for NestedArray<T>
where
    T: JsonIndex,
{
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        nested_array_to_value(self).serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for NestedArray<T>
where
    T: JsonIndex,
{
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = Value::deserialize(deserializer)?;
        Ok(parse_nested_array(&v))
    }
}

// ---------------------------------------------------------------------------
// Parsing from `serde_json::Value` into a `NestedArray<T>`
// ---------------------------------------------------------------------------
fn parse_nested_array<T: JsonIndex>(v: &Value) -> NestedArray<T> {
    match v {
        Value::Array(elems) => {
            if elems.is_empty() {
                return NestedArray::Indices(Vec::new());
            }
            // If the first element is itself an Array, assume it's "Nested"
            if let Value::Array(_) = &elems[0] {
                let mut nested = Vec::with_capacity(elems.len());
                for sub in elems {
                    nested.push(parse_nested_array(sub));
                }
                NestedArray::Nested(nested)
            } else {
                // Indices: parse each element via `T::from_value()`
                let mut indices = Vec::with_capacity(elems.len());
                for elem in elems {
                    if let Some(val) = T::from_value(elem) {
                        indices.push(val);
                    } else {
                        // If we can't parse, you could choose to skip or push a default.
                        // Here we skip.
                    }
                }
                NestedArray::Indices(indices)
            }
        }
        // Not an array? Return an empty Indices array by default
        _ => NestedArray::Indices(Vec::new()),
    }
}

// ---------------------------------------------------------------------------
// Converting a `NestedArray<T>` to `serde_json::Value`
// ---------------------------------------------------------------------------
fn nested_array_to_value<T: JsonIndex>(na: &NestedArray<T>) -> Value {
    match na {
        NestedArray::Indices(vec_of_t) => {
            let arr = vec_of_t.iter().map(|t| t.to_value()).collect();
            Value::Array(arr)
        }
        NestedArray::Nested(vec_of_nested) => {
            let arr = vec_of_nested
                .iter()
                .map(|sub_na| nested_array_to_value(sub_na))
                .collect();
            Value::Array(arr)
        }
    }
}

impl Boundaries {
    fn update_indices_recursively(&mut self, violdnew: &mut HashMap<usize, usize>) {
        match self {
            Boundaries::Indices(arr) => {
                for index in arr {
                    let old_idx = *index;
                    let new_idx = {
                        let len = violdnew.len();
                        *violdnew.entry(old_idx as usize).or_insert_with(|| len)
                    };
                    *index = new_idx as u32;
                }
            }
            Boundaries::Nested(nested_vec) => {
                for sub in nested_vec {
                    sub.update_indices_recursively(violdnew);
                }
            }
        }
    }
    fn offset_geometry_boundaries(&mut self, offset: usize) {
        match self {
            Boundaries::Indices(indices) => {
                for index in indices {
                    *index += offset as u32;
                }
            }
            Boundaries::Nested(nested) => {
                for sub in nested {
                    sub.offset_geometry_boundaries(offset);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticsSurface {
    #[serde(rename = "type")]
    pub thetype: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<u32>>,
    #[serde(flatten)]
    pub other: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Semantics {
    pub values: SemanticsValues,
    pub surfaces: Vec<SemanticsSurface>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Geometry {
    #[serde(rename = "type")]
    pub thetype: GeometryType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lod: Option<String>,
    pub boundaries: Boundaries,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantics: Option<Semantics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub material: Option<HashMap<String, MaterialReference>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture: Option<HashMap<String, TextureReference>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<usize>,
    #[serde(rename = "transformationMatrix")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transformation_matrix: Option<[f64; 16]>, // 4x4 matrix stored row-by-row
}
impl Geometry {
    fn update_geometry_boundaries(&mut self, violdnew: &mut HashMap<usize, usize>) {
        match &mut self.boundaries {
            Boundaries::Indices(indices) => {
                for index in indices {
                    let old_idx = *index;
                    let new_idx = {
                        let len = violdnew.len();
                        *violdnew.entry(old_idx as usize).or_insert_with(|| len)
                    };
                    *index = new_idx as u32;
                }
            }
            Boundaries::Nested(nested) => {
                for sub in nested {
                    match sub {
                        Boundaries::Indices(r) => {
                            for index in r {
                                let old_idx = *index;
                                let new_idx = {
                                    let len = violdnew.len();
                                    *violdnew.entry(old_idx as usize).or_insert_with(|| len)
                                };
                                *index = new_idx as u32;
                            }
                        }
                        Boundaries::Nested(_) => {
                            sub.update_indices_recursively(violdnew);
                        }
                    }
                }
            }
        }
    }

    fn offset_geometry_boundaries(&mut self, offset: usize) {
        self.boundaries.offset_geometry_boundaries(offset);
    }

    fn update_material(&mut self, m_oldnew: &mut HashMap<usize, usize>) {
        match &mut self.material {
            Some(x) => {
                for (_key, mat) in &mut *x {
                    //-- material.value
                    if mat.value.is_some() {
                        let thevalue: usize = mat.value.unwrap();
                        let r = m_oldnew.get(&thevalue);
                        if r.is_none() {
                            let l = m_oldnew.len();
                            m_oldnew.insert(thevalue, l);
                            mat.value = Some(l);
                        } else {
                            let r2 = r.unwrap();
                            mat.value = Some(*r2);
                        }
                        continue;
                    }

                    //-- else it's material.values
                    if let Some(values) = &mut mat.values {
                        // Helper function to update indices in a nested array
                        fn update_indices(
                            array: &mut NestedArray<Option<usize>>,
                            m_oldnew: &mut HashMap<usize, usize>,
                        ) {
                            match array {
                                NestedArray::Indices(indices) => {
                                    for idx in indices.iter_mut().filter_map(|x| x.as_mut()) {
                                        let old_idx = *idx;
                                        let new_idx = {
                                            let y = m_oldnew.get(&old_idx);
                                            if y.is_none() {
                                                let l = m_oldnew.len();
                                                m_oldnew.insert(old_idx, l);
                                                l
                                            } else {
                                                *y.unwrap()
                                            }
                                        };
                                        *idx = new_idx;
                                    }
                                }
                                NestedArray::Nested(nested) => {
                                    for sub in nested {
                                        update_indices(sub, m_oldnew);
                                    }
                                }
                            }
                        }

                        update_indices(values, m_oldnew);
                    }
                }
                self.material = Some(x.clone());
            }
            None => (),
        }
    }
    fn update_texture(
        &mut self,
        t_oldnew: &mut HashMap<usize, usize>,
        t_v_oldnew: &mut HashMap<usize, usize>,
        offset: usize,
    ) {
        match &mut self.texture {
            Some(x) => {
                for (_key, tex) in &mut *x {
                    // Helper function to update indices in a nested array
                    fn update_texture_indices(
                        array: &mut NestedArray<Option<usize>>,
                        t_oldnew: &mut HashMap<usize, usize>,
                        t_v_oldnew: &mut HashMap<usize, usize>,
                        offset: usize,
                        depth: usize,
                    ) {
                        match array {
                            NestedArray::Indices(indices) => {
                                for (i, idx) in indices.iter_mut().enumerate() {
                                    if let Some(old_idx) = idx {
                                        let new_idx = if i == 0 {
                                            // First index is texture index
                                            let y = t_oldnew.get(old_idx);
                                            if y.is_none() {
                                                let l = t_oldnew.len();
                                                t_oldnew.insert(*old_idx, l);
                                                l
                                            } else {
                                                *y.unwrap()
                                            }
                                        } else {
                                            // Other indices are vertex texture coordinates
                                            let y = t_v_oldnew.get(old_idx);
                                            if y.is_none() {
                                                let l = t_v_oldnew.len();
                                                t_v_oldnew.insert(*old_idx, l + offset);
                                                l + offset
                                            } else {
                                                *y.unwrap()
                                            }
                                        };
                                        *idx = Some(new_idx);
                                    }
                                }
                            }
                            NestedArray::Nested(nested) => {
                                for sub in nested {
                                    update_texture_indices(
                                        sub,
                                        t_oldnew,
                                        t_v_oldnew,
                                        offset,
                                        depth + 1,
                                    );
                                }
                            }
                        }
                    }

                    update_texture_indices(&mut tex.values, t_oldnew, t_v_oldnew, offset, 0);
                }
            }
            None => (),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Transform {
    pub scale: Vec<f64>,
    pub translate: Vec<f64>,
}
impl Transform {
    fn new() -> Self {
        Transform {
            scale: vec![1.0, 1.0, 1.0],
            translate: vec![0., 0., 0.],
        }
    }
}

pub type GeographicalExtent = [f64; 6];

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Address {
    #[serde(rename = "thoroughfareNumber")]
    pub thoroughfare_number: i64,
    #[serde(rename = "thoroughfareName")]
    pub thoroughfare_name: String,
    pub locality: String,
    #[serde(rename = "postalCode")]
    pub postal_code: String,
    pub country: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PointOfContact {
    #[serde(rename = "contactName")]
    pub contact_name: String,
    #[serde(rename = "contactType")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_type: Option<String>,
    #[serde(rename = "role")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(rename = "emailAddress")]
    pub email_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<Address>,
}

/// A reference system following the OGC Name Type Specification.
///
/// The format follows: `http://www.opengis.net/def/crs/{authority}/{version}/{code}`
/// where:
/// - `{authority}` designates the authority responsible for the definition of this CRS
///   (usually "EPSG" or "OGC")
/// - `{version}` designates the specific version of the CRS
///   (use "0" if there is no version)
/// - `{code}` is the identifier for the specific coordinate reference system
#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceSystem {
    pub base_url: String,
    pub authority: String,
    pub version: String,
    pub code: String,
}

impl ReferenceSystem {
    pub fn new(base_url: Option<String>, authority: String, version: String, code: String) -> Self {
        let base_url = base_url.unwrap_or(DEFAULT_CRS_BASE_URL.to_string());
        ReferenceSystem {
            base_url,
            authority,
            version,
            code,
        }
    }

    pub fn to_url(&self) -> String {
        format!(
            "{}/{}/{}/{}",
            self.base_url, self.authority, self.version, self.code
        )
    }

    // OGC Name Type Specification:
    // http://www.opengis.net/def/crs/{authority}/{version}/{code}
    // where {authority} designates the authority responsible for the definition of this CRS (usually "EPSG" or "OGC"), and where {version} designates the specific version of the CRS ("0" (zero) is used if there is no version).
    pub fn from_url(url: &str) -> Result<Self> {
        if !url.contains("//www.opengis.net/def/crs") {
            return Err(CjseqError::Generic(
                "Invalid reference system URL".to_string(),
            ));
        }

        let i = url.find("crs").unwrap();
        let s = &url[i + 4..];

        let parts: Vec<&str> = s.split("/").collect();
        if parts.len() != 3 {
            return Err(CjseqError::Generic(
                "Invalid reference system URL".to_string(),
            ));
        }

        Ok(ReferenceSystem {
            base_url: url[..i + 3].to_string(),
            authority: parts[0].to_string(),
            version: parts[1].to_string(),
            code: parts[2].to_string(),
        })
    }
}

impl Serialize for ReferenceSystem {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_url().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ReferenceSystem {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let url = String::deserialize(deserializer)?;
        ReferenceSystem::from_url(&url).map_err(|e| serde::de::Error::custom(e.to_string()))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Metadata {
    #[serde(rename = "geographicalExtent")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geographical_extent: Option<GeographicalExtent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    #[serde(rename = "pointOfContact")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub point_of_contact: Option<PointOfContact>,
    #[serde(rename = "referenceDate")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_date: Option<String>,
    #[serde(rename = "referenceSystem")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_system: Option<ReferenceSystem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GeometryTemplates {
    pub templates: Vec<Geometry>,
    #[serde(rename = "vertices-templates")]
    pub vertices_templates: Vec<[f64; 3]>,
}

pub trait Validate {
    fn validate(&self) -> Result<()>;
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct MaterialObject {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "ambientIntensity")]
    pub ambient_intensity: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "diffuseColor")]
    pub diffuse_color: Option<[f64; 3]>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "emissiveColor")]
    pub emissive_color: Option<[f64; 3]>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "specularColor")]
    pub specular_color: Option<[f64; 3]>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "shininess")]
    pub shininess: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "transparency")]
    pub transparency: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "isSmooth")]
    pub is_smooth: Option<bool>,
}

impl Validate for MaterialObject {
    fn validate(&self) -> Result<()> {
        if let Some(intensity) = self.ambient_intensity {
            if !(0.0..=1.0).contains(&intensity) {
                return Err(CjseqError::InvalidValue {
                    field: "ambient_intensity".to_string(),
                    reason: "must be between 0.0 and 1.0".to_string(),
                });
            }
        }

        for (name, color) in [
            ("diffuse_color", &self.diffuse_color),
            ("emissive_color", &self.emissive_color),
            ("specular_color", &self.specular_color),
        ] {
            if let Some(c) = color {
                for &v in c.iter() {
                    if !(0.0..=1.0).contains(&v) {
                        return Err(CjseqError::InvalidValue {
                            field: name.to_string(),
                            reason: "must be between 0.0 and 1.0".to_string(),
                        });
                    }
                }
            }
        }

        if let Some(shininess) = self.shininess {
            if !(0.0..=1.0).contains(&shininess) {
                return Err(CjseqError::InvalidValue {
                    field: "shininess".to_string(),
                    reason: "must be between 0.0 and 1.0".to_string(),
                });
            }
        }

        if let Some(transparency) = self.transparency {
            if !(0.0..=1.0).contains(&transparency) {
                return Err(CjseqError::InvalidValue {
                    field: "transparency".to_string(),
                    reason: "must be between 0.0 and 1.0".to_string(),
                });
            }
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum TextFormat {
    Png,
    Jpg,
}

impl Default for TextFormat {
    fn default() -> Self {
        TextFormat::Jpg
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WrapMode {
    None,
    Wrap,
    Mirror,
    Clamp,
    Border,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TextType {
    Unknown,
    Specific,
    Typical,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct TextureObject {
    #[serde(rename = "type")]
    pub texture_format: TextFormat,
    pub image: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "wrapMode")]
    pub wrap_mode: Option<WrapMode>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "textureType")]
    pub texture_type: Option<TextType>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "borderColor")]
    pub border_color: Option<[f64; 4]>,
}

impl Validate for TextureObject {
    fn validate(&self) -> Result<()> {
        if let Some(colors) = &self.border_color {
            for (i, &c) in colors.iter().enumerate() {
                if !(0.0..=1.0).contains(&c) {
                    return Err(CjseqError::InvalidValue {
                        field: "border_color".to_string(),
                        reason: format!("border_color[{}] must be between 0.0 and 1.0", i),
                    });
                }
            }
        }
        Ok(())
    }
}

pub type MaterialValues = NestedArray<Option<usize>>;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MaterialReference {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<MaterialValues>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<usize>,
}

pub type TextureValues = NestedArray<Option<usize>>;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TextureReference {
    pub values: TextureValues,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Appearance {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub materials: Option<Vec<MaterialObject>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub textures: Option<Vec<TextureObject>>,
    #[serde(rename = "vertices-texture")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertices_texture: Option<Vec<[f64; 2]>>, // Array of [u,v] coordinates
    #[serde(rename = "default-theme-texture")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_theme_texture: Option<String>,
    #[serde(rename = "default-theme-material")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_theme_material: Option<String>,
}

impl Appearance {
    fn new() -> Self {
        Appearance {
            materials: None,
            textures: None,
            vertices_texture: None,
            default_theme_texture: None,
            default_theme_material: None,
        }
    }

    fn add_material(&mut self, value: MaterialObject) -> usize {
        // Validate material before adding
        if let Err(e) = value.validate() {
            panic!("Invalid material: {}", e);
        }

        match &mut self.materials {
            Some(x) => match x.iter().position(|e| e.name == value.name) {
                Some(y) => y,
                None => {
                    x.push(value);
                    x.len() - 1
                }
            },
            None => {
                let mut ls = Vec::new();
                ls.push(value);
                self.materials = Some(ls);
                0
            }
        }
    }

    fn add_texture(&mut self, value: TextureObject) -> usize {
        // Validate texture before adding
        if let Err(e) = value.validate() {
            panic!("Invalid texture: {}", e);
        }

        match &mut self.textures {
            Some(x) => match x.iter().position(|e| e.image == value.image) {
                Some(y) => y,
                None => {
                    x.push(value);
                    x.len() - 1
                }
            },
            None => {
                let mut ls = Vec::new();
                ls.push(value);
                self.textures = Some(ls);
                0
            }
        }
    }

    fn add_vertices_texture(&mut self, vs: Vec<[f64; 2]>) {
        match &mut self.vertices_texture {
            Some(x) => {
                x.extend(vs);
            }
            None => {
                self.vertices_texture = Some(vs);
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Extension {
    pub url: String,
    pub version: String,
}

impl Extension {
    // Convert an extension reference to a minimal extension file template
    pub fn new(url: String, version: String) -> Self {
        Extension { url, version }
    }

    // Fetch the full extension file from the URL
    pub fn fetch_extension_file(&self, name: String) -> Result<ExtensionFile> {
        ExtensionFile::fetch_from_url(name, self.url.clone(), self.version.clone())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ExtensionFile {
    #[serde(rename = "type")]
    pub thetype: String, // Must be "CityJSONExtension"
    pub name: String,
    pub description: String,
    pub url: String,
    pub version: String,
    #[serde(rename = "versionCityJSON")]
    pub version_city_json: String,
    #[serde(rename = "extraAttributes")]
    pub extra_attributes: Value,
    #[serde(rename = "extraCityObjects")]
    pub extra_city_objects: Value,
    #[serde(rename = "extraRootProperties")]
    pub extra_root_properties: Value,
    #[serde(rename = "extraSemanticSurfaces")]
    pub extra_semantic_surfaces: Value,
}

impl ExtensionFile {
    /// Creates a new ExtensionFile with the given parameters without fetching data
    pub fn new(name: String, url: String, version: String) -> Self {
        ExtensionFile {
            thetype: "CityJSONExtension".to_string(),
            name,
            description: "".to_string(),
            url,
            version,
            version_city_json: "2.0".to_string(),
            extra_attributes: json!({}),
            extra_city_objects: json!({}),
            extra_root_properties: json!({}),
            extra_semantic_surfaces: json!({}),
        }
    }

    /// Creates a new ExtensionFile by fetching JSON schema from the URL
    pub fn fetch_from_url(name: String, url: String, version: String) -> Result<Self> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Native implementation using reqwest
            use std::time::Duration;

            // Create a client with a timeout
            let client = match reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
            {
                Ok(client) => client,
                Err(e) => return Err(CjseqError::Generic(e.to_string())),
            };

            // Fetch the extension schema
            let response = match client.get(&url).send() {
                Ok(response) => response,
                Err(e) => return Err(CjseqError::HttpError(e)),
            };

            // Parse the JSON response
            let schema: serde_json::Value = match response.json() {
                Ok(json) => json,
                Err(e) => return Err(CjseqError::HttpError(e)),
            };

            // Create and populate the extension file
            let mut extension = Self::new(name, url, version);
            Self::populate_extension_from_json(&mut extension, schema);
            Ok(extension)
        }

        #[cfg(target_arch = "wasm32")]
        {
            // WASM implementation using gloo-net
            use wasm_bindgen_futures::JsFuture;

            // This is a blocking function that needs async in WASM
            // Return a placeholder and warn the user
            eprintln!("Warning: fetch_from_url is not fully implemented for WASM targets. Use the async version instead.");

            // Create a placeholder extension file
            let extension = Self::new(name, url, version);
            Ok(extension)
        }
    }

    // For WASM environments: async version of fetch_from_url
    #[cfg(target_arch = "wasm32")]
    pub async fn fetch_from_url_async(name: String, url: String, version: String) -> Result<Self> {
        use gloo_net::http::Request;

        // Fetch the extension schema
        let response = match Request::get(&url).send().await {
            Ok(response) => response,
            Err(e) => return Err(CjseqError::GlooHttpError(e)),
        };

        // Parse the JSON response
        let schema: serde_json::Value = match response.json().await {
            Ok(json) => json,
            Err(e) => return Err(CjseqError::GlooHttpError(e)),
        };

        // Create and populate the extension file
        let mut extension = Self::new(name, url, version);
        Self::populate_extension_from_json(&mut extension, schema);
        Ok(extension)
    }

    // Helper function to populate extension data from JSON response
    // Used by both native and WASM implementations
    fn populate_extension_from_json(extension: &mut Self, schema: serde_json::Value) {
        if let Some(obj) = schema.get("description").and_then(|v| v.as_str()) {
            extension.description = obj.to_string();
        }

        // Extract schema components if available
        if let Some(obj) = schema.get("extraAttributes").and_then(|v| v.as_object()) {
            extension.extra_attributes = json!(obj);
        }

        if let Some(obj) = schema.get("extraCityObjects").and_then(|v| v.as_object()) {
            extension.extra_city_objects = json!(obj);
        }

        if let Some(obj) = schema
            .get("extraRootProperties")
            .and_then(|v| v.as_object())
        {
            extension.extra_root_properties = json!(obj);
        }

        if let Some(obj) = schema
            .get("extraSemanticSurfaces")
            .and_then(|v| v.as_object())
        {
            extension.extra_semantic_surfaces = json!(obj);
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.thetype != "CityJSONExtension" {
            return Err(CjseqError::InvalidValue {
                field: "thetype".to_string(),
                reason: "must be 'CityJSONExtension'".to_string(),
            });
        }

        if self.name.is_empty() {
            return Err(CjseqError::InvalidValue {
                field: "name".to_string(),
                reason: "cannot be empty".to_string(),
            });
        }

        if self.url.is_empty() {
            return Err(CjseqError::InvalidValue {
                field: "url".to_string(),
                reason: "cannot be empty".to_string(),
            });
        }

        if self.version.is_empty() {
            return Err(CjseqError::InvalidValue {
                field: "version".to_string(),
                reason: "cannot be empty".to_string(),
            });
        }

        if self.version_city_json.is_empty() {
            return Err(CjseqError::InvalidValue {
                field: "versionCityJSON".to_string(),
                reason: "cannot be empty".to_string(),
            });
        }

        // Validate that the "extra" fields are objects (can be empty but must be objects)
        for (field_name, field) in [
            ("extraAttributes", &self.extra_attributes),
            ("extraCityObjects", &self.extra_city_objects),
            ("extraRootProperties", &self.extra_root_properties),
            ("extraSemanticSurfaces", &self.extra_semantic_surfaces),
        ] {
            if !field.is_object() {
                return Err(CjseqError::InvalidValue {
                    field: field_name.to_string(),
                    reason: "must be a JSON object".to_string(),
                });
            }
        }

        Ok(())
    }

    // Get all the extension city object types defined in this extension
    pub fn get_city_object_types(&self) -> Vec<String> {
        match self.extra_city_objects.as_object() {
            Some(obj) => obj.keys().map(|k| k.clone()).collect(),
            None => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use std::io::{BufRead, BufReader};

    /// Test cases derived from CityJSON specification v2.0.1
    /// See: https://www.cityjson.org/specs/2.0.1/#semantics-of-geometric-primitives

    /// MultiPoint: Single array of vertex indices
    /// [v1, v2, v3, ...]
    #[test]
    fn test_multipoint_boundaries() {
        let json_value = json!([2, 44, 0, 7]);
        let boundaries = parse_nested_array::<usize>(&json_value);
        assert_eq!(boundaries, NestedArray::Indices(vec![2, 44, 0, 7]));
    }

    /// MultiLineString: Array of arrays, each inner array represents a linestring
    /// [[v1, v2, v3, ...], [v1, v2, v3, ...], ...]
    #[test]
    fn test_multilinestring_boundaries() {
        let json_value = json!([[2, 3, 5], [77, 55, 212]]);
        let boundaries = parse_nested_array::<usize>(&json_value);
        assert_eq!(
            boundaries,
            NestedArray::Nested(vec![
                NestedArray::Indices(vec![2, 3, 5]),
                NestedArray::Indices(vec![77, 55, 212]),
            ])
        );
    }

    /// MultiSurface: Array of surfaces, each surface is an array of rings, each ring is an array of vertex indices
    /// [[[v1, v2, v3, ...]], [[v1, v2, v3, ...]], ...]
    /// The innermost array represents a ring (exterior or interior)
    #[test]
    fn test_multisurface_boundaries() {
        let json_value = json!([[[0, 3, 2, 1]], [[4, 5, 6, 7]], [[0, 1, 5, 4]]]);
        let boundaries = parse_nested_array::<usize>(&json_value);
        assert_eq!(
            boundaries,
            NestedArray::Nested(vec![
                NestedArray::Nested(vec![NestedArray::Indices(vec![0, 3, 2, 1])]),
                NestedArray::Nested(vec![NestedArray::Indices(vec![4, 5, 6, 7])]),
                NestedArray::Nested(vec![NestedArray::Indices(vec![0, 1, 5, 4])]),
            ])
        );
    }

    /// Solid: Array of shells (exterior + optional interior), each shell is an array of surfaces
    /// Each surface is an array of rings, each ring is an array of vertex indices
    /// [
    ///   [[[v1, v2, ...]], [[v1, v2, ...]], ...],  // exterior shell
    ///   [[[v1, v2, ...]], [[v1, v2, ...]], ...]   // interior shell
    /// ]
    #[test]
    fn test_solid_boundaries() {
        let json_value = json!([
            [
                [[0, 3, 2, 1, 22]],
                [[4, 5, 6, 7]],
                [[0, 1, 5, 4]],
                [[1, 2, 6, 5]]
            ],
            [
                [[240, 243, 124]],
                [[244, 246, 724]],
                [[34, 414, 45]],
                [[111, 246, 5]]
            ]
        ]);
        let boundaries = parse_nested_array::<usize>(&json_value);
        assert_eq!(
            boundaries,
            NestedArray::Nested(vec![
                NestedArray::Nested(vec![
                    NestedArray::Nested(vec![NestedArray::Indices(vec![0, 3, 2, 1, 22])]),
                    NestedArray::Nested(vec![NestedArray::Indices(vec![4, 5, 6, 7])]),
                    NestedArray::Nested(vec![NestedArray::Indices(vec![0, 1, 5, 4])]),
                    NestedArray::Nested(vec![NestedArray::Indices(vec![1, 2, 6, 5])]),
                ]),
                NestedArray::Nested(vec![
                    NestedArray::Nested(vec![NestedArray::Indices(vec![240, 243, 124])]),
                    NestedArray::Nested(vec![NestedArray::Indices(vec![244, 246, 724])]),
                    NestedArray::Nested(vec![NestedArray::Indices(vec![34, 414, 45])]),
                    NestedArray::Nested(vec![NestedArray::Indices(vec![111, 246, 5])]),
                ]),
            ])
        );
    }

    /// CompositeSolid: Array of solids, each solid follows the Solid structure above
    /// [
    ///   [ // First solid
    ///     [[[v1, v2, ...]], [[v1, v2, ...]], ...],  // exterior shell
    ///     [[[v1, v2, ...]], [[v1, v2, ...]], ...]   // interior shell
    ///   ],
    ///   [ // Second solid
    ///     [[[v1, v2, ...]], [[v1, v2, ...]], ...]   // only exterior shell
    ///   ]
    /// ]
    #[test]
    fn test_composite_solid_boundaries() {
        let json_value = json!([
            [
                [
                    [[0, 3, 2, 1, 22]],
                    [[4, 5, 6, 7]],
                    [[0, 1, 5, 4]],
                    [[1, 2, 6, 5]]
                ],
                [
                    [[240, 243, 124]],
                    [[244, 246, 724]],
                    [[34, 414, 45]],
                    [[111, 246, 5]]
                ]
            ],
            [[
                [[666, 667, 668]],
                [[74, 75, 76]],
                [[880, 881, 885]],
                [[111, 122, 226]]
            ]]
        ]);
        let boundaries = parse_nested_array::<usize>(&json_value);
        assert_eq!(
            boundaries,
            NestedArray::Nested(vec![
                NestedArray::Nested(vec![
                    NestedArray::Nested(vec![
                        NestedArray::Nested(vec![NestedArray::Indices(vec![0, 3, 2, 1, 22])]),
                        NestedArray::Nested(vec![NestedArray::Indices(vec![4, 5, 6, 7])]),
                        NestedArray::Nested(vec![NestedArray::Indices(vec![0, 1, 5, 4])]),
                        NestedArray::Nested(vec![NestedArray::Indices(vec![1, 2, 6, 5])]),
                    ]),
                    NestedArray::Nested(vec![
                        NestedArray::Nested(vec![NestedArray::Indices(vec![240, 243, 124])]),
                        NestedArray::Nested(vec![NestedArray::Indices(vec![244, 246, 724])]),
                        NestedArray::Nested(vec![NestedArray::Indices(vec![34, 414, 45])]),
                        NestedArray::Nested(vec![NestedArray::Indices(vec![111, 246, 5])]),
                    ]),
                ]),
                NestedArray::Nested(vec![NestedArray::Nested(vec![
                    NestedArray::Nested(vec![NestedArray::Indices(vec![666, 667, 668])]),
                    NestedArray::Nested(vec![NestedArray::Indices(vec![74, 75, 76])]),
                    NestedArray::Nested(vec![NestedArray::Indices(vec![880, 881, 885])]),
                    NestedArray::Nested(vec![NestedArray::Indices(vec![111, 122, 226])]),
                ]),])
            ])
        );
    }

    #[test]
    fn test_appearance_parsing() {
        // Read the test fixture. The file is Rotterdams data.
        let file =
            fs::File::open("tests/fixtures/appearance.jsonl").expect("Unable to read test fixture");
        let reader = BufReader::new(file);
        let lines: Vec<String> = reader
            .lines()
            .map(|line| line.expect("Failed to read line"))
            .collect();

        // Skip first line (CityJSON object) and use second line (CityJSONFeature)
        let feature: CityJSONFeature = serde_json::from_str(&lines[1]).unwrap();

        // Test appearance exists
        let appearance = feature.appearance.unwrap();

        let materials = appearance.materials.unwrap();
        let textures = appearance.textures.unwrap();
        let vertices_tex = appearance.vertices_texture.unwrap();

        // Test textures
        assert!(textures.len() == 5);
        assert!(textures[0].texture_format == TextFormat::Jpg);
        assert!(textures[0].image == "appearances/0320_2_12.jpg");

        // Test vertices-texture
        assert!(vertices_tex.len() == 30);
        assert_eq!(vertices_tex[0], [0.2517, 0.1739]);

        // Test materials
        assert!(materials.len() == 2);
        assert_eq!(materials[0].name, "roofandground");
        assert_eq!(materials[0].ambient_intensity, Some(0.2000));
        assert_eq!(materials[0].diffuse_color, Some([0.9000, 0.1000, 0.7500]));
        assert_eq!(materials[0].emissive_color, Some([0.9000, 0.1000, 0.7500]));
        assert_eq!(materials[0].specular_color, Some([0.9, 0.1, 0.75]));
        assert_eq!(materials[0].shininess, Some(0.2));
        assert_eq!(materials[0].transparency, Some(0.5));
        assert_eq!(materials[0].is_smooth, Some(false));

        // Verify optional fields are None
        assert!(appearance.default_theme_texture.is_none());
        assert!(appearance.default_theme_material.is_none());
    }

    #[test]
    fn test_add_material() {
        let mut appearance = Appearance::new();

        // Test adding first material
        let mat1 = MaterialObject {
            name: "roofandground".to_string(),
            ambient_intensity: Some(0.2),
            diffuse_color: Some([0.9, 0.1, 0.75]),
            emissive_color: Some([0.9, 0.1, 0.75]),
            specular_color: Some([0.9, 0.1, 0.75]),
            shininess: Some(0.2),
            transparency: Some(0.5),
            is_smooth: Some(false),
        };
        let index1 = appearance.add_material(mat1.clone());
        assert_eq!(index1, 0);

        // Test adding duplicate material (should return same index)
        let index2 = appearance.add_material(mat1.clone());
        assert_eq!(index2, 0);

        // Test adding different material
        let mat2 = MaterialObject {
            name: "wall".to_string(),
            ambient_intensity: Some(0.4),
            diffuse_color: Some([0.1, 0.1, 0.9]),
            emissive_color: Some([0.1, 0.1, 0.9]),
            specular_color: Some([0.9, 0.1, 0.75]),
            shininess: Some(0.0),
            transparency: Some(0.5),
            is_smooth: Some(true),
        };
        let index3 = appearance.add_material(mat2);
        assert_eq!(index3, 1);
    }

    #[test]
    fn test_add_texture() {
        let mut appearance = Appearance::new();

        // Test adding first texture
        let tex1 = TextureObject {
            texture_format: TextFormat::Jpg,
            image: "appearances/0320_2_12.jpg".to_string(),
            wrap_mode: None,
            texture_type: None,
            border_color: None,
        };
        let index1 = appearance.add_texture(tex1.clone());
        assert_eq!(index1, 0);

        // Test adding duplicate texture (should return same index)
        let index2 = appearance.add_texture(tex1);
        assert_eq!(index2, 0);

        // Test adding different texture
        let tex2 = TextureObject {
            texture_format: TextFormat::Jpg,
            image: "appearances/0320_2_13.jpg".to_string(),
            wrap_mode: None,
            texture_type: None,
            border_color: None,
        };
        let index3 = appearance.add_texture(tex2);
        assert_eq!(index3, 1);
    }

    #[test]
    fn test_add_vertices_texture() {
        let mut appearance = Appearance::new();

        // Test adding first set of vertices
        let vertices1 = vec![[0.2517, 0.1739], [0.3155, 0.2015]];
        appearance.add_vertices_texture(vertices1.clone());
        assert_eq!(appearance.vertices_texture.as_ref().unwrap().len(), 2);

        // Test adding more vertices
        let vertices2 = vec![[0.2734, 0.3057], [0.2269, 0.2883]];
        appearance.add_vertices_texture(vertices2);
        assert_eq!(appearance.vertices_texture.as_ref().unwrap().len(), 4);

        // Verify all vertices are present in correct order
        let all_vertices = appearance.vertices_texture.unwrap();
        assert_eq!(all_vertices[0], [0.2517, 0.1739]);
        assert_eq!(all_vertices[1], [0.3155, 0.2015]);
        assert_eq!(all_vertices[2], [0.2734, 0.3057]);
        assert_eq!(all_vertices[3], [0.2269, 0.2883]);
    }

    #[test]
    fn test_material_validation() {
        let valid_material = MaterialObject {
            name: "test".to_string(),
            ambient_intensity: Some(0.5),
            diffuse_color: Some([0.1, 0.2, 0.3]),
            emissive_color: Some([0.4, 0.5, 0.6]),
            specular_color: Some([0.7, 0.8, 0.9]),
            shininess: Some(0.4),
            transparency: Some(0.3),
            is_smooth: Some(true),
        };
        assert!(valid_material.validate().is_ok());

        let invalid_ambient = MaterialObject {
            ambient_intensity: Some(1.5),
            ..valid_material.clone()
        };
        assert!(invalid_ambient.validate().is_err());

        let invalid_diffuse = MaterialObject {
            diffuse_color: Some([-0.1, 0.5, 1.2]),
            ..valid_material.clone()
        };
        assert!(invalid_diffuse.validate().is_err());

        let invalid_emissive = MaterialObject {
            emissive_color: Some([0.5, 1.5, 0.5]),
            ..valid_material.clone()
        };
        assert!(invalid_emissive.validate().is_err());

        let invalid_specular = MaterialObject {
            specular_color: Some([0.5, 0.5, 1.5]),
            ..valid_material.clone()
        };
        assert!(invalid_specular.validate().is_err());

        let invalid_shininess = MaterialObject {
            shininess: Some(2.0),
            ..valid_material.clone()
        };
        assert!(invalid_shininess.validate().is_err());

        let invalid_transparency = MaterialObject {
            transparency: Some(-0.5),
            ..valid_material.clone()
        };
        assert!(invalid_transparency.validate().is_err());
    }

    #[test]
    fn test_texture_validation() {
        let valid_texture = TextureObject {
            texture_format: TextFormat::Jpg,
            image: "test.jpg".to_string(),
            wrap_mode: Some(WrapMode::Wrap),
            texture_type: Some(TextType::Specific),
            border_color: Some([0.1, 0.2, 0.3, 0.4]),
        };
        assert!(valid_texture.validate().is_ok());

        let invalid_border_color = TextureObject {
            border_color: Some([-0.1, 0.5, 1.2, 0.5]),
            ..valid_texture.clone()
        };
        assert!(invalid_border_color.validate().is_err());

        let invalid_border_color_2 = TextureObject {
            border_color: Some([0.5, 0.5, 0.5, 1.5]),
            ..valid_texture.clone()
        };
        assert!(invalid_border_color_2.validate().is_err());
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn test_extension_file_fetch() {
        // Note: This test makes a network request and might fail if the URL is invalid
        // or if there's no internet connection
        let result = ExtensionFile::fetch_from_url(
            "Noise".to_string(),
            "https://www.cityjson.org/schemas/2.0/extensions/noise.ext.json".to_string(),
            "1.0".to_string(),
        );

        // Just check if we can parse it without errors
        if let Ok(extension) = result {
            assert_eq!(extension.name, "Noise");
            assert!(extension.validate().is_ok());
        }
    }

    #[test]
    fn test_extension_file_validation() {
        let valid_ext = ExtensionFile::new(
            "Noise".to_string(),
            "https://www.cityjson.org/schemas/extensions/noise.ext.json".to_string(),
            "1.0".to_string(),
        );
        assert!(valid_ext.validate().is_ok());

        // Test invalid type
        let mut invalid_type = valid_ext.clone();
        invalid_type.thetype = "Invalid".to_string();
        assert!(invalid_type.validate().is_err());

        // Test empty name
        let mut invalid_name = valid_ext.clone();
        invalid_name.name = "".to_string();
        assert!(invalid_name.validate().is_err());

        // Test empty URL
        let mut invalid_url = valid_ext.clone();
        invalid_url.url = "".to_string();
        assert!(invalid_url.validate().is_err());
    }
}
