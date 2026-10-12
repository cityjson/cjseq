use crate::geometry::Geometry;
use crate::metadata::GeographicalExtent;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The known 1st- and 2nd-level City Object types (§ 2 The different City
/// Objects). Every variant is a unit variant and every identifier is already
/// spelled exactly as the spec requires (eg `TINRelief`, `WaterBody`,
/// `CityFurniture`), so serde's default (non-untagged) derive collapses each
/// one to its bare name string with no `#[serde(rename)]` needed.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum KnownCityObjectType {
    Bridge,
    BridgePart,
    BridgeInstallation,
    BridgeConstructiveElement,
    BridgeRoom,
    BridgeFurniture,
    Building,
    BuildingPart,
    BuildingInstallation,
    BuildingConstructiveElement,
    BuildingFurniture,
    BuildingStorey,
    BuildingRoom,
    BuildingUnit,
    CityFurniture,
    CityObjectGroup,
    GenericCityObject,
    LandUse,
    OtherConstruction,
    PlantCover,
    SolitaryVegetationObject,
    TINRelief,
    Road,
    Railway,
    Waterway,
    TransportSquare,
    Tunnel,
    TunnelPart,
    TunnelInstallation,
    TunnelConstructiveElement,
    TunnelHollowSpace,
    TunnelFurniture,
    WaterBody,
}

/// The `type` member of a City Object (§ 2 The different City Objects).
///
/// `Known` covers the 1st- and 2nd-level City Object types defined by the
/// CityJSON 2.0.1 spec. Any other value is a CityJSON Extension type, which
/// the spec requires to start with `"+"` (§ 8 Extensions) -- `Extension`
/// carries that string verbatim so a file using an Extension (eg
/// `"+NoiseCityFurnitureSegment"`) round-trips byte for byte.
///
/// This is `#[serde(untagged)]` over a *nested* known-variants enum, not a
/// single flat enum mixing unit variants with `Extension(String)` directly:
/// under `#[serde(untagged)]`, a bare unit variant (de)serializes as JSON
/// `null`, not its name (this is a documented serde quirk -- untagged
/// dispatch tries each variant's own `Deserialize` standalone, and a unit
/// variant's is `deserialize_unit`, which only accepts `null`). A flat
/// mixed enum was tried and empirically failed this exact way: every known
/// name fell through to `Extension` because none of the unit variants ever
/// matched a JSON string. Nesting the known names inside their own
/// unit-only enum sidesteps this, because *that* enum, derived without
/// `#[serde(untagged)]`, gets the ordinary external-tagging collapse (a
/// unit-only enum's tag+content is just the bare name string).
///
/// # Match on the value, not on a reference
///
/// Each known name is also available flat, as an associated `const` (eg
/// [`CityObjectType::BuildingPart`]) -- see the `impl` block below for the
/// full list. They are `const`s, not variants: this enum has exactly the two
/// variants above. That distinction bites in exactly one place, and it bites
/// silently, so it belongs here rather than in the small print:
///
/// ```
/// # use cjseq::{CityObjectType, KnownCityObjectType};
/// # let t = &CityObjectType::Building;
/// match *t {
///     CityObjectType::Building => { /* .. */ }
///     CityObjectType::Extension(ref s) => { let _ = s; }
///     _ => {}
/// }
/// ```
///
/// Note the `*`. Matching a `const` path against a `&CityObjectType` -- ie
/// `match t` where `t: &CityObjectType`, or `match &t` -- is `E0308`
/// ("mismatched types"), and rustc's fix-it suggests renaming the arm to a
/// fresh binding. That *compiles*, and silently turns the arm into a
/// catch-all that matches everything, because `Building` in pattern position
/// reads as a new irrefutable binding rather than as the constant. The `_`
/// arm is still required after `Extension(ref s)`: the match is not
/// exhaustive over just the known names, since `Known` and `Extension` are
/// the real variants.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum CityObjectType {
    Known(KnownCityObjectType),
    /// Not a core CityJSON type: a CityJSON Extension type, always spelled
    /// with a leading `"+"`.
    Extension(String),
}

/// Flat, spec-spelling access to each known variant (eg
/// `CityObjectType::BuildingPart`), so call sites read exactly like the
/// CityJSON type name rather than `CityObjectType::Known(KnownCityObjectType::BuildingPart)`.
/// These are associated `const`s, not enum variants -- `CityObjectType`
/// itself has exactly the two variants above -- but a `const` path is usable
/// wherever a variant path would be: in expressions (`x == CityObjectType::Bridge`)
/// and, because the type derives `PartialEq`/`Eq`, in match patterns too --
/// but only when matching on a *value*. See
/// [`CityObjectType`][CityObjectType#match-on-the-value-not-on-a-reference]
/// for why that matters and how it fails silently otherwise.
#[allow(non_upper_case_globals)]
impl CityObjectType {
    pub const Bridge: CityObjectType = CityObjectType::Known(KnownCityObjectType::Bridge);
    pub const BridgePart: CityObjectType = CityObjectType::Known(KnownCityObjectType::BridgePart);
    pub const BridgeInstallation: CityObjectType =
        CityObjectType::Known(KnownCityObjectType::BridgeInstallation);
    pub const BridgeConstructiveElement: CityObjectType =
        CityObjectType::Known(KnownCityObjectType::BridgeConstructiveElement);
    pub const BridgeRoom: CityObjectType = CityObjectType::Known(KnownCityObjectType::BridgeRoom);
    pub const BridgeFurniture: CityObjectType =
        CityObjectType::Known(KnownCityObjectType::BridgeFurniture);
    pub const Building: CityObjectType = CityObjectType::Known(KnownCityObjectType::Building);
    pub const BuildingPart: CityObjectType =
        CityObjectType::Known(KnownCityObjectType::BuildingPart);
    pub const BuildingInstallation: CityObjectType =
        CityObjectType::Known(KnownCityObjectType::BuildingInstallation);
    pub const BuildingConstructiveElement: CityObjectType =
        CityObjectType::Known(KnownCityObjectType::BuildingConstructiveElement);
    pub const BuildingFurniture: CityObjectType =
        CityObjectType::Known(KnownCityObjectType::BuildingFurniture);
    pub const BuildingStorey: CityObjectType =
        CityObjectType::Known(KnownCityObjectType::BuildingStorey);
    pub const BuildingRoom: CityObjectType =
        CityObjectType::Known(KnownCityObjectType::BuildingRoom);
    pub const BuildingUnit: CityObjectType =
        CityObjectType::Known(KnownCityObjectType::BuildingUnit);
    pub const CityFurniture: CityObjectType =
        CityObjectType::Known(KnownCityObjectType::CityFurniture);
    pub const CityObjectGroup: CityObjectType =
        CityObjectType::Known(KnownCityObjectType::CityObjectGroup);
    pub const GenericCityObject: CityObjectType =
        CityObjectType::Known(KnownCityObjectType::GenericCityObject);
    pub const LandUse: CityObjectType = CityObjectType::Known(KnownCityObjectType::LandUse);
    pub const OtherConstruction: CityObjectType =
        CityObjectType::Known(KnownCityObjectType::OtherConstruction);
    pub const PlantCover: CityObjectType = CityObjectType::Known(KnownCityObjectType::PlantCover);
    pub const SolitaryVegetationObject: CityObjectType =
        CityObjectType::Known(KnownCityObjectType::SolitaryVegetationObject);
    pub const TINRelief: CityObjectType = CityObjectType::Known(KnownCityObjectType::TINRelief);
    pub const Road: CityObjectType = CityObjectType::Known(KnownCityObjectType::Road);
    pub const Railway: CityObjectType = CityObjectType::Known(KnownCityObjectType::Railway);
    pub const Waterway: CityObjectType = CityObjectType::Known(KnownCityObjectType::Waterway);
    pub const TransportSquare: CityObjectType =
        CityObjectType::Known(KnownCityObjectType::TransportSquare);
    pub const Tunnel: CityObjectType = CityObjectType::Known(KnownCityObjectType::Tunnel);
    pub const TunnelPart: CityObjectType = CityObjectType::Known(KnownCityObjectType::TunnelPart);
    pub const TunnelInstallation: CityObjectType =
        CityObjectType::Known(KnownCityObjectType::TunnelInstallation);
    pub const TunnelConstructiveElement: CityObjectType =
        CityObjectType::Known(KnownCityObjectType::TunnelConstructiveElement);
    pub const TunnelHollowSpace: CityObjectType =
        CityObjectType::Known(KnownCityObjectType::TunnelHollowSpace);
    pub const TunnelFurniture: CityObjectType =
        CityObjectType::Known(KnownCityObjectType::TunnelFurniture);
    pub const WaterBody: CityObjectType = CityObjectType::Known(KnownCityObjectType::WaterBody);
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CityObject {
    #[serde(rename = "type")]
    pub thetype: CityObjectType,
    #[serde(rename = "geographicalExtent")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geographical_extent: Option<GeographicalExtent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geometry: Option<Vec<Geometry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<String>>,
    /// § 2.5 CityObjectGroup only: the role of each id in `children`, in the
    /// same order. `cityobjects.schema.json` types each item as
    /// `["string", "null"]`, so an unspecified role is `null`, not simply
    /// absent from the array.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children_roles: Option<Vec<Option<String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parents: Option<Vec<String>>,
    /// Members the schema does not name, carried verbatim so they survive a
    /// round trip. `pub`, like every other `#[serde(flatten)]` catch-all in
    /// this crate, so a `CityObject` can be written as a struct literal.
    #[serde(flatten)]
    pub other: serde_json::Value,
}

impl CityObject {
    /// A City Object of `thetype` carrying nothing else.
    ///
    /// `other` is deliberately not a constructor parameter: it is the
    /// `#[serde(flatten)]` catch-all for members the schema does not name, and
    /// a value built in code has none. Callers that need one set the named
    /// fields directly, which are all `pub`.
    ///
    /// This is a convenience, not a necessity: every field is `pub`, so a
    /// struct literal works too. It saves writing seven `None`s and getting
    /// the `other` sentinel right (see below).
    ///
    /// `other` is an empty *object*, not `Value::Null`: that is what
    /// deserializing a City Object with no unrecognized members produces, and
    /// the derived `PartialEq` compares it. A `Null` here would serialize
    /// identically but make every constructed object compare unequal to its
    /// parsed twin -- silently, and only ever noticed by a test that compares
    /// whole objects.
    pub fn new(thetype: CityObjectType) -> Self {
        CityObject {
            thetype,
            geographical_extent: None,
            attributes: None,
            geometry: None,
            children: None,
            children_roles: None,
            parents: None,
            other: Value::Object(serde_json::Map::new()),
        }
    }

    pub fn get_type(&self) -> CityObjectType {
        self.thetype.clone()
    }
    pub(crate) fn is_toplevel(&self) -> bool {
        match &self.parents {
            Some(x) => x.is_empty(),
            None => true,
        }
    }
    pub(crate) fn get_children_keys(&self) -> Vec<String> {
        let mut re: Vec<String> = Vec::new();
        if let Some(x) = &self.children {
            for each in x {
                re.push(each.to_string());
            }
        }
        re
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A constructed City Object must equal the one deserializing the
    /// equivalent minimal JSON produces. Serializing both and comparing the
    /// output is not enough: `other: Value::Null` and `other: Value::Object({})`
    /// both serialize to nothing, so only `==` distinguishes them -- and `==`
    /// is what a caller round-tripping a document through its own encoder will
    /// reach for.
    #[test]
    fn a_constructed_city_object_equals_its_parsed_twin() {
        let constructed = CityObject::new(CityObjectType::Building);
        let parsed: CityObject = serde_json::from_str(r#"{"type":"Building"}"#).unwrap();
        assert_eq!(constructed, parsed);
        assert_eq!(
            serde_json::to_value(&constructed).unwrap(),
            serde_json::json!({"type": "Building"})
        );

        // And the same for an Extension type, whose name lives in the tag
        // rather than in `other`.
        let constructed = CityObject::new(CityObjectType::Extension("+Noise".into()));
        let parsed: CityObject = serde_json::from_str(r#"{"type":"+Noise"}"#).unwrap();
        assert_eq!(constructed, parsed);
    }

    #[test]
    fn extension_city_object_type_roundtrips_with_its_plus() {
        let t: CityObjectType =
            serde_json::from_value(serde_json::json!("+NoiseCityFurnitureSegment")).unwrap();
        assert_eq!(
            t,
            CityObjectType::Extension("+NoiseCityFurnitureSegment".into())
        );
        assert_eq!(
            serde_json::to_value(&t).unwrap(),
            serde_json::json!("+NoiseCityFurnitureSegment")
        );
    }

    #[test]
    fn known_city_object_type_is_a_unit_variant() {
        let t: CityObjectType = serde_json::from_value(serde_json::json!("BuildingPart")).unwrap();
        assert_eq!(t, CityObjectType::BuildingPart);
    }

    /// A known name prefixed with `+` is not the known variant: extensions are
    /// spelled with a leading `+` precisely so they never collide with a core
    /// type, and ordering (known arm declared first) must not accidentally
    /// swallow it either.
    #[test]
    fn plus_prefixed_known_name_lands_in_extension_not_the_known_variant() {
        let t: CityObjectType = serde_json::from_value(serde_json::json!("+Building")).unwrap();
        assert_eq!(t, CityObjectType::Extension("+Building".into()));
        assert_ne!(t, CityObjectType::Building);
    }

    /// The stated critical requirement names a specific shape: a `CityObject`
    /// (inside a `CityJSONFeature`, since that is how `flatcitybuf`'s
    /// `noise_extension` fixture is shaped) whose `type` is an Extension
    /// string, where `CityObjectType` sits behind `CityObject`'s
    /// `#[serde(flatten)] other` field. The bare-`CityObjectType` tests above
    /// don't exercise that flatten path at all, so this pins it at the
    /// document level, asserting on the serialized *string* -- not a
    /// re-parsed `Value` -- so a change that reorders or reformats output
    /// would fail this test even though it would pass a structural check.
    ///
    /// The extra `"+noiseLevel"` member is deliberate and load-bearing: it is
    /// *not* `"attributes"` (that member has its own dedicated,
    /// already-typed `CityObject` field and so would never touch `other` at
    /// all) but a genuinely unrecognized top-level member, so it is the one
    /// that actually exercises `#[serde(flatten)] other: serde_json::Value`.
    /// It carries a nested object (not a bare scalar) to prove nested
    /// content survives too, and there is exactly one such extra member,
    /// because `other`'s `serde_json::Map` has no stable iteration order
    /// once it holds two or more keys.
    #[test]
    fn extension_typed_city_object_roundtrips_byte_for_byte_inside_a_feature() {
        let input = concat!(
            r#"{"type":"CityJSONFeature","id":"id-1","#,
            r#""CityObjects":{"id-1":{"type":"+NoiseCityFurnitureSegment","#,
            r#""+noiseLevel":{"dB":55}}},"#,
            r#""vertices":[]}"#
        );
        let parsed: crate::cityjson::CityJSONFeature = serde_json::from_str(input).unwrap();
        assert_eq!(
            parsed.city_objects["id-1"].thetype,
            CityObjectType::Extension("+NoiseCityFurnitureSegment".into())
        );
        let reserialized = serde_json::to_string(&parsed).unwrap();
        assert_eq!(
            reserialized, input,
            "an Extension-typed CityObject with an extra flattened member inside a \
             CityJSONFeature must round-trip byte-for-byte"
        );
    }

    /// Every public type in this crate derives `PartialEq`; `CityObject` was
    /// missing it.
    #[test]
    fn city_object_with_equal_fields_is_equal() {
        let input = serde_json::json!({"type": "Building"});
        let a: CityObject = serde_json::from_value(input.clone()).unwrap();
        let b: CityObject = serde_json::from_value(input).unwrap();
        assert_eq!(a, b);
        let mut c = b.clone();
        c.children = Some(vec!["id-2".to_string()]);
        assert_ne!(a, c);
    }

    /// `cityobjects.schema.json`'s `_AbstractCityObject.geographicalExtent`
    /// has `minItems`/`maxItems` 6 -- a fixed-length array, not an arbitrary
    /// `Vec<f64>` -- so a 5- or 7-element extent must fail to deserialize
    /// rather than being silently accepted.
    #[test]
    fn city_object_geographical_extent_rejects_the_wrong_length() {
        let too_short = serde_json::json!({
            "type": "Building",
            "geographicalExtent": [0.0, 0.0, 0.0, 1.0, 1.0]
        });
        assert!(serde_json::from_value::<CityObject>(too_short).is_err());

        let too_long = serde_json::json!({
            "type": "Building",
            "geographicalExtent": [0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 2.0]
        });
        assert!(serde_json::from_value::<CityObject>(too_long).is_err());
    }

    #[test]
    fn city_object_geographical_extent_round_trips_as_a_six_element_array() {
        let input = serde_json::json!({
            "type": "Building",
            "geographicalExtent": [84710.1, 446846.0, -5.3, 84757.1, 446944.0, 40.9]
        });
        let co: CityObject = serde_json::from_value(input.clone()).unwrap();
        assert_eq!(
            co.geographical_extent,
            Some([84710.1, 446846.0, -5.3, 84757.1, 446944.0, 40.9])
        );
        assert_eq!(serde_json::to_value(&co).unwrap(), input);
    }

    /// § 2.5 CityObjectGroup: a group `CityObject` may carry a `children_roles`
    /// member, one entry per id in `children`, describing that child's role
    /// in the group. `cityobjects.schema.json`'s `CityObjectGroup` types each
    /// item as `["string", "null"]`, so an unspecified role is `null`, not an
    /// absent array entry -- this must round-trip too.
    #[test]
    fn city_object_group_children_roles_round_trips_including_null_entries() {
        let input = serde_json::json!({
            "type": "CityObjectGroup",
            "children": ["building1", "building3"],
            "children_roles": ["residential building", null]
        });
        let co: CityObject = serde_json::from_value(input.clone()).unwrap();
        assert_eq!(
            co.children_roles,
            Some(vec![Some("residential building".to_string()), None])
        );
        assert_eq!(serde_json::to_value(&co).unwrap(), input);
    }

    /// Every known CityObjectType name round-trips through its unit variant,
    /// not through Extension -- a typo'd rename would otherwise silently fall
    /// through to Extension and no other test would catch it.
    #[test]
    fn every_known_city_object_type_round_trips_as_its_unit_variant() {
        let known: &[(&str, CityObjectType)] = &[
            ("Bridge", CityObjectType::Bridge),
            ("BridgePart", CityObjectType::BridgePart),
            ("BridgeInstallation", CityObjectType::BridgeInstallation),
            (
                "BridgeConstructiveElement",
                CityObjectType::BridgeConstructiveElement,
            ),
            ("BridgeRoom", CityObjectType::BridgeRoom),
            ("BridgeFurniture", CityObjectType::BridgeFurniture),
            ("Building", CityObjectType::Building),
            ("BuildingPart", CityObjectType::BuildingPart),
            ("BuildingInstallation", CityObjectType::BuildingInstallation),
            (
                "BuildingConstructiveElement",
                CityObjectType::BuildingConstructiveElement,
            ),
            ("BuildingFurniture", CityObjectType::BuildingFurniture),
            ("BuildingStorey", CityObjectType::BuildingStorey),
            ("BuildingRoom", CityObjectType::BuildingRoom),
            ("BuildingUnit", CityObjectType::BuildingUnit),
            ("CityFurniture", CityObjectType::CityFurniture),
            ("CityObjectGroup", CityObjectType::CityObjectGroup),
            ("GenericCityObject", CityObjectType::GenericCityObject),
            ("LandUse", CityObjectType::LandUse),
            ("OtherConstruction", CityObjectType::OtherConstruction),
            ("PlantCover", CityObjectType::PlantCover),
            (
                "SolitaryVegetationObject",
                CityObjectType::SolitaryVegetationObject,
            ),
            ("TINRelief", CityObjectType::TINRelief),
            ("Road", CityObjectType::Road),
            ("Railway", CityObjectType::Railway),
            ("Waterway", CityObjectType::Waterway),
            ("TransportSquare", CityObjectType::TransportSquare),
            ("Tunnel", CityObjectType::Tunnel),
            ("TunnelPart", CityObjectType::TunnelPart),
            ("TunnelInstallation", CityObjectType::TunnelInstallation),
            (
                "TunnelConstructiveElement",
                CityObjectType::TunnelConstructiveElement,
            ),
            ("TunnelHollowSpace", CityObjectType::TunnelHollowSpace),
            ("TunnelFurniture", CityObjectType::TunnelFurniture),
            ("WaterBody", CityObjectType::WaterBody),
        ];
        for (name, expected) in known {
            let parsed: CityObjectType = serde_json::from_value(serde_json::json!(name)).unwrap();
            assert_eq!(&parsed, expected, "{name} did not parse to its own variant");
            assert_eq!(
                serde_json::to_value(&parsed).unwrap(),
                serde_json::json!(name),
                "{name} did not round-trip back to its own string"
            );
        }
    }
}
