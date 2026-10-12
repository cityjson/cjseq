use crate::geometry::GeometryType;
use std::fmt;

/// The crate's error type.
///
/// Implemented by hand: this crate does not hand-write serde impls, and it
/// does not take a derive dependency for its error type either.
#[derive(Debug)]
pub enum CjseqError {
    /// A value was not the JSON the type expected, and nothing more specific
    /// could be said about why.
    Json(serde_json::Error),
    /// A geometry's `boundaries` were nested to a depth its `type` does not
    /// allow. serde reports this as "invalid type: integer, expected a
    /// sequence", which names neither the geometry nor the depth it wanted;
    /// this variant names both.
    GeometryDepth {
        geometry_type: GeometryType,
        /// The nesting depth the geometry type requires.
        expected: usize,
        /// The nesting depth the `boundaries` actually had.
        found: usize,
    },
    /// An I/O operation failed, eg opening or reading a CityJSON(Seq) file.
    Io(std::io::Error),
    /// A document parsed as valid JSON, and every typed field on it parsed
    /// fine, but it still fails a structural rule this crate enforces beyond
    /// what serde alone can express -- eg its `type` is not `"CityJSON"`, or
    /// its `version` is not one this crate supports.
    Validation(String),
    /// A City Object names, as one of its children, an id that is not in the
    /// document.
    DanglingReference {
        /// The City Object holding the reference.
        from: String,
        /// The id it refers to.
        to: String,
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
            CjseqError::Validation(_)
            | CjseqError::DanglingReference { .. }
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
