use crate::index::{Indices, MaterialIndex, PrimitiveValues, TextureIndex, UvIndex};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// One ring's texture entry. In JSON it is `[texture, uv, uv, ...]` -- the
/// texture index first, then one texture-vertex index per vertex of the ring
/// -- or `[null]` for a ring with no texture.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(try_from = "Vec<Option<usize>>", into = "Vec<Option<usize>>")]
pub enum TexturedRing {
    /// `[null]`
    Untextured,
    /// `[texture, uvs...]`
    Textured {
        texture: TextureIndex,
        uvs: Vec<UvIndex>,
    },
}

impl TryFrom<Vec<Option<usize>>> for TexturedRing {
    type Error = crate::error::CjseqError;
    fn try_from(v: Vec<Option<usize>>) -> crate::error::Result<Self> {
        match v.split_first() {
            Some((None, [])) => Ok(TexturedRing::Untextured),
            Some((Some(texture), rest)) if !rest.is_empty() => Ok(TexturedRing::Textured {
                texture: TextureIndex(*texture),
                uvs: rest
                    .iter()
                    .map(|uv| {
                        uv.map(UvIndex).ok_or_else(|| {
                            crate::error::CjseqError::Validation(
                                "a textured ring has a null texture-vertex index".to_string(),
                            )
                        })
                    })
                    .collect::<crate::error::Result<_>>()?,
            }),
            _ => Err(crate::error::CjseqError::Validation(format!(
                "{v:?} is not a texture ring: expected [null] or [texture, uv, ...]"
            ))),
        }
    }
}

impl From<TexturedRing> for Vec<Option<usize>> {
    fn from(r: TexturedRing) -> Self {
        match r {
            TexturedRing::Untextured => vec![None],
            TexturedRing::Textured { texture, uvs } => std::iter::once(Some(texture.0))
                .chain(uvs.into_iter().map(|uv| Some(uv.0)))
                .collect(),
        }
    }
}
/// One face's rings, exterior first, mirroring [`crate::Surface`].
pub type TexturedSurface = Vec<TexturedRing>;
/// One volume's bounding faces, mirroring [`crate::Shell`].
pub type TexturedShell = Vec<TexturedSurface>;

impl Indices<TextureIndex> for TexturedRing {
    fn for_each_index(&mut self, f: &mut dyn FnMut(&mut TextureIndex)) {
        if let TexturedRing::Textured { texture, .. } = self {
            f(texture)
        }
    }
    fn visit_index(&self, f: &mut dyn FnMut(TextureIndex)) {
        if let TexturedRing::Textured { texture, .. } = self {
            f(*texture)
        }
    }
}

impl Indices<UvIndex> for TexturedRing {
    fn for_each_index(&mut self, f: &mut dyn FnMut(&mut UvIndex)) {
        if let TexturedRing::Textured { uvs, .. } = self {
            uvs.iter_mut().for_each(f)
        }
    }
    fn visit_index(&self, f: &mut dyn FnMut(UvIndex)) {
        if let TexturedRing::Textured { uvs, .. } = self {
            uvs.iter().for_each(|uv| f(*uv))
        }
    }
}

/// Keep an explicit `null` distinct from an absent member: a plain
/// `Option<Option<T>>` reads both as `None`.
fn present_even_if_null<'de, T, D>(de: D) -> std::result::Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    T::deserialize(de).map(Some)
}

/// One theme's material for a geometry: a `value` for the whole geometry, or
/// `values` with one index per primitive.
///
/// The schema requires exactly one of the two (checked by `validate`) and lets
/// `values` be `null`, so `values` is a double option: absent, `null`, or an
/// array. `V` is the shape of the array, fixed by the geometry type:
/// [`PrimitiveValues`], [`crate::SolidValues`] or [`crate::MultiSolidValues`]
/// of [`MaterialIndex`].
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(bound(deserialize = "V: Deserialize<'de>"))]
pub struct MaterialReference<V = PrimitiveValues<MaterialIndex>> {
    #[serde(
        default,
        deserialize_with = "present_even_if_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub values: Option<Option<V>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<MaterialIndex>,
    /// Members the schema does not name, kept verbatim.
    #[serde(flatten)]
    pub other: Map<String, Value>,
}

impl<V: Indices<MaterialIndex>> Indices<MaterialIndex> for MaterialReference<V> {
    fn for_each_index(&mut self, f: &mut dyn FnMut(&mut MaterialIndex)) {
        self.value.for_each_index(f);
        self.values.for_each_index(f);
    }
    fn visit_index(&self, f: &mut dyn FnMut(MaterialIndex)) {
        self.value.visit_index(f);
        self.values.visit_index(f);
    }
}

/// One theme's texture for a geometry: `values` nested as deeply as the
/// boundaries, one [`TexturedRing`] per ring. The schema requires no member,
/// so `values` may be absent.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TextureReference<V = Vec<TexturedSurface>> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<V>,
    /// Members the schema does not name, kept verbatim.
    #[serde(flatten)]
    pub other: Map<String, Value>,
}

impl<I, V: Indices<I>> Indices<I> for TextureReference<V> {
    fn for_each_index(&mut self, f: &mut dyn FnMut(&mut I)) {
        self.values.for_each_index(f);
    }
    fn visit_index(&self, f: &mut dyn FnMut(I)) {
        self.values.visit_index(f);
    }
}

/// An RGB colour, three numbers.
pub type Color = [f64; 3];

/// A texture's `borderColor`: three or four numbers, written back as given.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum BorderColor {
    /// `minItems: 3`
    Rgb(Color),
    /// `maxItems: 4`
    Rgba([f64; 4]),
}

/// A texture image's format, `"PNG"` or `"JPG"` (upper case, unlike
/// `wrapMode` and `textureType`).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureFormat {
    PNG,
    JPG,
}

/// How a texture repeats outside `[0, 1]`.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum WrapMode {
    None,
    Wrap,
    Mirror,
    Clamp,
    Border,
}

/// What a texture depicts.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum TextureType {
    Unknown,
    Specific,
    Typical,
}

/// One entry of `appearance.materials`, referred to by a [`MaterialIndex`].
/// `name` is required. The schema closes the object, so a member it does not
/// name is an error.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MaterialObject {
    /// The schema's one required member.
    pub name: String,
    #[serde(rename = "ambientIntensity", skip_serializing_if = "Option::is_none")]
    pub ambient_intensity: Option<f64>,
    #[serde(rename = "diffuseColor", skip_serializing_if = "Option::is_none")]
    pub diffuse_color: Option<Color>,
    #[serde(rename = "emissiveColor", skip_serializing_if = "Option::is_none")]
    pub emissive_color: Option<Color>,
    #[serde(rename = "specularColor", skip_serializing_if = "Option::is_none")]
    pub specular_color: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shininess: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transparency: Option<f64>,
    #[serde(rename = "isSmooth", skip_serializing_if = "Option::is_none")]
    pub is_smooth: Option<bool>,
}

/// One entry of `appearance.textures`, referred to by a [`TextureIndex`].
/// Every member is optional. The schema closes the object, so a member it does
/// not name is an error.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TextureObject {
    /// The image format (`type` in JSON).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub thetype: Option<TextureFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(rename = "wrapMode", skip_serializing_if = "Option::is_none")]
    pub wrap_mode: Option<WrapMode>,
    #[serde(rename = "textureType", skip_serializing_if = "Option::is_none")]
    pub texture_type: Option<TextureType>,
    #[serde(rename = "borderColor", skip_serializing_if = "Option::is_none")]
    pub border_color: Option<BorderColor>,
}

/// A float's bits, with `-0.0` folded onto `0.0` so that values equal under
/// `==` hash alike.
fn bits(x: f64) -> u64 {
    (x + 0.0).to_bits()
}

/// What two materials must share to be the same palette entry: every member,
/// floats compared by value. Hashable, unlike `MaterialObject` itself.
#[derive(PartialEq, Eq, Hash)]
pub(crate) struct MaterialKey {
    name: String,
    floats: [Option<u64>; 3],
    colours: [Option<[u64; 3]>; 3],
    is_smooth: Option<bool>,
}

impl MaterialObject {
    pub(crate) fn key(&self) -> MaterialKey {
        let colour = |c: &Option<Color>| c.map(|c| c.map(bits));
        MaterialKey {
            name: self.name.clone(),
            floats: [self.ambient_intensity, self.shininess, self.transparency]
                .map(|f| f.map(bits)),
            colours: [
                colour(&self.diffuse_color),
                colour(&self.emissive_color),
                colour(&self.specular_color),
            ],
            is_smooth: self.is_smooth,
        }
    }
}

/// As [`MaterialKey`], for textures.
#[derive(PartialEq, Eq, Hash)]
pub(crate) struct TextureKey {
    thetype: Option<TextureFormat>,
    image: Option<String>,
    wrap_mode: Option<WrapMode>,
    texture_type: Option<TextureType>,
    border_color: Option<Vec<u64>>,
}

impl TextureObject {
    pub(crate) fn key(&self) -> TextureKey {
        TextureKey {
            thetype: self.thetype,
            image: self.image.clone(),
            wrap_mode: self.wrap_mode,
            texture_type: self.texture_type,
            border_color: self.border_color.as_ref().map(|b| match b {
                BorderColor::Rgb(c) => c.iter().copied().map(bits).collect(),
                BorderColor::Rgba(c) => c.iter().copied().map(bits).collect(),
            }),
        }
    }
}

/// The appearance library of a document or feature: the palettes that
/// material, texture and texture-vertex indices point into. The schema closes
/// the object, so a member it does not name is an error.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Appearance {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub materials: Option<Vec<MaterialObject>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub textures: Option<Vec<TextureObject>>,
    /// The (u, v) texture vertices, referred to by a [`UvIndex`].
    #[serde(rename = "vertices-texture")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertices_texture: Option<Vec<[f64; 2]>>,
    #[serde(rename = "default-theme-texture")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_theme_texture: Option<String>,
    #[serde(rename = "default-theme-material")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_theme_material: Option<String>,
}
impl Appearance {
    pub(crate) fn new() -> Self {
        Appearance {
            materials: None,
            textures: None,
            vertices_texture: None,
            default_theme_texture: None,
            default_theme_material: None,
        }
    }
    pub(crate) fn add_vertices_texture(&mut self, mut vs: Vec<[f64; 2]>) {
        self.vertices_texture
            .get_or_insert_with(Vec::new)
            .append(&mut vs);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Theme objects are open, so members they do not name are kept.
    #[test]
    fn a_material_or_texture_theme_keeps_members_the_schema_does_not_name() {
        let m: MaterialReference =
            serde_json::from_value(serde_json::json!({"value": 0, "vendorData": true})).unwrap();
        assert_eq!(m.value, Some(MaterialIndex(0)));
        assert_eq!(
            serde_json::to_value(&m).unwrap(),
            serde_json::json!({"value": 0, "vendorData": true})
        );

        let t: TextureReference =
            serde_json::from_value(serde_json::json!({"values": [[[0, 1, 2]]], "note": "x"}))
                .unwrap();
        assert_eq!(
            serde_json::to_value(&t).unwrap(),
            serde_json::json!({"values": [[[0, 1, 2]]], "note": "x"})
        );
    }

    /// An absent member stays absent, and `{"values": null}` stays distinct from
    /// `{}`.
    #[test]
    fn the_catch_all_does_not_disturb_absent_versus_null() {
        let empty: MaterialReference = serde_json::from_value(serde_json::json!({})).unwrap();
        assert!(empty.other.is_empty());
        assert_eq!(serde_json::to_value(&empty).unwrap(), serde_json::json!({}));

        let null: MaterialReference =
            serde_json::from_value(serde_json::json!({"values": null})).unwrap();
        assert_eq!(null.values, Some(None));
        assert_eq!(
            serde_json::to_value(&null).unwrap(),
            serde_json::json!({"values": null})
        );
    }

    #[test]
    fn material_indices_serialize_as_numbers_and_null() {
        // cjseq2 0.1.0 emitted [[], [1]] here. That is invalid CityJSON.
        let m: MaterialReference = serde_json::from_value(serde_json::json!({
            "values": [null, 1]
        }))
        .unwrap();
        assert_eq!(
            serde_json::to_value(&m).unwrap(),
            serde_json::json!({"values": [null, 1]})
        );
    }

    #[test]
    fn solid_material_values_keep_their_shell_level() {
        let m: MaterialReference<crate::SolidValues<MaterialIndex>> =
            serde_json::from_value(serde_json::json!({
                "values": [[0, 1]]
            }))
            .unwrap();
        assert_eq!(
            serde_json::to_value(&m).unwrap(),
            serde_json::json!({"values": [[0, 1]]})
        );
    }

    #[test]
    fn texture_ring_is_index_then_uvs() {
        let t: TextureReference = serde_json::from_value(serde_json::json!({
            "values": [[[0, 10, 11, 12]]]
        }))
        .unwrap();
        assert_eq!(
            serde_json::to_value(&t).unwrap(),
            serde_json::json!({"values": [[[0, 10, 11, 12]]]})
        );
    }

    /// With `value` instead of `values`, the absent member is not written.
    #[test]
    fn material_value_is_a_bare_integer() {
        let m: MaterialReference = serde_json::from_value(serde_json::json!({"value": 3})).unwrap();
        assert_eq!(m.value, Some(MaterialIndex(3)));
        assert_eq!(
            serde_json::to_value(&m).unwrap(),
            serde_json::json!({"value": 3})
        );
    }

    /// A texture theme may have no `values`.
    #[test]
    fn a_texture_reference_may_carry_no_values() {
        let absent: TextureReference = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(absent.values, None);
        //-- and an absent `values` must not reappear as `null` on the way out
        assert_eq!(
            serde_json::to_value(&absent).unwrap(),
            serde_json::json!({})
        );

        //-- `{"values": null}` is not valid CityJSON (`values` is an array);
        //-- it is accepted and written back as the valid `{}`
        let null: TextureReference =
            serde_json::from_value(serde_json::json!({"values": null})).unwrap();
        assert_eq!(null.values, None);
        assert_eq!(serde_json::to_value(&null).unwrap(), serde_json::json!({}));
    }

    /// `values` is absent, `null` or an array, and each is written back as it
    /// was.
    #[test]
    fn material_values_distinguishes_absent_from_null_from_array() {
        //-- absent: only `value` carries the assignment
        let absent: MaterialReference =
            serde_json::from_value(serde_json::json!({"value": 3})).unwrap();
        assert_eq!(absent.values, None);
        assert_eq!(
            serde_json::to_value(&absent).unwrap(),
            serde_json::json!({"value": 3}),
            "an absent `values` must stay absent"
        );

        //-- explicitly null: "no materials anywhere on this geometry"
        let null: MaterialReference =
            serde_json::from_value(serde_json::json!({"values": null})).unwrap();
        assert_eq!(null.values, Some(None));
        assert_eq!(
            serde_json::to_value(&null).unwrap(),
            serde_json::json!({"values": null}),
            "an explicit `null` must be written back as `null`, not dropped"
        );

        //-- an array: the ordinary case
        let array: MaterialReference =
            serde_json::from_value(serde_json::json!({"values": [0, 1]})).unwrap();
        assert_eq!(
            array.values,
            Some(Some(vec![Some(MaterialIndex(0)), Some(MaterialIndex(1))]))
        );
        assert_eq!(
            serde_json::to_value(&array).unwrap(),
            serde_json::json!({"values": [0, 1]})
        );

        //-- and a bare `{}` stays `{}` rather than gaining a null
        let empty: MaterialReference = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(serde_json::to_value(&empty).unwrap(), serde_json::json!({}));
    }

    //-------------------------------------------------------------------
    //-- the document-level appearance *library*: `materials`/`textures`
    //-------------------------------------------------------------------

    /// The five `wrapMode` spellings round-trip; any other spelling, including a
    /// differently-cased one, is an error.
    #[test]
    fn every_wrap_mode_round_trips_and_nothing_else_is_accepted() {
        for mode in ["none", "wrap", "mirror", "clamp", "border"] {
            let input = serde_json::json!({"textures": [{"wrapMode": mode}]});
            let a: Appearance = serde_json::from_value(input.clone())
                .unwrap_or_else(|e| panic!("{mode} is a schema-valid wrapMode: {e}"));
            assert_eq!(
                serde_json::to_value(&a).unwrap(),
                input,
                "wrapMode {mode:?} must come back exactly as it went in"
            );
        }

        for bad in ["Wrap", "WRAP", "None", "repeat", ""] {
            assert!(
                serde_json::from_value::<Appearance>(
                    serde_json::json!({"textures": [{"wrapMode": bad}]})
                )
                .is_err(),
                "{bad:?} is not one of the schema's five wrapMode spellings"
            );
        }
    }

    /// `textureType` is lower case and `type` upper case; anything else is an
    /// error.
    #[test]
    fn every_texture_type_and_format_round_trips_and_nothing_else_is_accepted() {
        for t in ["unknown", "specific", "typical"] {
            let input = serde_json::json!({"textures": [{"textureType": t}]});
            let a: Appearance = serde_json::from_value(input.clone())
                .unwrap_or_else(|e| panic!("{t} is a schema-valid textureType: {e}"));
            assert_eq!(serde_json::to_value(&a).unwrap(), input);
        }
        for bad in ["Unknown", "SPECIFIC", "other"] {
            assert!(
                serde_json::from_value::<Appearance>(
                    serde_json::json!({"textures": [{"textureType": bad}]})
                )
                .is_err(),
                "{bad:?} is not one of the schema's three textureType spellings"
            );
        }

        for f in ["PNG", "JPG"] {
            let input = serde_json::json!({"textures": [{"type": f}]});
            let a: Appearance = serde_json::from_value(input.clone())
                .unwrap_or_else(|e| panic!("{f} is a schema-valid texture type: {e}"));
            assert_eq!(serde_json::to_value(&a).unwrap(), input);
        }
        //-- the schema spells these upper case, and lists only these two;
        //-- `JPEG` and `jpg` are both outside it
        for bad in ["png", "jpg", "JPEG", "TIFF"] {
            assert!(
                serde_json::from_value::<Appearance>(
                    serde_json::json!({"textures": [{"type": bad}]})
                )
                .is_err(),
                "{bad:?} is not one of the schema's two texture formats"
            );
        }
    }

    /// Every texture member is optional: none is invented, none is lost.
    #[test]
    fn a_texture_object_keeps_every_member_it_was_given_and_invents_none() {
        let full = serde_json::json!({"textures": [{
            "type": "JPG",
            "image": "appearances/wood.jpg",
            "wrapMode": "wrap",
            "textureType": "unknown",
            "borderColor": [0.0, 0.0, 0.0, 1.0]
        }]});
        let a: Appearance = serde_json::from_value(full.clone()).unwrap();
        assert_eq!(serde_json::to_value(&a).unwrap(), full);

        let bare = serde_json::json!({"textures": [{}]});
        let a: Appearance = serde_json::from_value(bare.clone()).unwrap();
        assert_eq!(
            serde_json::to_value(&a).unwrap(),
            bare,
            "the Texture schema has no `required`, so an empty texture object \
             is valid and must not gain members"
        );
    }

    #[test]
    fn border_color_is_three_or_four_numbers() {
        for ok in [
            serde_json::json!([0.0, 0.0, 0.0]),
            serde_json::json!([0.0, 0.0, 0.0, 1.0]),
        ] {
            let input = serde_json::json!({"textures": [{"borderColor": ok}]});
            let a: Appearance = serde_json::from_value(input.clone())
                .unwrap_or_else(|e| panic!("{ok} is a schema-valid borderColor: {e}"));
            assert_eq!(serde_json::to_value(&a).unwrap(), input);
        }
        for bad in [
            serde_json::json!([]),
            serde_json::json!([0.0]),
            serde_json::json!([0.0, 0.0]),
            serde_json::json!([0.0, 0.0, 0.0, 1.0, 1.0]),
        ] {
            assert!(
                serde_json::from_value::<Appearance>(
                    serde_json::json!({"textures": [{"borderColor": bad}]})
                )
                .is_err(),
                "{bad} violates borderColor's minItems 3 / maxItems 4"
            );
        }
    }

    #[test]
    fn a_material_object_requires_a_name_and_three_number_colours() {
        let full = serde_json::json!({"materials": [{
            "name": "roof",
            "ambientIntensity": 0.4,
            "diffuseColor": [0.5, 0.4, 0.3],
            "emissiveColor": [0.0, 0.0, 0.0],
            "specularColor": [1.0, 1.0, 1.0],
            "shininess": 0.2,
            "transparency": 0.0,
            "isSmooth": false
        }]});
        let a: Appearance = serde_json::from_value(full.clone()).unwrap();
        assert_eq!(serde_json::to_value(&a).unwrap(), full);

        //-- `name` is the schema's one required member
        assert!(
            serde_json::from_value::<Appearance>(
                serde_json::json!({"materials": [{"diffuseColor": [0.5, 0.4, 0.3]}]})
            )
            .is_err(),
            "`name` is required by the Material schema"
        );

        //-- and a colour is exactly three numbers, never two, never four
        for bad in [
            serde_json::json!([0.5, 0.4]),
            serde_json::json!([0.5, 0.4, 0.3, 0.2]),
        ] {
            assert!(
                serde_json::from_value::<Appearance>(
                    serde_json::json!({"materials": [{"name": "m", "diffuseColor": bad}]})
                )
                .is_err(),
                "{bad} violates diffuseColor's minItems 3 / maxItems 3"
            );
        }

        //-- only `name` need be there
        let bare = serde_json::json!({"materials": [{"name": "m"}]});
        let a: Appearance = serde_json::from_value(bare.clone()).unwrap();
        assert_eq!(serde_json::to_value(&a).unwrap(), bare);
    }

    /// Materials and textures are closed objects: an unnamed member, or a
    /// misspelt one, is an error.
    #[test]
    fn material_and_texture_objects_admit_no_members_the_schema_does_not_name() {
        assert!(
            serde_json::from_value::<Appearance>(
                serde_json::json!({"materials": [{"name": "m", "vendorData": true}]})
            )
            .is_err(),
            "the Material schema declares additionalProperties: false"
        );
        assert!(
            serde_json::from_value::<Appearance>(
                serde_json::json!({"textures": [{"image": "a.jpg", "vendorData": true}]})
            )
            .is_err(),
            "the Texture schema declares additionalProperties: false"
        );
        //-- a near-miss spelling is an unnamed member too
        assert!(
            serde_json::from_value::<Appearance>(
                serde_json::json!({"textures": [{"wrapmode": "wrap"}]})
            )
            .is_err(),
            "`wrapmode` is not `wrapMode`"
        );
    }

    /// The appearance object is closed too.
    #[test]
    fn the_appearance_object_admits_no_members_the_schema_does_not_name() {
        for bad in [
            serde_json::json!({"vendorData": true}),
            //-- a near-miss on each of the two hyphenated names, which are the
            //-- easiest of the five to mistype
            serde_json::json!({"default_theme_texture": "winter"}),
            serde_json::json!({"vertices_texture": [[0.0, 0.0]]}),
            //-- and a near-miss alongside a member that is correct, so the
            //-- rejection cannot be an accident of the object being otherwise
            //-- empty
            serde_json::json!({"materials": [], "texture": []}),
        ] {
            assert!(
                serde_json::from_value::<Appearance>(bad.clone()).is_err(),
                "{bad} carries a member `appearance` does not name, and the \
                 schema declares additionalProperties: false"
            );
        }

        //-- the control: all five named members together are accepted, so the
        //-- loop above is not passing merely because everything is rejected
        let all_five = serde_json::json!({
            "materials": [{"name": "m"}],
            "textures": [{"image": "a.jpg"}],
            "vertices-texture": [[0.0, 0.0]],
            "default-theme-texture": "winter",
            "default-theme-material": "irradiation"
        });
        let a: Appearance = serde_json::from_value(all_five.clone()).unwrap();
        assert_eq!(serde_json::to_value(&a).unwrap(), all_five);
    }

    #[test]
    fn a_texture_vertex_is_exactly_two_numbers() {
        for bad in [
            serde_json::json!({"vertices-texture": [[0.5]]}),
            serde_json::json!({"vertices-texture": [[0.5, 0.5, 0.5]]}),
            serde_json::json!({"vertices-texture": [[]]}),
            //-- one good vertex is not enough to rescue a bad one
            serde_json::json!({"vertices-texture": [[0.0, 0.0], [1.0, 1.0, 1.0]]}),
        ] {
            assert!(
                serde_json::from_value::<Appearance>(bad.clone()).is_err(),
                "{bad} has a texture vertex that is not exactly two numbers"
            );
        }

        //-- the control, round-tripping through the fixed-size array
        let good = serde_json::json!({"vertices-texture": [[0.0, 0.5], [1.0, 1.0]]});
        let a: Appearance = serde_json::from_value(good.clone()).unwrap();
        assert_eq!(a.vertices_texture, Some(vec![[0.0, 0.5], [1.0, 1.0]]));
        assert_eq!(serde_json::to_value(&a).unwrap(), good);
    }

    /// Empty palettes stay empty rather than becoming absent.
    #[test]
    fn an_empty_appearance_library_stays_empty() {
        let input = serde_json::json!({"materials": [], "textures": []});
        let a: Appearance = serde_json::from_value(input.clone()).unwrap();
        assert_eq!(serde_json::to_value(&a).unwrap(), input);
    }
}
