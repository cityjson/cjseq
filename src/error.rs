use crate::geometry::GeometryType;
use std::fmt;

/// The crate's error type.
#[derive(Debug)]
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
