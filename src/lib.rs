use serde::{Deserialize, Serialize};
use serde_json::{json, Error, Value};
use std::collections::HashMap;

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
    pub metadata: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub appearance: Option<Appearance>,
    #[serde(rename = "geometry-templates")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geometry_templates: Option<GeometryTemplates>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Value>,
    #[serde(flatten)]
    other: serde_json::Value,
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
        let cjj: CityJSON = serde_json::from_str(s)?;
        Ok(cjj)
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
            sorted_ids: vec![],
        }
    }
    pub fn cat_metadata(&self) -> Self {
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
    pub fn cat_feature(&self, i: usize) -> Option<CityJSONFeature> {
        let i2 = self.sorted_ids.get(i);
        // println!("{:?}", i2);
        if i2.is_none() {
            return None;
        }
        let obj = self.city_objects.get(i2.unwrap());
        if obj.is_none() {
            return None;
        }
        // println!("{:?}", obj);
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
        // io::stdout()
        //     .write_all(&format!("{}\n", serde_json::to_string(&cjf).unwrap()).as_bytes())?;
    }
    pub fn add_one_cjf(&mut self, mut cjf: CityJSONFeature) {
        // let mut g_oldnew: HashMap<usize, usize> = HashMap::new();
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
        self.add_vertices(cjf.vertices.clone());
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
    pub fn retransform(&mut self) {
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
    pub fn sort_features(&mut self, ss: SortingStrategy) {
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
    fn add_vertices(&mut self, mut v: Vec<Vec<i64>>) {
        self.vertices.append(&mut v);
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Geometry {
    #[serde(rename = "type")]
    thetype: GeometryType,
    #[serde(skip_serializing_if = "Option::is_none")]
    lod: Option<String>,
    boundaries: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    semantics: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    material: Option<HashMap<String, Material>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    texture: Option<HashMap<String, Texture>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    template: Option<usize>,
    #[serde(rename = "transformationMatrix")]
    #[serde(skip_serializing_if = "Option::is_none")]
    transformation_matrix: Option<Value>,
}
impl Geometry {
    pub fn update_geometry_boundaries(&mut self, violdnew: &mut HashMap<usize, usize>) {
        match self.thetype {
            GeometryType::MultiPoint => {
                let a: Vec<usize> = serde_json::from_value(self.boundaries.clone()).unwrap();
                let mut a2 = a.clone();
                for (i, x) in a.iter().enumerate() {
                    let kk = violdnew.get(&x);
                    if kk.is_none() {
                        let l = violdnew.len();
                        violdnew.insert(*x, l);
                        a2[i] = l;
                    } else {
                        let kk = kk.unwrap();
                        a2[i] = *kk;
                    }
                }
                self.boundaries = serde_json::to_value(&a2).unwrap();
            }
            GeometryType::MultiLineString => {
                let a: Vec<Vec<usize>> = serde_json::from_value(self.boundaries.take()).unwrap();
                let mut a2 = a.clone();
                for (i, x) in a.iter().enumerate() {
                    for (j, y) in x.iter().enumerate() {
                        // r.push(z);
                        let kk = violdnew.get(&y);
                        if kk.is_none() {
                            let l = violdnew.len();
                            violdnew.insert(*y, l);
                            a2[i][j] = l;
                        } else {
                            let kk = kk.unwrap();
                            a2[i][j] = *kk;
                        }
                    }
                }
                self.boundaries = serde_json::to_value(&a2).unwrap();
            }
            GeometryType::MultiSurface | GeometryType::CompositeSurface => {
                let a: Vec<Vec<Vec<usize>>> =
                    serde_json::from_value(self.boundaries.take()).unwrap();
                let mut a2 = a.clone();
                for (i, x) in a.iter().enumerate() {
                    for (j, y) in x.iter().enumerate() {
                        for (k, z) in y.iter().enumerate() {
                            let kk = violdnew.get(&z);
                            if kk.is_none() {
                                let l = violdnew.len();
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
            }
            GeometryType::Solid => {
                let a: Vec<Vec<Vec<Vec<usize>>>> =
                    serde_json::from_value(self.boundaries.take()).unwrap();
                let mut a2 = a.clone();
                for (i, x) in a.iter().enumerate() {
                    for (j, y) in x.iter().enumerate() {
                        for (k, z) in y.iter().enumerate() {
                            for (l, zz) in z.iter().enumerate() {
                                let kk = violdnew.get(&zz);
                                if kk.is_none() {
                                    let l2 = violdnew.len();
                                    violdnew.insert(*zz, l2);
                                    a2[i][j][k][l] = l2;
                                } else {
                                    let kk = kk.unwrap();
                                    a2[i][j][k][l] = *kk;
                                }
                            }
                        }
                    }
                }
                self.boundaries = serde_json::to_value(&a2).unwrap();
            }
            GeometryType::MultiSolid | GeometryType::CompositeSolid => {
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
                                        let l2 = violdnew.len();
                                        violdnew.insert(*zzz, l2);
                                        a2[i][j][k][l][m] = l2;
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
            GeometryType::GeometryInstance => {
                let a: Vec<usize> = serde_json::from_value(self.boundaries.clone()).unwrap();
                let mut a2 = a.clone();
                for (i, x) in a.iter().enumerate() {
                    let kk = violdnew.get(&x);
                    if kk.is_none() {
                        let l = violdnew.len();
                        violdnew.insert(*x, l);
                        a2[i] = l;
                    } else {
                        let kk = kk.unwrap();
                        a2[i] = *kk;
                    }
                }
                self.boundaries = serde_json::to_value(&a2).unwrap();
            }
        }
    }

    pub fn offset_geometry_boundaries(&mut self, offset: usize) {
        match self.thetype {
            GeometryType::MultiPoint => {
                let a: Vec<usize> = serde_json::from_value(self.boundaries.clone()).unwrap();
                let mut a2 = a.clone();
                for (i, x) in a.iter().enumerate() {
                    a2[i] = *x + offset;
                }
                self.boundaries = serde_json::to_value(&a2).unwrap();
            }
            GeometryType::MultiLineString => {
                let a: Vec<Vec<usize>> = serde_json::from_value(self.boundaries.take()).unwrap();
                let mut a2 = a.clone();
                for (i, x) in a.iter().enumerate() {
                    for (j, y) in x.iter().enumerate() {
                        // r.push(z);
                        a2[i][j] = *y + offset;
                    }
                }
                self.boundaries = serde_json::to_value(&a2).unwrap();
            }
            GeometryType::MultiSurface | GeometryType::CompositeSurface => {
                let a: Vec<Vec<Vec<usize>>> =
                    serde_json::from_value(self.boundaries.take()).unwrap();
                let mut a2 = a.clone();
                for (i, x) in a.iter().enumerate() {
                    for (j, y) in x.iter().enumerate() {
                        for (k, z) in y.iter().enumerate() {
                            a2[i][j][k] = *z + offset;
                        }
                    }
                }
                self.boundaries = serde_json::to_value(&a2).unwrap();
            }
            GeometryType::Solid => {
                let a: Vec<Vec<Vec<Vec<usize>>>> =
                    serde_json::from_value(self.boundaries.take()).unwrap();
                let mut a2 = a.clone();
                for (i, x) in a.iter().enumerate() {
                    for (j, y) in x.iter().enumerate() {
                        for (k, z) in y.iter().enumerate() {
                            for (l, zz) in z.iter().enumerate() {
                                a2[i][j][k][l] = *zz + offset;
                            }
                        }
                    }
                }
                self.boundaries = serde_json::to_value(&a2).unwrap();
            }
            GeometryType::MultiSolid | GeometryType::CompositeSolid => {
                let a: Vec<Vec<Vec<Vec<Vec<usize>>>>> =
                    serde_json::from_value(self.boundaries.take()).unwrap();
                let mut a2 = a.clone();
                for (i, x) in a.iter().enumerate() {
                    for (j, y) in x.iter().enumerate() {
                        for (k, z) in y.iter().enumerate() {
                            for (l, zz) in z.iter().enumerate() {
                                for (m, zzz) in zz.iter().enumerate() {
                                    a2[i][j][k][l][m] = *zzz + offset;
                                }
                            }
                        }
                    }
                }
                self.boundaries = serde_json::to_value(&a2).unwrap();
            }
            GeometryType::GeometryInstance => {
                let a: Vec<usize> = serde_json::from_value(self.boundaries.clone()).unwrap();
                let mut a2 = a.clone();
                for (i, x) in a.iter().enumerate() {
                    a2[i] = *x + offset;
                }
                self.boundaries = serde_json::to_value(&a2).unwrap();
            }
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
    pub fn update_texture(
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

#[derive(Serialize, Deserialize, Debug)]
struct Vertex {
    x: i64,
    y: i64,
    z: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transform {
    pub scale: Vec<f64>,
    pub translate: Vec<f64>,
}
impl Transform {
    pub fn new() -> Self {
        Transform {
            scale: vec![1.0, 1.0, 1.0],
            translate: vec![0., 0., 0.],
        }
    }
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
