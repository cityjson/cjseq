use serde_derive::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CityJSON {
    #[serde(rename = "type")]
    pub thetype: String,
    pub version: String,
    pub transform: Value,
    #[serde(rename = "CityObjects")]
    pub city_objects: HashMap<String, CityObject>,
    pub vertices: Vec<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub appearance: Option<Appearance>,
    #[serde(rename = "geometry-templates")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geometry_templates: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Value>,
    #[serde(flatten)]
    other: serde_json::Value,
}
impl CityJSON {
    pub fn new() -> Self {
        let co: HashMap<String, CityObject> = HashMap::new();
        let v: Vec<Vec<i64>> = Vec::new();
        let tr = json!({
          "transform": {
            "scale": [1.0, 1.0, 1.0],
            "translate": [0.0, 0.0, 0.0]
          },
        });
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
        }
    }
    pub fn get_empty_copy(&self) -> Self {
        let co: HashMap<String, CityObject> = HashMap::new();
        let v: Vec<Vec<i64>> = Vec::new();
        CityJSON {
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
        }
    }
    pub fn add_co(&mut self, id: String, co: CityObject) {
        self.city_objects.insert(id, co);
    }
    pub fn add_vertices(&mut self, mut v: Vec<Vec<i64>>) {
        self.vertices.append(&mut v);
    }
    pub fn add_vertices_texture(&mut self, vs: Vec<Vec<f64>>) {
        match &mut self.appearance {
            Some(x) => x.add_vertices_texture(vs),
            None => {
                let mut a: Appearance = Appearance::new();
                a.add_vertices_texture(vs);
                self.appearance = Some(a);
            }
        };
    }
    pub fn add_material(&mut self, jm: Value) -> usize {
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
    pub fn add_texture(&mut self, jm: Value) -> usize {
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
    pub fn add_one_cjf(&mut self, mut cjf: CityJSONFeature) {
        let mut g_oldnew: HashMap<usize, usize> = HashMap::new();
        let mut m_oldnew: HashMap<usize, usize> = HashMap::new();
        let mut t_oldnew: HashMap<usize, usize> = HashMap::new();
        let mut t_v_oldnew: HashMap<usize, usize> = HashMap::new();
        let g_offset = self.vertices.len();
        let mut t_offset = 0;
        if let Some(cjf_app) = &cjf.appearance {
            // println!("{:?}", cjf_app);
            if let Some(cjf_mat) = &cjf_app.materials {
                // println!("{:?}", cjf_mat);
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
                // TODO : add other Geometric primitives
                for g in geoms.iter_mut() {
                    //-- boundaries
                    g.update_geometry_boundaries(&mut g_oldnew, g_offset);
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
        self.add_vertices(cjf.vertices.clone());
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
    pub fn add_co(&mut self, id: String, co: CityObject) {
        self.city_objects.insert(id, co);
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
    pub fn is_toplevel(&self) -> bool {
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
    pub fn get_children_keys(&self) -> Vec<String> {
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Geometry {
    #[serde(rename = "type")]
    pub thetype: String,
    pub lod: String,
    pub boundaries: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantics: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub material: Option<HashMap<String, Material>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture: Option<HashMap<String, Texture>>,
}
impl Geometry {
    pub fn update_geometry_boundaries(
        &mut self,
        violdnew: &mut HashMap<usize, usize>,
        offset: usize,
    ) {
        // TODO: GeometryInstance?
        if self.thetype == "MultiPoint" {
            let a: Vec<usize> = serde_json::from_value(self.boundaries.clone()).unwrap();
            let mut a2 = a.clone();
            for (i, x) in a.iter().enumerate() {
                let kk = violdnew.get(&x);
                if kk.is_none() {
                    let l = violdnew.len() + offset;
                    violdnew.insert(*x, l);
                    a2[i] = l;
                } else {
                    let kk = kk.unwrap();
                    a2[i] = *kk;
                }
            }
            self.boundaries = serde_json::to_value(&a2).unwrap();
        } else if self.thetype == "MultiLineString" {
            let a: Vec<Vec<usize>> = serde_json::from_value(self.boundaries.take()).unwrap();
            let mut a2 = a.clone();
            for (i, x) in a.iter().enumerate() {
                for (j, y) in x.iter().enumerate() {
                    // r.push(z);
                    let kk = violdnew.get(&y);
                    if kk.is_none() {
                        let l = violdnew.len() + offset;
                        violdnew.insert(*y, l);
                        a2[i][j] = l;
                    } else {
                        let kk = kk.unwrap();
                        a2[i][j] = *kk;
                    }
                }
            }
            self.boundaries = serde_json::to_value(&a2).unwrap();
        } else if self.thetype == "MultiSurface" || self.thetype == "CompositeSurface" {
            let a: Vec<Vec<Vec<usize>>> = serde_json::from_value(self.boundaries.take()).unwrap();
            let mut a2 = a.clone();
            for (i, x) in a.iter().enumerate() {
                for (j, y) in x.iter().enumerate() {
                    for (k, z) in y.iter().enumerate() {
                        // r.push(z);
                        let kk = violdnew.get(&z);
                        if kk.is_none() {
                            let l = violdnew.len() + offset;
                            violdnew.insert(*z, l);
                            a2[i][j][k] = l;
                        } else {
                            let kk = kk.unwrap();
                            a2[i][j][k] = *kk;
                        }
                    }
                }
            }
            self.boundaries = serde_json::to_value(&a2).unwrap();
        } else if self.thetype == "Solid" {
            let a: Vec<Vec<Vec<Vec<usize>>>> =
                serde_json::from_value(self.boundaries.take()).unwrap();
            let mut a2 = a.clone();
            for (i, x) in a.iter().enumerate() {
                for (j, y) in x.iter().enumerate() {
                    for (k, z) in y.iter().enumerate() {
                        for (l, zz) in z.iter().enumerate() {
                            // r.push(z);
                            let kk = violdnew.get(&zz);
                            if kk.is_none() {
                                let l = violdnew.len() + offset;
                                violdnew.insert(*zz, l);
                                a2[i][j][k][l] = l;
                            } else {
                                let kk = kk.unwrap();
                                a2[i][j][k][l] = *kk;
                            }
                        }
                    }
                }
            }
            self.boundaries = serde_json::to_value(&a2).unwrap();
        } else if self.thetype == "MultiSolid" || self.thetype == "CompositeSolid" {
            let a: Vec<Vec<Vec<Vec<Vec<usize>>>>> =
                serde_json::from_value(self.boundaries.take()).unwrap();
            let mut a2 = a.clone();
            for (i, x) in a.iter().enumerate() {
                for (j, y) in x.iter().enumerate() {
                    for (k, z) in y.iter().enumerate() {
                        for (l, zz) in z.iter().enumerate() {
                            for (m, zzz) in zz.iter().enumerate() {
                                let kk = violdnew.get(&zzz);
                                if kk.is_none() {
                                    let l = violdnew.len() + offset;
                                    violdnew.insert(*zzz, l);
                                    a2[i][j][k][l][m] = l;
                                } else {
                                    let kk = kk.unwrap();
                                    a2[i][j][k][l][m] = *kk;
                                }
                            }
                        }
                    }
                }
            }
            self.boundaries = serde_json::to_value(&a2).unwrap();
        }
    }
    pub fn update_material(&mut self, m_oldnew: &mut HashMap<usize, usize>) {
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
                    if self.thetype == "MultiSurface" || self.thetype == "CompositeSurface" {
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
                    } else if self.thetype == "Solid" {
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
                    } else if self.thetype == "MultiSolid" || self.thetype == "CompositeSolid" {
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
                }
                self.material = Some(x.clone());
            }
            None => (),
        }
    }
    pub fn update_texture(
        &mut self,
        t_oldnew: &mut HashMap<usize, usize>,
        t_v_oldnew: &mut HashMap<usize, usize>,
        offset: usize,
    ) {
        match &mut self.texture {
            Some(x) => {
                for (_key, tex) in &mut *x {
                    //-- TODO: other Geometry type for textures
                    if self.thetype == "MultiSurface" || self.thetype == "CompositeSurface" {
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
                }
            }
            None => (),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Vertex {
    x: i64,
    y: i64,
    z: i64,
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
    pub fn new() -> Self {
        Appearance {
            materials: None,
            textures: None,
            vertices_texture: None,
            default_theme_texture: None,
            default_theme_material: None,
        }
    }
    pub fn add_material(&mut self, jm: Value) -> usize {
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
    pub fn add_texture(&mut self, jm: Value) -> usize {
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
    pub fn add_vertices_texture(&mut self, mut vs: Vec<Vec<f64>>) {
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
