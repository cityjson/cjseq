use crate::index::{Indices, PrimitiveValues, SurfaceIndex};
#[cfg(doc)]
use crate::index::{MultiSolidValues, SolidValues};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The semantic surface types the spec defines (§ 3.3), each serialized as
/// its name.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum KnownSemanticSurfaceType {
    RoofSurface,
    GroundSurface,
    WallSurface,
    ClosureSurface,
    OuterCeilingSurface,
    OuterFloorSurface,
    Window,
    Door,
    InteriorWallSurface,
    CeilingSurface,
    FloorSurface,
    WaterSurface,
    WaterGroundSurface,
    WaterClosureSurface,
    TrafficArea,
    AuxiliaryTrafficArea,
    TransportationMarking,
    TransportationHole,
}

/// The `type` of a semantic surface (§ 3.3): a known type, or an Extension
/// type, which starts with `+` (eg `"+ThermalSurface"`, § 8.5) and is kept
/// verbatim. In JSON it is a string; a string that is neither a known name nor
/// `+` followed by a name is an error.
///
/// # Matching
///
/// Each known type is also an associated `const` ([`SemanticSurfaceType::RoofSurface`]
/// and so on). Match on a value, not a reference:
///
/// ```
/// # use cjseq::SemanticSurfaceType;
/// # let t = &SemanticSurfaceType::RoofSurface;
/// match *t {
///     SemanticSurfaceType::RoofSurface => { /* .. */ }
///     SemanticSurfaceType::Extension(ref s) => { let _ = s; }
///     _ => {}
/// }
/// ```
///
/// Without the `*` the match does not compile, and the fix rustc suggests
/// turns the `RoofSurface` arm into a binding that matches everything. The `_`
/// arm is needed because the real variants are `Known` and `Extension`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(try_from = "String", into = "String")]
pub enum SemanticSurfaceType {
    Known(KnownSemanticSurfaceType),
    /// An Extension type, with its leading `+`.
    Extension(String),
}

impl TryFrom<String> for SemanticSurfaceType {
    type Error = crate::error::CjseqError;
    /// A known name, or an Extension name: `+` followed by at least one
    /// character. Anything else -- a misspelt core type, say -- is an error.
    fn try_from(s: String) -> crate::error::Result<Self> {
        if let Ok(k) = serde_json::from_value::<KnownSemanticSurfaceType>(Value::String(s.clone()))
        {
            return Ok(SemanticSurfaceType::Known(k));
        }
        if s.len() > 1 && s.starts_with('+') {
            return Ok(SemanticSurfaceType::Extension(s));
        }
        Err(crate::error::CjseqError::Validation(format!(
            "{s:?} is not a semantic surface type; an Extension type starts with '+'"
        )))
    }
}

impl From<SemanticSurfaceType> for String {
    fn from(t: SemanticSurfaceType) -> String {
        match t {
            SemanticSurfaceType::Known(k) => match serde_json::to_value(k) {
                Ok(Value::String(s)) => s,
                _ => unreachable!("a unit variant serializes as its name"),
            },
            SemanticSurfaceType::Extension(s) => s,
        }
    }
}

/// Each known type as a constant, spelled as in the spec; see
/// [`SemanticSurfaceType`] on matching them.
#[allow(non_upper_case_globals)]
impl SemanticSurfaceType {
    pub const RoofSurface: SemanticSurfaceType =
        SemanticSurfaceType::Known(KnownSemanticSurfaceType::RoofSurface);
    pub const GroundSurface: SemanticSurfaceType =
        SemanticSurfaceType::Known(KnownSemanticSurfaceType::GroundSurface);
    pub const WallSurface: SemanticSurfaceType =
        SemanticSurfaceType::Known(KnownSemanticSurfaceType::WallSurface);
    pub const ClosureSurface: SemanticSurfaceType =
        SemanticSurfaceType::Known(KnownSemanticSurfaceType::ClosureSurface);
    pub const OuterCeilingSurface: SemanticSurfaceType =
        SemanticSurfaceType::Known(KnownSemanticSurfaceType::OuterCeilingSurface);
    pub const OuterFloorSurface: SemanticSurfaceType =
        SemanticSurfaceType::Known(KnownSemanticSurfaceType::OuterFloorSurface);
    pub const Window: SemanticSurfaceType =
        SemanticSurfaceType::Known(KnownSemanticSurfaceType::Window);
    pub const Door: SemanticSurfaceType =
        SemanticSurfaceType::Known(KnownSemanticSurfaceType::Door);
    pub const InteriorWallSurface: SemanticSurfaceType =
        SemanticSurfaceType::Known(KnownSemanticSurfaceType::InteriorWallSurface);
    pub const CeilingSurface: SemanticSurfaceType =
        SemanticSurfaceType::Known(KnownSemanticSurfaceType::CeilingSurface);
    pub const FloorSurface: SemanticSurfaceType =
        SemanticSurfaceType::Known(KnownSemanticSurfaceType::FloorSurface);
    pub const WaterSurface: SemanticSurfaceType =
        SemanticSurfaceType::Known(KnownSemanticSurfaceType::WaterSurface);
    pub const WaterGroundSurface: SemanticSurfaceType =
        SemanticSurfaceType::Known(KnownSemanticSurfaceType::WaterGroundSurface);
    pub const WaterClosureSurface: SemanticSurfaceType =
        SemanticSurfaceType::Known(KnownSemanticSurfaceType::WaterClosureSurface);
    pub const TrafficArea: SemanticSurfaceType =
        SemanticSurfaceType::Known(KnownSemanticSurfaceType::TrafficArea);
    pub const AuxiliaryTrafficArea: SemanticSurfaceType =
        SemanticSurfaceType::Known(KnownSemanticSurfaceType::AuxiliaryTrafficArea);
    pub const TransportationMarking: SemanticSurfaceType =
        SemanticSurfaceType::Known(KnownSemanticSurfaceType::TransportationMarking);
    pub const TransportationHole: SemanticSurfaceType =
        SemanticSurfaceType::Known(KnownSemanticSurfaceType::TransportationHole);
}

/// One semantic surface: its type, its place in the parent/children hierarchy,
/// and any further attributes the file chooses to carry.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SemanticsSurface {
    #[serde(rename = "type")]
    pub thetype: SemanticSurfaceType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<SurfaceIndex>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<SurfaceIndex>>,
    /// CityJSON lets a semantic surface carry arbitrary extra members
    /// (`slope`, `direction`, ...); they are kept verbatim so that a file
    /// round-trips without losing them.
    #[serde(flatten)]
    pub other: Map<String, Value>,
}

/// The `semantics` member of a geometry: the semantic surfaces, and `values`
/// assigning one of them to each primitive. `V` is the shape of `values`,
/// fixed by the geometry type -- [`PrimitiveValues`], [`SolidValues`] or
/// [`MultiSolidValues`] of [`SurfaceIndex`] -- so values nested to the wrong
/// depth for their geometry fail to parse.
///
/// `values` is required but may be `null`, so it is written out even when
/// `None`. The `semantics` object is open: members the schema does not name
/// are kept in `other`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Semantics<V = PrimitiveValues<SurfaceIndex>> {
    pub surfaces: Vec<SemanticsSurface>,
    pub values: Option<V>,
    #[serde(flatten)]
    pub other: Map<String, Value>,
}

impl<V: Indices<SurfaceIndex>> Indices<SurfaceIndex> for Semantics<V> {
    fn for_each_index(&mut self, f: &mut dyn FnMut(&mut SurfaceIndex)) {
        for s in &mut self.surfaces {
            s.parent.for_each_index(f);
            s.children.for_each_index(f);
        }
        self.values.for_each_index(f);
    }
    fn visit_index(&self, f: &mut dyn FnMut(SurfaceIndex)) {
        for s in &self.surfaces {
            s.parent.visit_index(f);
            s.children.visit_index(f);
        }
        self.values.visit_index(f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `semantics` object is open, so members it does not name are kept.
    #[test]
    fn semantics_keeps_members_the_schema_does_not_name() {
        let input = serde_json::json!({
            "surfaces": [{"type": "RoofSurface"}],
            "values": [0],
            "vendorData": {"x": 1}
        });
        let s: Semantics = serde_json::from_value(input.clone()).unwrap();
        assert_eq!(s.other["vendorData"], serde_json::json!({"x": 1}));
        assert_eq!(serde_json::to_value(&s).unwrap(), input);
    }

    /// And with nothing extra, nothing extra appears.
    #[test]
    fn semantics_without_extras_serializes_unchanged() {
        let input = serde_json::json!({
            "surfaces": [{"type": "RoofSurface"}],
            "values": null
        });
        let s: Semantics = serde_json::from_value(input.clone()).unwrap();
        assert!(s.other.is_empty());
        assert_eq!(serde_json::to_value(&s).unwrap(), input);
    }

    #[test]
    fn solid_semantics_values_are_one_per_surface_per_shell() {
        let s: Semantics<crate::SolidValues<SurfaceIndex>> =
            serde_json::from_value(serde_json::json!({
                "surfaces": [{"type": "RoofSurface"}, {"type": "WallSurface"}],
                "values": [[0, 1, null]]
            }))
            .unwrap();
        assert_eq!(
            serde_json::to_value(&s).unwrap()["values"],
            serde_json::json!([[0, 1, null]])
        );
    }

    #[test]
    fn null_semantics_index_stays_null() {
        let s: Semantics = serde_json::from_value(serde_json::json!({
            "surfaces": [{"type": "RoofSurface"}], "values": [null, 0]
        }))
        .unwrap();
        //-- `null`, never `[]`
        assert_eq!(
            serde_json::to_value(&s).unwrap()["values"],
            serde_json::json!([null, 0])
        );
    }

    #[test]
    fn semantics_surface_and_semantics_implement_eq() {
        fn assert_impls_eq<T: Eq>() {}
        assert_impls_eq::<SemanticsSurface>();
        assert_impls_eq::<Semantics>();
    }

    /// A semantic surface's hierarchy and extra members round-trip.
    #[test]
    fn surface_extras_and_hierarchy_round_trip() {
        let input = serde_json::json!({
            "surfaces": [
                {"type": "RoofSurface", "slope": 33.4, "children": [1]},
                {"type": "Window", "parent": 0, "direction": "north"}
            ],
            "values": [0, 1]
        });
        let s: Semantics = serde_json::from_value(input.clone()).unwrap();
        assert_eq!(s.surfaces[0].children, Some(vec![SurfaceIndex(1)]));
        assert_eq!(s.surfaces[1].parent, Some(SurfaceIndex(0)));
        assert_eq!(
            s.surfaces[1].other["direction"],
            serde_json::json!("north"),
            "extra members must be kept, not dropped"
        );
        assert_eq!(serde_json::to_value(&s).unwrap(), input);
    }

    /// With no extra members, none are written.
    #[test]
    fn surface_with_no_extras_round_trips_without_spurious_members() {
        let input = serde_json::json!({
            "surfaces": [{"type": "RoofSurface"}],
            "values": [0]
        });
        let s: Semantics = serde_json::from_value(input.clone()).unwrap();
        assert!(s.surfaces[0].other.is_empty());
        assert_eq!(serde_json::to_value(&s).unwrap(), input);
    }

    /// `values` is required but may be `null`, so a `null` is written back, not
    /// dropped.
    #[test]
    fn a_null_semantics_values_member_round_trips() {
        let input = serde_json::json!({
            "surfaces": [{"type": "RoofSurface"}], "values": null
        });
        let s: Semantics =
            serde_json::from_value(input.clone()).expect("`values: null` is schema-valid CityJSON");
        assert_eq!(s.values, None);
        assert_eq!(serde_json::to_value(&s).unwrap(), input);
    }

    #[test]
    fn extension_semantic_surface_type_roundtrips_with_its_plus() {
        let t: SemanticSurfaceType =
            serde_json::from_value(serde_json::json!("+ThermalSurface")).unwrap();
        assert_eq!(t, SemanticSurfaceType::Extension("+ThermalSurface".into()));
        assert_eq!(
            serde_json::to_value(&t).unwrap(),
            serde_json::json!("+ThermalSurface")
        );
    }

    #[test]
    fn plus_prefixed_known_surface_name_lands_in_extension() {
        let t: SemanticSurfaceType =
            serde_json::from_value(serde_json::json!("+RoofSurface")).unwrap();
        assert_eq!(t, SemanticSurfaceType::Extension("+RoofSurface".into()));
        assert_ne!(t, SemanticSurfaceType::RoofSurface);
    }

    /// An Extension surface with an extra member, inside a geometry, round-trips
    /// byte for byte.
    #[test]
    fn extension_typed_semantics_surface_roundtrips_byte_for_byte_inside_a_geometry() {
        let input = concat!(
            r#"{"type":"MultiSurface","lod":"1","boundaries":[[[0,1,2]]],"#,
            r#""semantics":{"surfaces":[{"type":"+ThermalSurface","azimuth":30.5}],"values":[0]}}"#
        );
        let parsed: crate::geometry::Geometry = serde_json::from_str(input).unwrap();
        let reserialized = serde_json::to_string(&parsed).unwrap();
        assert_eq!(
            reserialized, input,
            "an Extension-typed SemanticsSurface with an extra flattened member inside a \
             Geometry must round-trip byte-for-byte"
        );
    }

    /// Every known name parses to its own variant, not to `Extension`.
    #[test]
    fn every_known_semantic_surface_type_round_trips_as_its_unit_variant() {
        let known: &[(&str, SemanticSurfaceType)] = &[
            ("RoofSurface", SemanticSurfaceType::RoofSurface),
            ("GroundSurface", SemanticSurfaceType::GroundSurface),
            ("WallSurface", SemanticSurfaceType::WallSurface),
            ("ClosureSurface", SemanticSurfaceType::ClosureSurface),
            (
                "OuterCeilingSurface",
                SemanticSurfaceType::OuterCeilingSurface,
            ),
            ("OuterFloorSurface", SemanticSurfaceType::OuterFloorSurface),
            ("Window", SemanticSurfaceType::Window),
            ("Door", SemanticSurfaceType::Door),
            (
                "InteriorWallSurface",
                SemanticSurfaceType::InteriorWallSurface,
            ),
            ("CeilingSurface", SemanticSurfaceType::CeilingSurface),
            ("FloorSurface", SemanticSurfaceType::FloorSurface),
            ("WaterSurface", SemanticSurfaceType::WaterSurface),
            (
                "WaterGroundSurface",
                SemanticSurfaceType::WaterGroundSurface,
            ),
            (
                "WaterClosureSurface",
                SemanticSurfaceType::WaterClosureSurface,
            ),
            ("TrafficArea", SemanticSurfaceType::TrafficArea),
            (
                "AuxiliaryTrafficArea",
                SemanticSurfaceType::AuxiliaryTrafficArea,
            ),
            (
                "TransportationMarking",
                SemanticSurfaceType::TransportationMarking,
            ),
            (
                "TransportationHole",
                SemanticSurfaceType::TransportationHole,
            ),
        ];
        for (name, expected) in known {
            let parsed: SemanticSurfaceType =
                serde_json::from_value(serde_json::json!(name)).unwrap();
            assert_eq!(&parsed, expected, "{name} did not parse to its own variant");
            assert_eq!(
                serde_json::to_value(&parsed).unwrap(),
                serde_json::json!(name),
                "{name} did not round-trip back to its own string"
            );
        }
    }
}
