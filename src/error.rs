use crate::geometry::GeometryType;
use std::fmt;

/// The crate's error type.
#[derive(Debug)]
#[non_exhaustive]
pub enum CjseqError {
    /// A value was not the JSON the type expected, and nothing more specific
    /// could be said about why.
    Json(serde_json::Error),
    /// A geometry's `boundaries` are nested to a depth its `type` does not
    /// allow; see [`crate::Geometry::from_json_value`].
    GeometryDepth {
        geometry_type: GeometryType,
        /// The nesting depth the geometry type requires.
        expected: usize,
        /// The nesting depth the `boundaries` actually had.
        found: usize,
    },
    /// An I/O operation failed, eg opening or reading a CityJSON(Seq) file.
    Io(std::io::Error),
    /// A rule beyond what the types express, eg a material theme with both
    /// `value` and `values`, or a malformed LoD or texture ring.
    Validation(String),
    /// A City Object (or a feature's `id`) refers to an id that is not there.
    DanglingReference {
        /// The City Object holding the reference.
        from: String,
        /// The id it refers to.
        to: String,
    },
    /// A parent and a child disagree: one names the other but is not named
    /// back (`children` vs `parents`).
    HierarchyMismatch { parent: String, child: String },
    /// An array of values has a different number of entries than the part
    /// of the geometry it describes.
    ValueCount {
        /// What the values are, eg `"semantics values"`.
        what: &'static str,
        /// The number the boundaries call for.
        expected: usize,
        found: usize,
    },
    /// A core City Object has an Extension City Object as a child, which
    /// § 8.7 rule 3 says it should not. Reported by `warnings()`, not
    /// `errors()`: the Noise Extension published with the spec does it.
    ExtensionChildOfCoreObject { parent: String, child: String },
    /// Collecting dropped a root member of a CityJSONFeature: the feature
    /// schema allows any, but a CityJSON document has nowhere to keep them
    /// per feature. Reported by [`crate::Collector::warnings`].
    DroppedMember { feature: String, member: String },
    /// City Objects that no feature reaches: their parent is missing, or
    /// does not list them as children.
    NotInAnyFeature { ids: Vec<String> },
    /// Two CityJSONSeq headers being collected into one document disagree
    /// on something that cannot be merged.
    HeaderMismatch { member: String },
    /// Collecting dropped a member of a later sequence's header that
    /// differed from the document's. Reported by
    /// [`crate::Collector::warnings`].
    DroppedHeaderMember { member: String },
    /// A City Object id that is already in the document, met again while
    /// collecting.
    DuplicateId { id: String },
    /// A name or version does not match the pattern the schema gives it.
    Pattern {
        /// What the value is, eg `"Extension City Object type"`.
        what: &'static str,
        value: String,
        /// The pattern, in words.
        expected: &'static str,
    },
    /// A CityJSONFeature carries a root member only a CityJSON document may
    /// carry, eg `transform`.
    MisplacedMember { member: String },
    /// An error inside one City Object.
    InCityObject { id: String, source: Box<CjseqError> },
    /// An error on one line of a CityJSONSeq (counting from 1).
    AtLine {
        line: usize,
        source: Box<CjseqError>,
    },
    /// An error inside one geometry template.
    InTemplate {
        index: usize,
        source: Box<CjseqError>,
    },
    /// An index points past the end of the array it indexes.
    IndexOutOfRange {
        kind: IndexKind,
        index: usize,
        /// The length of the array indexed.
        len: usize,
    },
}

/// What an out-of-range index was indexing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum IndexKind {
    Vertex,
    TextureVertex,
    Material,
    Texture,
    Surface,
    Template,
}

impl fmt::Display for CjseqError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CjseqError::Json(e) => write!(f, "invalid JSON: {e}"),
            CjseqError::GeometryDepth {
                geometry_type,
                expected,
                found,
            } => write!(
                f,
                "a {geometry_type:?} nests its boundaries {expected} levels deep, but these boundaries are nested {found} levels deep"
            ),
            CjseqError::Io(e) => write!(f, "I/O error: {e}"),
            CjseqError::Validation(msg) => write!(f, "invalid CityJSON: {msg}"),
            CjseqError::DanglingReference { from, to } => write!(
                f,
                "City Object {from:?} refers to {to:?}, which is not in the document"
            ),
            CjseqError::HierarchyMismatch { parent, child } => write!(
                f,
                "City Objects {parent:?} and {child:?} disagree on whether one is the other's parent"
            ),
            CjseqError::ValueCount {
                what,
                expected,
                found,
            } => write!(f, "{found} {what} where the boundaries call for {expected}"),
            CjseqError::ExtensionChildOfCoreObject { parent, child } => write!(
                f,
                "core City Object {parent:?} has the Extension City Object {child:?} as a child (§ 8.7 rule 3)"
            ),
            CjseqError::DroppedMember { feature, member } => write!(
                f,
                "dropped the root member {member:?} of CityJSONFeature {feature:?}"
            ),
            CjseqError::NotInAnyFeature { ids } => write!(
                f,
                "City Objects in no feature (a parent is missing or does not list them as children): {}",
                ids.join(", ")
            ),
            CjseqError::HeaderMismatch { member } => write!(
                f,
                "the sequences being collected disagree on {member}"
            ),
            CjseqError::DroppedHeaderMember { member } => write!(
                f,
                "dropped {member:?} of a later sequence's header, which differs from the first"
            ),
            CjseqError::DuplicateId { id } => {
                write!(f, "City Object {id:?} is in more than one feature")
            }
            CjseqError::Pattern {
                what,
                value,
                expected,
            } => write!(f, "{what} {value:?} is not {expected}"),
            CjseqError::MisplacedMember { member } => write!(
                f,
                "a CityJSONFeature cannot carry {member:?}; only the CityJSON document can"
            ),
            CjseqError::InCityObject { id, source } => write!(f, "in City Object {id:?}: {source}"),
            CjseqError::AtLine { line, source } => write!(f, "line {line}: {source}"),
            CjseqError::InTemplate { index, source } => {
                write!(f, "in geometry template {index}: {source}")
            }
            CjseqError::IndexOutOfRange { kind, index, len } => {
                write!(f, "{kind:?} index {index} is out of range (there are {len})")
            }
        }
    }
}

impl std::error::Error for CjseqError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CjseqError::Json(e) => Some(e),
            CjseqError::GeometryDepth { .. } => None,
            CjseqError::Io(e) => Some(e),
            CjseqError::InCityObject { source, .. }
            | CjseqError::InTemplate { source, .. }
            | CjseqError::AtLine { source, .. } => Some(source.as_ref()),
            CjseqError::Validation(_)
            | CjseqError::DanglingReference { .. }
            | CjseqError::HierarchyMismatch { .. }
            | CjseqError::ValueCount { .. }
            | CjseqError::MisplacedMember { .. }
            | CjseqError::Pattern { .. }
            | CjseqError::ExtensionChildOfCoreObject { .. }
            | CjseqError::DroppedMember { .. }
            | CjseqError::NotInAnyFeature { .. }
            | CjseqError::HeaderMismatch { .. }
            | CjseqError::DuplicateId { .. }
            | CjseqError::DroppedHeaderMember { .. }
            | CjseqError::IndexOutOfRange { .. } => None,
        }
    }
}

impl From<serde_json::Error> for CjseqError {
    fn from(e: serde_json::Error) -> Self {
        CjseqError::Json(e)
    }
}

impl From<std::io::Error> for CjseqError {
    fn from(e: std::io::Error) -> Self {
        CjseqError::Io(e)
    }
}

/// The crate's result type.
pub type Result<T> = std::result::Result<T, CjseqError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_converts_via_from_and_keeps_its_message() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "nope");
        let err: CjseqError = io_err.into();
        assert!(matches!(err, CjseqError::Io(_)));
        assert_eq!(err.to_string(), "I/O error: nope");
    }

    #[test]
    fn validation_error_carries_its_message() {
        let err = CjseqError::Validation("Input file not CityJSON.".to_string());
        assert_eq!(
            err.to_string(),
            "invalid CityJSON: Input file not CityJSON."
        );
    }
}
