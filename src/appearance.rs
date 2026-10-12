use crate::index::{MaterialIndex, TextureIndex, UvIndex};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// One shell's worth of material indices, one per surface. `None` — a whole
/// shell with no material — is `null` in JSON, which
/// `geomprimitives.schema.json` permits at every level of `material.values`
/// (`"type": ["array", "null"]`).
pub type MaterialShell = Vec<Option<MaterialIndex>>;
/// One solid's worth of material indices, one per surface, per shell.
pub type MaterialSolid = Vec<Option<MaterialShell>>;

/// The `values` array of a [`MaterialReference`]: one index into the
/// document's `materials` palette per *surface*, so it is nested exactly two
/// levels less deeply than that geometry's `boundaries` (CityJSON 2.0,
/// section 6.1).
///
/// `null` is permitted at *every* level, not only at the leaf: a surface, a
/// whole shell, or a whole solid may have no material. Every one of those
/// `None`s must serialize back as `null` and never as `[]` — that is
/// finding #7, and it is pinned by test at each level.
///
/// The variants are ordered shallowest-first; serde tries them in declaration
/// order, so a value only reaches a deeper variant when the shallower ones
/// cannot describe it.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum MaterialValues {
    /// `MultiSurface`, `CompositeSurface`: one index per surface.
    Surfaces(Vec<Option<MaterialIndex>>),
    /// `Solid`: one index per surface, per shell.
    Shells(Vec<Option<MaterialShell>>),
    /// `MultiSolid`, `CompositeSolid`: one index per surface, per shell, per
    /// solid.
    Solids(Vec<Option<MaterialSolid>>),
}

impl MaterialValues {
    /// Every material index, in document order, whatever the depth. The depth
    /// is known from the variant, so no runtime inspection is needed; the
    /// `flatten()`s skip `None` sub-arrays as well as `null` leaves.
    pub(crate) fn indices_mut(&mut self) -> Box<dyn Iterator<Item = &mut MaterialIndex> + '_> {
        match self {
            MaterialValues::Surfaces(v) => Box::new(v.iter_mut().flatten()),
            MaterialValues::Shells(v) => Box::new(v.iter_mut().flatten().flatten().flatten()),
            MaterialValues::Solids(v) => Box::new(
                v.iter_mut()
                    .flatten()
                    .flatten()
                    .flatten()
                    .flatten()
                    .flatten(),
            ),
        }
    }
}

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

/// The `values` array of a [`TextureReference`]: nested exactly as deeply as
/// the geometry's `boundaries` (CityJSON 2.0, section 6.2), with each ring
/// becoming `[texture_index, uv_index, ...]` — one more entry than the ring
/// has vertices. An untextured ring is `[null]`.
///
/// Only surface-bearing geometries can carry a texture, so the ladder starts
/// at `MultiSurface` depth: a `MultiPoint` or `MultiLineString` has no rings
/// to texture, and both declare `additionalProperties: false` without a
/// `texture` member.
///
/// Unlike [`MaterialValues`], the intermediate levels are *not* nullable.
/// `geomprimitives.schema.json` types `texture.values` and every one of its
/// intermediate `items` as a plain `"array"` — only the innermost items are
/// `["integer", "null"]` — so `[null, [[0, 10, 11]]]` is not valid CityJSON
/// and is rejected here.
///
/// The variants are ordered shallowest-first, as in [`MaterialValues`].
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum TextureValues {
    /// `MultiSurface`, `CompositeSurface`: per surface, per ring.
    Surface(Vec<TexturedSurface>),
    /// `Solid`: ... per shell.
    Shell(Vec<TexturedShell>),
    /// `MultiSolid`, `CompositeSolid`: ... per solid.
    Solid(Vec<Vec<TexturedShell>>),
}

impl TextureValues {
    /// Every ring, in document order, whatever the depth. A ring is
    /// `[texture_index, uv_index, ...]`, so the caller renumbers its first
    /// entry against a different map than the rest.
    pub(crate) fn rings_mut(&mut self) -> Box<dyn Iterator<Item = &mut TexturedRing> + '_> {
        match self {
            TextureValues::Surface(v) => Box::new(v.iter_mut().flatten()),
            TextureValues::Shell(v) => Box::new(v.iter_mut().flatten().flatten()),
            TextureValues::Solid(v) => Box::new(v.iter_mut().flatten().flatten().flatten()),
        }
    }
}

/// Keep an explicit `null` distinct from an absent member.
///
/// `serde_json` maps a JSON `null` onto the *outer* `None` of a plain
/// `Option<Option<T>>`, which collapses `{"values": null}` and `{}` onto the
/// same value and re-emits the first as the second. Deserializing the inner
/// `Option<T>` and wrapping that in `Some` keeps the two apart.
///
/// This delegates entirely to the derived `Deserialize` of `T` — it writes no
/// visitor — and it does not touch serialization, which stays fully derived.
fn present_even_if_null<'de, T, D>(de: D) -> std::result::Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    T::deserialize(de).map(Some)
}

/// One theme's material assignment for a geometry: either a `value` colouring
/// the whole object, or a depth-typed `values` array with one index per
/// surface.
///
/// The schema requires exactly one of the two
/// (`oneOf: [{required: ["value"]}, {required: ["values"]}]`) but separately
/// permits `"values": null`. So `values` is a *double* option: the outer level
/// is present-vs-absent, the inner is `null`-vs-array. Without that
/// distinction a schema-valid `{"values": null}` re-emits as `{}`, which
/// satisfies neither branch of the `oneOf` and is rejected by a validator.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MaterialReference {
    #[serde(
        default,
        deserialize_with = "present_even_if_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub values: Option<Option<MaterialValues>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<MaterialIndex>,
    /// The per-theme material object names `value` and `values` and declares
    /// no `additionalProperties: false`, so anything further is legal
    /// CityJSON and is kept rather than dropped.
    #[serde(flatten)]
    pub other: Map<String, Value>,
}

/// One theme's texture assignment for a geometry.
///
/// `values` is optional: the per-theme texture object in
/// `geomprimitives.schema.json` carries no `required` keyword at all (in
/// contrast to `material`, which requires exactly one of `value`/`values`), so
/// a bare `{"theme": {}}` is valid CityJSON.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TextureReference {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<TextureValues>,
    /// As with [`MaterialReference`]: the per-theme texture object declares no
    /// `additionalProperties: false`, so extra members are legal and kept.
    #[serde(flatten)]
    pub other: Map<String, Value>,
}

/// A material or border colour: three numbers, per
/// `appearance.schema.json`'s `"minItems": 3, "maxItems": 3` on
/// `diffuseColor`, `emissiveColor` and `specularColor`.
///
/// A fixed-size array, not a `Vec`, so the cardinality is checked by serde's
/// derived `Deserialize` rather than by a downstream `if len == 3`. The same
/// technique already types `GeometryInstance::transformation_matrix` as
/// `[f64; 16]`.
pub type Color = [f64; 3];

/// A texture's `borderColor`, whose schema is the one place a CityJSON colour
/// is *not* fixed at three numbers:
///
/// ```text
/// "borderColor": {
///   "type": "array", "items": {"type": "number"},
///   "minItems": 3, "maxItems": 4
/// }
/// ```
///
/// Three or four, and nothing else. A `Vec<f64>` would accept two or five and
/// leave the check to a consumer that may not make it; the untagged pair of
/// fixed-size arrays makes serde do it. Serialization is derived, and an
/// untagged enum of arrays serializes as the array itself, so `[0, 0, 0, 1]`
/// comes back spelled exactly as it went in.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum BorderColor {
    /// `minItems: 3`
    Rgb(Color),
    /// `maxItems: 4`
    Rgba([f64; 4]),
}

/// A texture image's file format: `"type": {"enum": ["PNG", "JPG"]}`.
///
/// Note the case: this member is spelled upper case where its `wrapMode` and
/// `textureType` siblings are lower. Getting that wrong in a hand-written
/// `match` is precisely the defect this type removes.
///
/// A closed enumeration in the schema -- unlike `CityObjectType` and
/// `SemanticSurfaceType`, which carry an `Extension(String)` case because
/// CityJSON Extensions may add to those two sets. Nothing in the spec lets an
/// Extension add a texture format, so there is no such case here and an
/// unrecognised spelling is an error.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureFormat {
    PNG,
    JPG,
}

/// How a texture repeats outside `[0, 1]`:
/// `"wrapMode": {"enum": ["none", "wrap", "mirror", "clamp", "border"]}`.
///
/// All five are lower case. A texture written `"wrapMode": "wrap"` once came
/// back as `"None"` from FlatCityBuf because a downstream `match` expected a
/// different spelling and its `_` arm turned the miss into a default; with the
/// spellings here, the same mistake is a `serde` error at the boundary and a
/// non-exhaustive `match` at every consumer.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum WrapMode {
    None,
    Wrap,
    Mirror,
    Clamp,
    Border,
}

/// What a texture depicts:
/// `"textureType": {"enum": ["unknown", "specific", "typical"]}`. Lower case,
/// and a closed set.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum TextureType {
    Unknown,
    Specific,
    Typical,
}

/// One entry of the document-level `appearance.materials` palette, referred to
/// by index from a [`MaterialReference`].
///
/// `appearance.schema.json#/Material`, in full:
///
/// ```text
/// "Material": {
///   "type": "object",
///   "properties": {
///     "name": {"type": "string"},
///     "ambientIntensity": {"type": "number"},
///     "diffuseColor":  {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3},
///     "emissiveColor": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3},
///     "specularColor": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3},
///     "shininess": {"type": "number"},
///     "transparency": {"type": "number"},
///     "isSmooth": {"type": "boolean"}
///   },
///   "required": ["name"],
///   "additionalProperties": false
/// }
/// ```
///
/// So `name` is a plain `String` and everything else an `Option`, and there is
/// no `#[serde(flatten)] other` catch-all: `additionalProperties: false` means
/// an unnamed member is not CityJSON at all. `deny_unknown_fields` says so out
/// loud rather than dropping it, which matters because dropping is silent data
/// loss and, for the all-optional [`TextureObject`] below, an object of
/// nothing but mis-spelled members would otherwise parse as an empty one.
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

/// One entry of the document-level `appearance.textures` palette, referred to
/// by the first entry of each ring in a [`TextureValues`].
///
/// `appearance.schema.json#/Texture`, in full:
///
/// ```text
/// "Texture": {
///   "type": "object",
///   "properties": {
///     "type": {"enum": ["PNG", "JPG"]},
///     "image": {"type": "string"},
///     "wrapMode": {"enum": ["none", "wrap", "mirror", "clamp", "border"]},
///     "textureType": {"enum": ["unknown", "specific", "typical"]},
///     "borderColor": {"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 4}
///   },
///   "additionalProperties": false
/// }
/// ```
///
/// There is no `required` keyword, so *every* member is optional and a bare
/// `{}` is valid CityJSON — including `image`, which FlatCityBuf's `.fbs`
/// currently makes mandatory.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TextureObject {
    /// The image file format. Named `thetype` because `type` is a Rust
    /// keyword, as in [`crate::CityJSONFeature`].
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

/// The document-level appearance library.
///
/// `cityjson.schema.json#/properties/appearance` names five members and then
/// declares `"additionalProperties": false`, exactly as the [`MaterialObject`]
/// and [`TextureObject`] it contains do —
/// `cityjsonfeature.schema.json#/properties/appearance` carries the same
/// clause verbatim, so the rule holds at both document levels. Hence
/// `deny_unknown_fields` here too: an extra member is not CityJSON, and
/// dropping it silently at this level while rejecting it one level down would
/// be inconsistent as well as lossy.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Appearance {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub materials: Option<Vec<MaterialObject>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub textures: Option<Vec<TextureObject>>,
    /// The (u, v) texture vertices. The schema pins each one to
    /// `minItems: 2, maxItems: 2`, so it is `[f64; 2]` and not `Vec<f64>`:
    /// a fixed-size array rejects a 1- or 3-element vertex on read and makes
    /// one unwritable, exactly as `[f64; 3]` does for a colour and `[f64; 6]`
    /// for a geographical extent.
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
    pub(crate) fn add_material(&mut self, jm: MaterialObject) -> usize {
        let ls = self.materials.get_or_insert_with(Vec::new);
        match ls.iter().position(|e| *e == jm) {
            Some(y) => y,
            None => {
                ls.push(jm);
                ls.len() - 1
            }
        }
    }
    pub(crate) fn add_texture(&mut self, jm: TextureObject) -> usize {
        let ls = self.textures.get_or_insert_with(Vec::new);
        match ls.iter().position(|e| *e == jm) {
            Some(y) => y,
            None => {
                ls.push(jm);
                ls.len() - 1
            }
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

    /// The per-theme `material` and `texture` objects in
    /// `geomprimitives.schema.json` name their members and then stop: neither
    /// declares `additionalProperties: false`, unlike the geometry object
    /// enclosing them, which does. So an extra member is legal CityJSON, and
    /// dropping it is silent data loss.
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

    /// The catch-all must not disturb what was already pinned: an absent
    /// member stays absent, and `{"values": null}` stays distinct from `{}`.
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
        // Finding #8: this shape came back as [[[0, 1]]] through FCB.
        let m: MaterialReference = serde_json::from_value(serde_json::json!({
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

    /// A `MaterialReference` may carry `value` instead of `values`; the absent
    /// member must not be serialized at all (never as `null`, never as `[]`).
    #[test]
    fn material_value_is_a_bare_integer() {
        let m: MaterialReference = serde_json::from_value(serde_json::json!({"value": 3})).unwrap();
        assert_eq!(m.value, Some(MaterialIndex(3)));
        assert_eq!(
            serde_json::to_value(&m).unwrap(),
            serde_json::json!({"value": 3})
        );
    }

    /// The untagged variant order is load-bearing: asserting only on the
    /// re-serialized JSON would pass even if a value landed in the wrong
    /// variant, so pin the variant identity itself.
    ///
    /// Material values are nested two levels less deeply than the boundaries
    /// of the geometry that carries them (CityJSON 2.0, section 6.1).
    #[test]
    fn material_values_land_in_the_variant_their_depth_implies() {
        let parse = |v: serde_json::Value| -> MaterialValues { serde_json::from_value(v).unwrap() };

        //-- MultiSurface/CompositeSurface: one index per surface
        assert!(matches!(
            parse(serde_json::json!([0, null, 2])),
            MaterialValues::Surfaces(_)
        ));
        //-- Solid: one index per surface, per shell
        assert!(matches!(
            parse(serde_json::json!([[0, null, 2]])),
            MaterialValues::Shells(_)
        ));
        //-- MultiSolid/CompositeSolid: ... per solid
        assert!(matches!(
            parse(serde_json::json!([[[0, null, 2]]])),
            MaterialValues::Solids(_)
        ));

        //-- the empty array is ambiguous; it must resolve to the shallowest
        //-- variant, and stay `[]` on the way out
        let empty = parse(serde_json::json!([]));
        assert!(matches!(empty, MaterialValues::Surfaces(_)));
        assert_eq!(serde_json::to_value(&empty).unwrap(), serde_json::json!([]));

        //-- deeper than the ladder is not silently accepted
        assert!(serde_json::from_value::<MaterialValues>(serde_json::json!([[[[0]]]])).is_err());
        //-- and neither is a bare scalar
        assert!(serde_json::from_value::<MaterialValues>(serde_json::json!(0)).is_err());
        //-- the untyped Option<Value> this replaced accepted six levels
        assert!(
            serde_json::from_value::<MaterialValues>(serde_json::json!([[[[[[0]]]]]])).is_err()
        );
    }

    /// `geomprimitives.schema.json` types `material.values` and every one of
    /// its intermediate `items` as `["array", "null"]`, so a whole null shell
    /// or a whole null solid is valid CityJSON. Each of those `None`s must
    /// come back as `null` — never as `[]`, which is finding #7, the bug this
    /// whole rewrite exists to prevent.
    #[test]
    fn a_null_material_sub_array_round_trips_as_null_at_every_level() {
        for input in [
            //-- leaf: a surface with no material
            serde_json::json!([0, null, 2]),
            //-- intermediate: a whole shell with no material
            serde_json::json!([[0, 1], null]),
            serde_json::json!([null, [0, 1]]),
            //-- intermediate: a whole solid, and a shell inside a solid
            serde_json::json!([[[0, 1]], null]),
            serde_json::json!([[[0, 1], null], null]),
            //-- nothing but nulls, at each depth
            serde_json::json!([null, null]),
            serde_json::json!([[null], null]),
        ] {
            let v: MaterialValues = serde_json::from_value(input.clone())
                .unwrap_or_else(|e| panic!("{input} is schema-valid CityJSON: {e}"));
            assert_eq!(
                serde_json::to_value(&v).unwrap(),
                input,
                "{input} must round-trip with its nulls intact, never as []"
            );
        }
    }

    /// The variant ladder must still resolve by depth now that the
    /// intermediate levels are `Option`.
    #[test]
    fn null_sub_arrays_do_not_disturb_the_material_ladder() {
        let parse = |v: serde_json::Value| -> MaterialValues { serde_json::from_value(v).unwrap() };

        assert!(matches!(
            parse(serde_json::json!([null, 1])),
            MaterialValues::Surfaces(_)
        ));
        assert!(matches!(
            parse(serde_json::json!([[0, 1], null])),
            MaterialValues::Shells(_)
        ));
        assert!(matches!(
            parse(serde_json::json!([[[0, 1]], null])),
            MaterialValues::Solids(_)
        ));

        //-- the empty-shape edges: each is ambiguous and must settle on the
        //-- shallowest variant that can hold it, and stay byte-identical
        for (input, is_surfaces) in [
            (serde_json::json!([]), true),
            (serde_json::json!([[]]), false),
            (serde_json::json!([[[]]]), false),
        ] {
            let v = parse(input.clone());
            assert_eq!(serde_json::to_value(&v).unwrap(), input, "{input}");
            assert_eq!(
                matches!(v, MaterialValues::Surfaces(_)),
                is_surfaces,
                "{input} landed in {v:?}"
            );
        }
        assert!(matches!(
            parse(serde_json::json!([[]])),
            MaterialValues::Shells(_)
        ));
        assert!(matches!(
            parse(serde_json::json!([[[]]])),
            MaterialValues::Solids(_)
        ));
    }

    /// The texture ladder is deliberately *not* null-tolerant at its
    /// intermediate levels: `geomprimitives.schema.json` types
    /// `texture.values` and each intermediate `items` as a plain `"array"`,
    /// with only the innermost items `["integer", "null"]`. An untextured ring
    /// is spelled `[null]`, not `null`.
    #[test]
    fn texture_nulls_are_leaf_only() {
        //-- what the schema allows: a null texture index inside a ring
        let t: TextureValues = serde_json::from_value(serde_json::json!([[[null]]])).unwrap();
        assert_eq!(
            serde_json::to_value(&t).unwrap(),
            serde_json::json!([[[null]]])
        );

        //-- what it does not: a null in place of a surface, ring, or shell
        for bad in [
            serde_json::json!([null, [[0, 10, 11]]]),
            serde_json::json!([[null]]),
            serde_json::json!([[[[0, 1]]], null]),
        ] {
            assert!(
                serde_json::from_value::<TextureValues>(bad.clone()).is_err(),
                "{bad} is not valid CityJSON and must be rejected"
            );
        }
    }

    /// The per-theme texture object has no `required` keyword in the schema,
    /// so both of these are valid CityJSON and must parse.
    #[test]
    fn a_texture_reference_may_carry_no_values() {
        let absent: TextureReference = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(absent.values, None);
        //-- and an absent `values` must not reappear as `null` on the way out
        assert_eq!(
            serde_json::to_value(&absent).unwrap(),
            serde_json::json!({})
        );

        //-- Documented leniency, NOT a schema claim: `texture.values` is typed
        //-- `"type": "array"`, so `{"values": null}` is *invalid* CityJSON. A
        //-- plain `Option` field accepts it anyway, and normalizes it to the
        //-- valid `{}` on the way out. Pinned so the normalization is
        //-- deliberate rather than incidental.
        let null: TextureReference =
            serde_json::from_value(serde_json::json!({"values": null})).unwrap();
        assert_eq!(null.values, None);
        assert_eq!(serde_json::to_value(&null).unwrap(), serde_json::json!({}));
    }

    /// A `material` theme has three distinct states, and the schema
    /// distinguishes all three: absent (invalid on its own, but the `value`
    /// sibling may be carrying the assignment), explicitly `null`, and an
    /// array. Collapsing `null` onto absent re-emits `{"values": null}` as
    /// `{}`, which satisfies neither branch of the schema's
    /// `oneOf: [{required: ["value"]}, {required: ["values"]}]`.
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
            Some(Some(MaterialValues::Surfaces(vec![
                Some(MaterialIndex(0)),
                Some(MaterialIndex(1))
            ])))
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

    /// The bug that motivated typing this library: a texture written
    /// `"wrapMode": "wrap"` came back as `"None"`, because the consumer's
    /// `match` expected a different spelling and its `_` arm coerced the
    /// mis-spelling to a valid-looking default.
    ///
    /// `appearance.schema.json` fixes the five spellings exactly, all lower
    /// case:
    /// ```text
    /// "wrapMode": { "enum": ["none", "wrap", "mirror", "clamp", "border"] }
    /// ```
    /// so every one of them must survive a round trip byte-identical, and
    /// anything else -- including a differently-cased spelling of a real one
    /// -- must be rejected here rather than defaulted downstream.
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

    /// `"textureType": { "enum": ["unknown", "specific", "typical"] }` and
    /// `"type": { "enum": ["PNG", "JPG"] }` -- note the case difference
    /// between the two members, which is exactly the kind of detail a string
    /// `match` gets wrong.
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

    /// A texture object carries all five members, or none of them: the
    /// `Texture` schema has no `required` keyword at all, so `{}` is valid,
    /// and an absent member must not reappear.
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

    /// `"borderColor": {"type": "array", "items": {"type": "number"},
    /// "minItems": 3, "maxItems": 4}` -- three *or* four, never two, never
    /// five. This cardinality has already bitten this project once.
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

    /// The `Material` schema is the mirror image of `Texture`: it *requires*
    /// `name`, and pins each of its three colours at exactly three numbers
    /// (`"minItems": 3, "maxItems": 3`).
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

    /// Both `Material` and `Texture` declare `"additionalProperties": false`,
    /// unlike the per-theme reference objects above. So an unnamed member is
    /// *not* legal CityJSON, and quietly dropping it -- which is what a
    /// catch-all-less struct does by default -- would let an invalid file
    /// through and lose data on the way. Reject it instead.
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
        //-- a near-miss spelling is an unnamed member, and so is caught by the
        //-- same rule: this is the whole point of the exercise
        assert!(
            serde_json::from_value::<Appearance>(
                serde_json::json!({"textures": [{"wrapmode": "wrap"}]})
            )
            .is_err(),
            "`wrapmode` is not `wrapMode`"
        );
    }

    /// The `appearance` object itself declares `additionalProperties: false`,
    /// exactly as the `Material` and `Texture` objects inside it do.
    /// `cityjson.schema.json#/properties/appearance`:
    ///
    /// ```text
    /// "appearance": {
    ///   "type": "object",
    ///   "properties": {
    ///     "default-theme-texture": {"type": "string"},
    ///     "default-theme-material": {"type": "string"},
    ///     "materials": {"type": "array", "items": {"$ref": "appearance.schema.json#/Material"}},
    ///     "textures":  {"type": "array", "items": {"$ref": "appearance.schema.json#/Texture"}},
    ///     "vertices-texture": {"type": "array", "items": {"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2}}
    ///   },
    ///   "additionalProperties": false
    /// }
    /// ```
    ///
    /// `cityjsonfeature.schema.json#/properties/appearance` carries the same
    /// clause verbatim, so the rule holds at both document levels. Dropping an
    /// extra member here while rejecting one a level down, inside `Material`,
    /// would be asymmetric: the schema forbids the member either way.
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

    /// `cityjson.schema.json` pins each texture vertex to
    /// `{"type": "array", "items": {"type": "number"}, "minItems": 2,
    /// "maxItems": 2}`, so `vertices-texture` is `Vec<[f64; 2]>`: a vertex
    /// with any other length is not CityJSON and must not parse, nor be
    /// constructible in Rust. This is the same principle `Color` (`[f64; 3]`),
    /// `GeographicalExtent` (`[f64; 6]`) and `transformationMatrix`
    /// (`[f64; 16]`) already apply.
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

    /// An `appearance` with empty arrays in it is valid, and the arrays must
    /// stay empty rather than becoming absent (`tests/data/empty_appearance`).
    #[test]
    fn an_empty_appearance_library_stays_empty() {
        let input = serde_json::json!({"materials": [], "textures": []});
        let a: Appearance = serde_json::from_value(input.clone()).unwrap();
        assert_eq!(serde_json::to_value(&a).unwrap(), input);
    }

    /// Texture values are nested exactly as deeply as the boundaries of the
    /// geometry that carries them (CityJSON 2.0, section 6.2), each ring
    /// becoming `[texture_index, uv_index, ...]`.
    #[test]
    fn texture_values_land_in_the_variant_their_depth_implies() {
        let parse = |v: serde_json::Value| -> TextureValues { serde_json::from_value(v).unwrap() };

        //-- MultiSurface/CompositeSurface: per surface, per ring
        assert!(matches!(
            parse(serde_json::json!([[[0, 10, 11, 12]]])),
            TextureValues::Surface(_)
        ));
        //-- Solid: ... per shell
        assert!(matches!(
            parse(serde_json::json!([[[[0, 10, 11, 12]]]])),
            TextureValues::Shell(_)
        ));
        //-- MultiSolid/CompositeSolid: ... per solid
        assert!(matches!(
            parse(serde_json::json!([[[[[0, 10, 11, 12]]]]])),
            TextureValues::Solid(_)
        ));
        //-- an untextured ring is `[null]`, at every depth
        assert!(matches!(
            parse(serde_json::json!([[[null]]])),
            TextureValues::Surface(_)
        ));

        let empty = parse(serde_json::json!([]));
        assert!(matches!(empty, TextureValues::Surface(_)));
        assert_eq!(serde_json::to_value(&empty).unwrap(), serde_json::json!([]));

        //-- too deep, and too shallow, are both rejected: the untyped
        //-- `Option<Value>` this replaces accepted either without a murmur
        assert!(serde_json::from_value::<TextureValues>(serde_json::json!([[[[[[0]]]]]])).is_err());
        assert!(serde_json::from_value::<TextureValues>(serde_json::json!([0, 1])).is_err());
    }
}
