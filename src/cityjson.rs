use crate::appearance::{Appearance, MaterialKey, MaterialObject, TextureKey, TextureObject};
use crate::city_object::{CityObject, CityObjectType};
use crate::error::{CjseqError, IndexKind, Result};
use crate::geometry::{Geometry, GeometryTemplates};
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
        } else if let Some(a) = &self.appearance {
            //-- no templates, so no palette to carry, but the default themes
            //-- are the document's and travel with the header
            if a.default_theme_material.is_some() || a.default_theme_texture.is_some() {
                let mut defaults = Appearance::new();
                defaults.default_theme_material = a.default_theme_material.clone();
                defaults.default_theme_texture = a.default_theme_texture.clone();
                cj0.appearance = Some(defaults);
            }
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

    /// Whether City Object `co` roots a feature: it has no parents, or only
    /// CityObjectGroups as parents (a group's members are not its parts).
    fn is_root(&self, co: &CityObject) -> bool {
        co.parents.iter().flatten().all(|p| {
            self.city_objects
                .get(p)
                .is_some_and(|pco| pco.thetype == CityObjectType::CityObjectGroup)
        })
    }

    /// The ids of the City Objects that root a feature, in `order`.
    fn feature_ids(&self, order: SortingStrategy) -> Vec<&String> {
        let mut ids: Vec<&String> = self
            .city_objects
            .iter()
            .filter(|(_, co)| self.is_root(co))
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

    /// The document's features, one per City Object that roots one, in
    /// `order`; see [`CityJSON::feature`].
    ///
    /// Every City Object must land in a feature. If one cannot, because its
    /// parent is missing or does not list it as a child, the only item is a
    /// [`CjseqError::NotInAnyFeature`] naming them all, so nothing is
    /// written partially.
    pub fn features(
        &self,
        order: SortingStrategy,
    ) -> Box<dyn Iterator<Item = Result<CityJSONFeature>> + '_> {
        let plan: Result<Vec<Vec<(&String, &CityObject)>>> = self
            .feature_ids(order)
            .into_iter()
            .map(|id| self.members(id, &self.city_objects[id]))
            .collect();
        let plan = match plan {
            Ok(p) => p,
            Err(e) => return Box::new(std::iter::once(Err(e))),
        };
        let covered: HashSet<&str> = plan.iter().flatten().map(|(id, _)| id.as_str()).collect();
        let missing: Vec<String> = self
            .city_objects
            .keys()
            .filter(|id| !covered.contains(id.as_str()))
            .cloned()
            .collect();
        if !missing.is_empty() {
            return Box::new(std::iter::once(Err(CjseqError::NotInAnyFeature {
                ids: missing,
            })));
        }
        Box::new(plan.into_iter().map(move |m| self.slice(m)))
    }

    /// The feature rooted at City Object `id`: it with all of its
    /// descendants, and only the vertices and appearance entries they use,
    /// renumbered. A CityObjectGroup's feature holds just the group.
    /// `Ok(None)` if there is no such City Object.
    ///
    /// A child id that names no City Object, or an index past the end of
    /// `vertices` or of an appearance palette, is an error.
    pub fn feature(&self, id: &str) -> Result<Option<CityJSONFeature>> {
        match self.city_objects.get_key_value(id) {
            Some((id, root)) => self.slice(self.members(id, root)?).map(Some),
            None => Ok(None),
        }
    }

    /// The root, then every descendant breadth-first. A group's members
    /// are not followed; `seen` stops a parent/child cycle from looping.
    fn members<'a>(
        &'a self,
        id: &'a String,
        root: &'a CityObject,
    ) -> Result<Vec<(&'a String, &'a CityObject)>> {
        let mut members: Vec<(&String, &CityObject)> = vec![(id, root)];
        let mut seen: HashSet<&str> = HashSet::from([id.as_str()]);
        let mut next = 0;
        while next < members.len() {
            let (parent_id, parent) = members[next];
            next += 1;
            if parent.thetype == CityObjectType::CityObjectGroup {
                continue;
            }
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
        Ok(members)
    }

    fn slice(&self, members: Vec<(&String, &CityObject)>) -> Result<CityJSONFeature> {
        let id = members[0].0;
        let mut cjf = CityJSONFeature::new();
        cjf.id = id.clone();
        let mut g_vi_oldnew: HashMap<VertexIndex, VertexIndex> = HashMap::new();
        let mut m_oldnew: HashMap<MaterialIndex, MaterialIndex> = HashMap::new();
        let mut t_oldnew: HashMap<TextureIndex, TextureIndex> = HashMap::new();
        let mut t_v_oldnew: HashMap<UvIndex, UvIndex> = HashMap::new();
        for (key, co) in members {
            let mut co2 = co.clone();
            for g in co2.geometries_mut() {
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
            for g in co.geometries_mut() {
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
            .filter(|co| self.is_root(co))
            .count()
    }
}

/// The first position of each key.
fn positions<K: std::hash::Hash + Eq>(keys: impl Iterator<Item = K>) -> HashMap<K, usize> {
    let mut m = HashMap::new();
    for (i, k) in keys.enumerate() {
        m.entry(k).or_insert(i);
    }
    m
}

/// Collects a CityJSONSeq back into one CityJSON document: start from the
/// header, [`Collector::add`] each feature, then [`Collector::finish`].
///
/// Features from several sequences can be collected into one document: hand
/// each further sequence's header to [`Collector::add_header`], and the
/// features that follow are re-expressed in the first header's transform.
pub struct Collector {
    doc: CityJSON,
    /// The transform the features being added are expressed in, when it is
    /// not the document's.
    correction: Option<Transform>,
    /// Where the current sequence's geometry templates start in the
    /// document's.
    template_offset: usize,
    /// Where each distinct material and texture already is in the
    /// document's palettes.
    materials: HashMap<MaterialKey, usize>,
    textures: HashMap<TextureKey, usize>,
    warnings: Vec<CjseqError>,
}

/// How one source's appearance indices map onto the document's palettes.
#[derive(Default)]
struct PaletteMaps {
    materials: HashMap<MaterialIndex, MaterialIndex>,
    textures: HashMap<TextureIndex, TextureIndex>,
    uvs: HashMap<UvIndex, UvIndex>,
}

impl Collector {
    pub fn new(header: CityJSON) -> Self {
        let app = header.appearance.as_ref();
        let materials = positions(
            app.and_then(|a| a.materials.as_ref())
                .into_iter()
                .flatten()
                .map(MaterialObject::key),
        );
        let textures = positions(
            app.and_then(|a| a.textures.as_ref())
                .into_iter()
                .flatten()
                .map(TextureObject::key),
        );
        Collector {
            doc: header,
            correction: None,
            template_offset: 0,
            materials,
            textures,
            warnings: Vec::new(),
        }
    }

    /// Merge `app` into the document's appearance: identical materials and
    /// textures become one entry, texture vertices are appended, and the
    /// default themes are adopted when the document has none. A default
    /// theme that disagrees with the document's is dropped, with a warning
    /// made by `dropped`.
    fn merge_appearance(
        &mut self,
        app: Appearance,
        dropped: &dyn Fn(String) -> CjseqError,
    ) -> PaletteMaps {
        let mut maps = PaletteMaps::default();
        let doc_app = self.doc.appearance.get_or_insert_with(Appearance::new);
        for (name, mine, theirs) in [
            (
                "appearance.default-theme-material",
                &mut doc_app.default_theme_material,
                app.default_theme_material,
            ),
            (
                "appearance.default-theme-texture",
                &mut doc_app.default_theme_texture,
                app.default_theme_texture,
            ),
        ] {
            match (mine.as_ref(), theirs) {
                (None, Some(t)) => *mine = Some(t),
                (Some(m), Some(t)) if *m != t => {
                    self.warnings.push(dropped(format!("{name} {t:?}")))
                }
                _ => {}
            }
        }
        for (i, m) in app.materials.into_iter().flatten().enumerate() {
            let all = doc_app.materials.get_or_insert_with(Vec::new);
            let at = *self.materials.entry(m.key()).or_insert_with(|| {
                all.push(m);
                all.len() - 1
            });
            maps.materials.insert(MaterialIndex(i), MaterialIndex(at));
        }
        for (i, t) in app.textures.into_iter().flatten().enumerate() {
            let all = doc_app.textures.get_or_insert_with(Vec::new);
            let at = *self.textures.entry(t.key()).or_insert_with(|| {
                all.push(t);
                all.len() - 1
            });
            maps.textures.insert(TextureIndex(i), TextureIndex(at));
        }
        if let Some(uvs) = app.vertices_texture {
            //-- appended verbatim, so each new index is the old one shifted by
            //-- however many were already collected
            let offset = doc_app.vertices_texture.as_ref().map_or(0, Vec::len);
            for i in 0..uvs.len() {
                maps.uvs.insert(UvIndex(i), UvIndex(offset + i));
            }
            doc_app.add_vertices_texture(uvs);
        }
        maps
    }

    /// The header of a further sequence, whose features follow. Its
    /// transform applies to them, so their vertices are re-expressed in the
    /// document's; its geometry templates are appended to the document's
    /// (with their appearance merged into the document's palettes), so its
    /// features' GeometryInstances keep pointing at the right ones; its
    /// extensions are added.
    ///
    /// A different CityJSON version, or an extension of the same name with
    /// a different URL or version, is an error. Metadata or other root
    /// members that differ from the document's are dropped, with a warning.
    pub fn add_header(&mut self, mut header: CityJSON) -> Result<()> {
        if header.version != self.doc.version {
            return Err(CjseqError::HeaderMismatch {
                member: format!("version {} vs {}", header.version, self.doc.version),
            });
        }
        for (name, ext) in header.extensions.take().into_iter().flatten() {
            let mine = self.doc.extensions.get_or_insert_with(IndexMap::new);
            match mine.get(&name) {
                Some(e) if *e != ext => {
                    return Err(CjseqError::HeaderMismatch {
                        member: format!("extensions.{name}"),
                    })
                }
                Some(_) => {}
                None => {
                    mine.insert(name, ext);
                }
            }
        }
        let header_dropped = |member: String| CjseqError::DroppedHeaderMember { member };
        if header.metadata.is_some() && header.metadata != self.doc.metadata {
            self.warnings.push(header_dropped("metadata".to_string()));
        }
        for (k, v) in std::mem::take(&mut header.other) {
            match self.doc.other.get(&k) {
                None => {
                    self.doc.other.insert(k, v);
                }
                Some(mine) if *mine != v => self.warnings.push(header_dropped(k)),
                Some(_) => {}
            }
        }
        let maps = match header.appearance.take() {
            Some(app) => self.merge_appearance(app, &header_dropped),
            None => PaletteMaps::default(),
        };
        let doc_templates = self
            .doc
            .geometry_templates
            .get_or_insert_with(|| GeometryTemplates {
                templates: Vec::new(),
                vertices_templates: Vec::new(),
            });
        self.template_offset = doc_templates.templates.len();
        if let Some(gt) = header.geometry_templates.take() {
            let v_offset = doc_templates.vertices_templates.len();
            let mut maps = maps;
            for mut g in gt.templates {
                g.offset_geometry_boundaries(v_offset);
                g.update_material(&mut maps.materials);
                g.update_texture(&mut maps.textures, &mut maps.uvs);
                doc_templates.templates.push(g);
            }
            doc_templates
                .vertices_templates
                .extend(gt.vertices_templates);
        }
        if doc_templates.templates.is_empty() {
            self.doc.geometry_templates = None;
        }
        self.correction = (header.transform != self.doc.transform).then_some(header.transform);
        Ok(())
    }

    /// Every index in `cjf` must point into its own vertices and palettes,
    /// or into the current sequence's templates: once merged, an index past
    /// the end would silently point at another feature's data.
    fn check_indices(&self, cjf: &mut CityJSONFeature) -> Result<()> {
        let len = |v: Option<usize>| v.unwrap_or(0);
        let app = cjf.appearance.as_ref();
        let materials = len(app.and_then(|a| a.materials.as_ref()).map(Vec::len));
        let textures = len(app.and_then(|a| a.textures.as_ref()).map(Vec::len));
        let uvs = len(app.and_then(|a| a.vertices_texture.as_ref()).map(Vec::len));
        let templates = self
            .doc
            .geometry_templates
            .as_ref()
            .map_or(0, |t| t.templates.len())
            - self.template_offset;
        let vertices = cjf.vertices.len();
        for (id, co) in &mut cjf.city_objects {
            let mut err: Option<CjseqError> = None;
            let mut check = |kind: IndexKind, index: usize, len: usize| {
                if index >= len && err.is_none() {
                    err = Some(CjseqError::IndexOutOfRange { kind, index, len });
                }
            };
            for g in co.geometries_mut() {
                g.visit_vertices(&mut |v| check(IndexKind::Vertex, v.0, vertices));
                g.for_each_material(&mut |m| check(IndexKind::Material, m.0, materials));
                g.for_each_texture(&mut |t: &mut TextureIndex| {
                    check(IndexKind::Texture, t.0, textures)
                });
                g.for_each_texture(&mut |uv: &mut UvIndex| {
                    check(IndexKind::TextureVertex, uv.0, uvs)
                });
                if let Geometry::GeometryInstance { template, .. } = g {
                    check(IndexKind::Template, template.0, templates);
                }
            }
            if let Some(e) = err {
                return Err(CjseqError::InCityObject {
                    id: id.clone(),
                    source: Box::new(e),
                });
            }
        }
        Ok(())
    }

    /// Add one feature: its City Objects, its vertices (re-expressed if a
    /// later header changed the transform) and its appearance, merged into
    /// the document's palettes. Root members the feature schema allows but
    /// does not name are dropped, each with a [`Collector::warnings`] entry.
    ///
    /// A City Object id already collected from an earlier feature is a
    /// [`CjseqError::DuplicateId`], and nothing of the feature is added.
    pub fn add(&mut self, mut cjf: CityJSONFeature) -> Result<()> {
        if !cjf.city_objects.contains_key(&cjf.id) {
            return Err(CjseqError::DanglingReference {
                from: "CityJSONFeature.id".to_string(),
                to: cjf.id,
            });
        }
        if let Some(id) = cjf
            .city_objects
            .keys()
            .find(|id| self.doc.city_objects.contains_key(*id))
        {
            return Err(CjseqError::DuplicateId { id: id.clone() });
        }
        self.check_indices(&mut cjf)?;
        for member in cjf.other.keys() {
            self.warnings.push(CjseqError::DroppedMember {
                feature: cjf.id.clone(),
                member: member.clone(),
            });
        }
        let id = cjf.id.clone();
        let mut maps = match cjf.appearance.take() {
            Some(app) => self.merge_appearance(app, &|member| CjseqError::DroppedMember {
                feature: id.clone(),
                member,
            }),
            None => PaletteMaps::default(),
        };
        let doc = &mut self.doc;
        let g_offset = doc.vertices.len();
        for (key, mut co) in cjf.city_objects {
            for g in co.geometries_mut() {
                g.offset_geometry_boundaries(g_offset);
                g.offset_template(self.template_offset);
                g.update_material(&mut maps.materials);
                g.update_texture(&mut maps.textures, &mut maps.uvs);
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

    /// What collecting has had to drop so far.
    pub fn warnings(&self) -> &[CjseqError] {
        &self.warnings
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

impl std::fmt::Display for CityJSONVersion {
    /// The version as the `version` member spells it, eg `2.0`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            CityJSONVersion::V1_1 => "1.1",
            CityJSONVersion::V2_0 => "2.0",
        })
    }
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
