use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Error, Number, Value};
use std::collections::HashMap;

const DEFAULT_CRS_BASE_URL: &str = "https://www.opengis.net/def/crs";

#[derive(Clone)]
pub enum SortingStrategy {
    Random,
    Alphabetical,
    Morton, //-- TODO implement Morton sorting
    Hilbert,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
    pub extensions: Option<Value>,
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
    pub fn from_str(s: &str) -> Result<Self, Error> {
        let mut cjj: CityJSON = serde_json::from_str(s)?;
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
                        let mut mats2: Vec<Value> = Vec::new();
                        mats2.resize(m_oldnew.len(), json!(null));
                        for (old, new) in &m_oldnew {
                            mats2[*new] = am[*old].clone();
                        }
                        acjf.materials = Some(mats2);
                    }
                    if a.textures.is_some() {
                        let at = a.textures.as_ref().unwrap();
                        let mut texs2: Vec<Value> = Vec::new();
                        texs2.resize(t_oldnew.len(), json!(null));
                        for (old, new) in &t_oldnew {
                            texs2[*new] = at[*old].clone();
                        }
                        acjf.textures = Some(texs2);
                    }
                    if a.vertices_texture.is_some() {
                        let atv = a.vertices_texture.as_ref().unwrap();
                        let mut t_new_vertices: Vec<Vec<f64>> = Vec::new();
                        t_new_vertices.resize(t_v_oldnew.len(), vec![]);
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
                let mut mats2: Vec<Value> = Vec::new();
                mats2.resize(m_oldnew.len(), json!(null));
                for (old, new) in &m_oldnew {
                    mats2[*new] = am[*old].clone();
                }
                acjf.materials = Some(mats2);
            }
            if a.textures.is_some() {
                let at = a.textures.as_ref().unwrap();
                let mut texs2: Vec<Value> = Vec::new();
                texs2.resize(t_oldnew.len(), json!(null));
                for (old, new) in &t_oldnew {
                    texs2[*new] = at[*old].clone();
                }
                acjf.textures = Some(texs2);
            }
            if a.vertices_texture.is_some() {
                let atv = a.vertices_texture.as_ref().unwrap();
                let mut t_new_vertices: Vec<Vec<f64>> = Vec::new();
                t_new_vertices.resize(t_v_oldnew.len(), vec![]);
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
    fn add_vertices_texture(&mut self, vs: Vec<Vec<f64>>) {
        match &mut self.appearance {
            Some(x) => x.add_vertices_texture(vs),
            None => {
                let mut a: Appearance = Appearance::new();
                a.add_vertices_texture(vs);
                self.appearance = Some(a);
            }
        };
    }
    fn add_material(&mut self, jm: Value) -> usize {
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
    fn add_texture(&mut self, jm: Value) -> usize {
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CityJSONFeature {
    #[serde(rename = "type")]
    pub thetype: String,
    pub id: String,
    #[serde(rename = "CityObjects")]
    pub city_objects: HashMap<String, CityObject>,
    pub vertices: Vec<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub appearance: Option<Appearance>,
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
        }
    }
    pub fn from_str(s: &str) -> Result<Self, Error> {
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CityObject {
    #[serde(rename = "type")]
    pub thetype: String,
    #[serde(rename = "geographicalExtent")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geographical_extent: Option<Vec<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geometry: Option<Vec<Geometry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parents: Option<Vec<String>>,
    #[serde(flatten)]
    other: serde_json::Value,
}

impl CityObject {
    pub fn new(
        thetype: String,
        geographical_extent: Option<Vec<f64>>,
        attributes: Option<Value>,
        geometry: Option<Vec<Geometry>>,
        children: Option<Vec<String>>,
        parents: Option<Vec<String>>,
        other: Option<Value>,
    ) -> Self {
        CityObject {
            thetype,
            geographical_extent,
            attributes,
            geometry,
            children,
            parents,
            other: other.unwrap_or(Value::Null),
        }
    }
    pub fn get_type(&self) -> String {
        self.thetype.clone()
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
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
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
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Geometry {
    #[serde(rename = "type")]
    pub thetype: GeometryType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lod: Option<String>,
    pub boundaries: Boundaries,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantics: Option<Semantics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub material: Option<HashMap<String, Material>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture: Option<HashMap<String, Texture>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<usize>,
    #[serde(rename = "transformationMatrix")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transformation_matrix: Option<Value>,
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
                    //-- else it's material.values (which differs per geom type)
                    match self.thetype {
                        GeometryType::MultiPoint | GeometryType::MultiLineString => (),
                        GeometryType::MultiSurface | GeometryType::CompositeSurface => {
                            if mat.values.is_some() {
                                let a: Vec<Option<usize>> =
                                    serde_json::from_value(mat.values.take().into()).unwrap();
                                let mut a2 = a.clone();
                                for (i, x) in a.iter().enumerate() {
                                    if x.is_some() {
                                        let y2 = m_oldnew.get(&x.unwrap());
                                        if y2.is_none() {
                                            let l = m_oldnew.len();
                                            m_oldnew.insert(x.unwrap(), l);
                                            a2[i] = Some(l);
                                        } else {
                                            let y2 = y2.unwrap();
                                            a2[i] = Some(*y2);
                                        }
                                    }
                                }
                                mat.values = Some(serde_json::to_value(&a2).unwrap());
                            }
                        }
                        GeometryType::Solid => {
                            if mat.values.is_some() {
                                let a: Vec<Vec<Option<usize>>> =
                                    serde_json::from_value(mat.values.take().into()).unwrap();
                                let mut a2 = a.clone();
                                for (i, x) in a.iter().enumerate() {
                                    for (j, y) in x.iter().enumerate() {
                                        if y.is_some() {
                                            let y2 = m_oldnew.get(&y.unwrap());
                                            if y2.is_none() {
                                                let l = m_oldnew.len();
                                                m_oldnew.insert(y.unwrap(), l);
                                                a2[i][j] = Some(l);
                                            } else {
                                                let y2 = y2.unwrap();
                                                a2[i][j] = Some(*y2);
                                            }
                                        }
                                    }
                                }
                                mat.values = Some(serde_json::to_value(&a2).unwrap());
                            }
                        }
                        GeometryType::MultiSolid | GeometryType::CompositeSolid => {
                            if mat.values.is_some() {
                                let a: Vec<Vec<Vec<Option<usize>>>> =
                                    serde_json::from_value(mat.values.take().into()).unwrap();
                                let mut a2 = a.clone();
                                for (i, x) in a.iter().enumerate() {
                                    for (j, y) in x.iter().enumerate() {
                                        for (k, z) in y.iter().enumerate() {
                                            if z.is_some() {
                                                let y2 = m_oldnew.get(&z.unwrap());
                                                if y2.is_none() {
                                                    let l = m_oldnew.len();
                                                    m_oldnew.insert(z.unwrap(), l);
                                                    a2[i][j][k] = Some(l);
                                                } else {
                                                    let y2 = y2.unwrap();
                                                    a2[i][j][k] = Some(*y2);
                                                }
                                            }
                                        }
                                    }
                                }
                                mat.values = Some(serde_json::to_value(&a2).unwrap());
                            }
                        }
                        GeometryType::GeometryInstance => todo!(),
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
                    match self.thetype {
                        GeometryType::MultiSurface | GeometryType::CompositeSurface => {
                            let a: Vec<Vec<Vec<Option<usize>>>> =
                                serde_json::from_value(tex.values.take().into()).unwrap();
                            let mut a2 = a.clone();
                            for (i, x) in a.iter().enumerate() {
                                for (j, y) in x.iter().enumerate() {
                                    for (k, z) in y.iter().enumerate() {
                                        if z.is_some() {
                                            let thevalue: usize = z.unwrap();
                                            if k == 0 {
                                                let y2 = t_oldnew.get(&thevalue);
                                                if y2.is_none() {
                                                    let l = t_oldnew.len();
                                                    t_oldnew.insert(thevalue, l);
                                                    a2[i][j][k] = Some(l);
                                                } else {
                                                    let y2 = y2.unwrap();
                                                    a2[i][j][k] = Some(*y2);
                                                }
                                            } else {
                                                let y2 = t_v_oldnew.get(&thevalue);
                                                if y2.is_none() {
                                                    let l = t_v_oldnew.len();
                                                    t_v_oldnew.insert(thevalue, l + offset);
                                                    a2[i][j][k] = Some(l);
                                                } else {
                                                    let y2 = y2.unwrap();
                                                    a2[i][j][k] = Some(*y2);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            tex.values = Some(serde_json::to_value(&a2).unwrap());
                        }
                        GeometryType::Solid => {
                            let a: Vec<Vec<Vec<Vec<Option<usize>>>>> =
                                serde_json::from_value(tex.values.take().into()).unwrap();
                            let mut a2 = a.clone();
                            for (i, x) in a.iter().enumerate() {
                                for (j, y) in x.iter().enumerate() {
                                    for (k, z) in y.iter().enumerate() {
                                        for (l, zz) in z.iter().enumerate() {
                                            if zz.is_some() {
                                                let thevalue: usize = zz.unwrap();
                                                if l == 0 {
                                                    let y2 = t_oldnew.get(&thevalue);
                                                    if y2.is_none() {
                                                        let l2 = t_oldnew.len();
                                                        t_oldnew.insert(thevalue, l2);
                                                        a2[i][j][k][l] = Some(l2);
                                                    } else {
                                                        let y2 = y2.unwrap();
                                                        a2[i][j][k][l] = Some(*y2);
                                                    }
                                                } else {
                                                    let y2 = t_v_oldnew.get(&thevalue);
                                                    if y2.is_none() {
                                                        let l2 = t_v_oldnew.len();
                                                        t_v_oldnew.insert(thevalue, l2 + offset);
                                                        a2[i][j][k][l] = Some(l2);
                                                    } else {
                                                        let y2 = y2.unwrap();
                                                        a2[i][j][k][l] = Some(*y2);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            tex.values = Some(serde_json::to_value(&a2).unwrap());
                        }
                        _ => todo!(),
                    }
                }
            }
            None => (),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
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
#[derive(Debug, Clone)]
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
    pub fn from_url(url: &str) -> Result<Self, &'static str> {
        if !url.contains("//www.opengis.net/def/crs") {
            return Err("Invalid reference system URL");
        }

        let i = url.find("crs").unwrap();
        let s = &url[i + 4..];

        let parts: Vec<&str> = s.split("/").collect();
        if parts.len() != 3 {
            return Err("Invalid reference system URL");
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
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_url().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ReferenceSystem {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let url = String::deserialize(deserializer)?;
        ReferenceSystem::from_url(&url).map_err(serde::de::Error::custom)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeometryTemplates {
    pub templates: Vec<Geometry>,
    #[serde(rename = "vertices-templates")]
    pub vertices_templates: Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Material {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Texture {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Appearance {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub materials: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub textures: Option<Vec<Value>>,
    #[serde(rename = "vertices-texture")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertices_texture: Option<Vec<Vec<f64>>>,
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
    fn add_material(&mut self, jm: Value) -> usize {
        let re = match &mut self.materials {
            Some(x) => match x.iter().position(|e| *e == jm) {
                Some(y) => y,
                None => {
                    x.push(jm);
                    x.len() - 1
                }
            },
            None => {
                let mut ls: Vec<Value> = Vec::new();
                ls.push(jm);
                self.materials = Some(ls);
                0
            }
        };
        re
    }
    fn add_texture(&mut self, jm: Value) -> usize {
        let re = match &mut self.textures {
            Some(x) => match x.iter().position(|e| *e == jm) {
                Some(y) => y,
                None => {
                    x.push(jm);
                    x.len() - 1
                }
            },
            None => {
                let mut ls: Vec<Value> = Vec::new();
                ls.push(jm);
                self.textures = Some(ls);
                0
            }
        };
        re
    }
    fn add_vertices_texture(&mut self, mut vs: Vec<Vec<f64>>) {
        match &mut self.vertices_texture {
            Some(x) => {
                x.append(&mut vs);
            }
            None => {
                let mut ls: Vec<Vec<f64>> = Vec::new();
                ls.append(&mut vs);
                self.vertices_texture = Some(ls);
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Test cases derived from CityJSON specification v2.0.1
    /// See: https://www.cityjson.org/specs/2.0.1/#semantics-of-geometric-primitives

    /// MultiPoint: Single array of vertex indices
    /// [v1, v2, v3, ...]
    #[test]
    fn test_multipoint_boundaries() {
        let json_value = json!([2, 44, 0, 7]);
        let boundaries = parse_nested_array(&json_value);
        assert_eq!(boundaries, NestedArray::Indices(vec![2, 44, 0, 7]));
    }

    /// MultiLineString: Array of arrays, each inner array represents a linestring
    /// [[v1, v2, v3, ...], [v1, v2, v3, ...], ...]
    #[test]
    fn test_multilinestring_boundaries() {
        let json_value = json!([[2, 3, 5], [77, 55, 212]]);
        let boundaries = parse_nested_array(&json_value);
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
        let boundaries = parse_nested_array(&json_value);
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
        let boundaries = parse_nested_array(&json_value);
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
        let boundaries = parse_nested_array(&json_value);
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
                ]),]),
            ])
        );
    }
}
