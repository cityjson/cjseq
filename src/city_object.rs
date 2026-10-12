use crate::geometry::Geometry;
use crate::metadata::GeographicalExtent;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The 1st- and 2nd-level City Object types the spec defines (§ 2), each
/// serialized as its name.
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

/// The `type` of a City Object (§ 2): a known type, or an Extension type,
/// which starts with `+` (eg `"+NoiseCityFurnitureSegment"`, § 8) and is kept
/// verbatim. In JSON it is a string; a string that is neither a known name nor
/// `+` followed by a name is an error, so a misspelt core type (`"Buildng"`)
/// does not pass as an Extension.
///
/// # Matching
///
/// Each known type is also an associated `const` ([`CityObjectType::BuildingPart`]
/// and so on). Match on a value, not a reference:
///
/// ```
/// # use cjseq::CityObjectType;
/// # let t = &CityObjectType::Building;
/// match *t {
///     CityObjectType::Building => { /* .. */ }
///     CityObjectType::Extension(ref s) => { let _ = s; }
///     _ => {}
/// }
/// ```
///
/// Without the `*` the match does not compile, and the fix rustc suggests
/// turns the `Building` arm into a binding that matches everything. The `_` arm
/// is needed because the real variants are `Known` and `Extension`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(try_from = "String", into = "String")]
pub enum CityObjectType {
    Known(KnownCityObjectType),
    /// An Extension type, with its leading `+`.
    Extension(String),
}

impl TryFrom<String> for CityObjectType {
    type Error = crate::error::CjseqError;
    /// A known name, or an Extension name: `+` followed by at least one
    /// character. Anything else -- a misspelt core type, say -- is an error.
    fn try_from(s: String) -> crate::error::Result<Self> {
        if let Ok(k) = serde_json::from_value::<KnownCityObjectType>(Value::String(s.clone())) {
            return Ok(CityObjectType::Known(k));
        }
        if s.len() > 1 && s.starts_with('+') {
            return Ok(CityObjectType::Extension(s));
        }
        Err(crate::error::CjseqError::Validation(format!(
            "{s:?} is not a City Object type; an Extension type starts with '+'"
        )))
    }
}

impl From<CityObjectType> for String {
    fn from(t: CityObjectType) -> String {
        match t {
            CityObjectType::Known(k) => match serde_json::to_value(k) {
                Ok(Value::String(s)) => s,
                _ => unreachable!("a unit variant serializes as its name"),
            },
            CityObjectType::Extension(s) => s,
        }
    }
}

/// Each known type as a constant, spelled as in the spec; see
/// [`CityObjectType`] on matching them.
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
    pub attributes: Option<Map<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geometry: Option<Vec<Geometry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<String>>,
    /// CityObjectGroup only (§ 2.5): the role of each id in `children`, in the
    /// same order; an unspecified role is `null`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children_roles: Option<Vec<Option<String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parents: Option<Vec<String>>,
    /// Members the schema does not name, kept verbatim.
    #[serde(flatten)]
    pub other: Map<String, Value>,
}

impl CityObject {
    /// A City Object of `thetype` with nothing else; it equals the one parsed
    /// from `{"type": ...}`. Every field is `pub`, so set the others directly.
    pub fn new(thetype: CityObjectType) -> Self {
        CityObject {
            thetype,
            geographical_extent: None,
            attributes: None,
            geometry: None,
            children: None,
            children_roles: None,
            parents: None,
            other: Map::new(),
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
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A constructed City Object equals the one parsed from the same JSON, which
    /// only `==` can tell: an empty `other` and a missing one serialize alike.
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

    /// A known name with a `+` is an Extension type, not the known one.
    #[test]
    fn plus_prefixed_known_name_lands_in_extension_not_the_known_variant() {
        let t: CityObjectType = serde_json::from_value(serde_json::json!("+Building")).unwrap();
        assert_eq!(t, CityObjectType::Extension("+Building".into()));
        assert_ne!(t, CityObjectType::Building);
    }

    /// An Extension-typed City Object with an extra, nested member, inside a
    /// feature, round-trips byte for byte.
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

    /// `geographicalExtent` has exactly six numbers.
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

    /// `children_roles` round-trips, `null` roles included.
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

    /// Every known name parses to its own variant, not to `Extension`.
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
