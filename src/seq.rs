//! Reading and writing CityJSONSeq (CityJSON Text Sequences, § 7.2): one
//! CityJSON header line, then one CityJSONFeature per line.

use crate::cityjson::{CityJSON, CityJSONFeature};
use crate::error::{CjseqError, Result};
use serde::Deserialize;
use std::io::{BufRead, Write};
use std::str::FromStr;

/// One line of a CityJSONSeq.
//-- a Line is matched on and taken apart as soon as it is read, so the size
//-- difference between the two never sits in a collection
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum Line {
    /// A `"type": "CityJSON"` line: the first line of a sequence, or of each
    /// sequence when several are concatenated.
    Header(CityJSON),
    /// A `"type": "CityJSONFeature"` line.
    Feature(CityJSONFeature),
}

/// Just the `type` member of a line, borrowed.
#[derive(Deserialize)]
struct TypeOnly<'a> {
    #[serde(rename = "type", borrow)]
    thetype: std::borrow::Cow<'a, str>,
}

impl FromStr for Line {
    type Err = CjseqError;
    /// Parse one line. Nearly every line is a feature, so it is parsed as one
    /// straight away; only when that fails is the line's `type` looked at, to
    /// parse a header or report the feature's own error.
    fn from_str(s: &str) -> Result<Line> {
        let feature_err = match serde_json::from_str::<CityJSONFeature>(s) {
            Ok(f) => return Ok(Line::Feature(f)),
            Err(e) => e,
        };
        match serde_json::from_str::<TypeOnly>(s) {
            Ok(t) if t.thetype == "CityJSON" => Ok(Line::Header(serde_json::from_str(s)?)),
            Ok(t) if t.thetype == "CityJSONFeature" => Err(feature_err.into()),
            Ok(t) => Err(CjseqError::Validation(format!(
                "a CityJSONSeq line is a \"CityJSON\" or a \"CityJSONFeature\", not {:?}",
                t.thetype
            ))),
            Err(_) => Err(feature_err.into()),
        }
    }
}

/// Reads a CityJSONSeq one line at a time. Blank lines are skipped; an error
/// says which line it is on.
pub struct SeqReader<R> {
    lines: std::io::Lines<R>,
    line: usize,
}

impl<R: BufRead> SeqReader<R> {
    pub fn new(reader: R) -> Self {
        SeqReader {
            lines: reader.lines(),
            line: 0,
        }
    }
}

impl<R: BufRead> Iterator for SeqReader<R> {
    type Item = Result<Line>;
    fn next(&mut self) -> Option<Result<Line>> {
        loop {
            let text = self.lines.next()?;
            self.line += 1;
            let line = self.line;
            let at = |e: CjseqError| CjseqError::AtLine {
                line,
                source: Box::new(e),
            };
            match text {
                Err(e) => return Some(Err(at(e.into()))),
                Ok(t) if t.trim().is_empty() => continue,
                Ok(t) => return Some(t.parse().map_err(at)),
            }
        }
    }
}

/// Writes a CityJSONSeq: each header or feature as one line of compact JSON.
pub struct SeqWriter<W> {
    out: W,
}

impl<W: Write> SeqWriter<W> {
    pub fn new(out: W) -> Self {
        SeqWriter { out }
    }

    pub fn write_header(&mut self, header: &CityJSON) -> Result<()> {
        self.write(header)
    }

    pub fn write_feature(&mut self, feature: &CityJSONFeature) -> Result<()> {
        self.write(feature)
    }

    fn write<T: serde::Serialize>(&mut self, value: &T) -> Result<()> {
        serde_json::to_writer(&mut self.out, value)?;
        self.out.write_all(b"\n")?;
        Ok(())
    }

    /// The underlying writer, eg to flush it.
    pub fn into_inner(self) -> W {
        self.out
    }
}
