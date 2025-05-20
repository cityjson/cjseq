use crate::{CityJSON, CityJSONFeature, Float, Geometry};
use std::fs::File;
use std::io::{Result as IoResult, Write};
use std::path::Path;

/// Converts a CityJSON object to OBJ format and writes to a string.
///
/// # Arguments
///
/// * `city_json` - The CityJSON object to convert.
///
/// # Returns
///
/// A string containing the OBJ data.
pub fn to_obj_string(city_json: &CityJSON) -> String {
    let mut output = Vec::new();
    to_obj(city_json, &mut output).unwrap();
    String::from_utf8(output).unwrap()
}

/// Writes a CityJSON object as OBJ format to a file.
///
/// # Arguments
///
/// * `city_json` - The CityJSON object to convert.
/// * `path` - The output file path.
///
/// # Returns
///
/// An IoResult indicating success or failure.
pub fn to_obj_file(city_json: &CityJSON, path: impl AsRef<Path>) -> IoResult<()> {
    let mut file = File::create(path)?;
    to_obj(city_json, &mut file)
}

/// Converts a CityJSON object to OBJ format.
///
/// # Arguments
///
/// * `city_json` - The CityJSON object to convert.
/// * `writer` - The writer to output OBJ format to.
///
/// # Returns
///
/// A result indicating success or an I/O error.
pub fn to_obj<W: Write>(city_json: &CityJSON, writer: &mut W) -> IoResult<()> {
    // OBJ files start with comments describing the file
    writeln!(writer, "# Converted from CityJSON to OBJ")?;
    writeln!(writer, "# by CJSeq converter")?;
    writeln!(writer)?;

    // Process all vertices first
    // Apply transform to convert integer coordinates to real world coordinates
    let scale = &city_json.transform.scale;
    let translate = &city_json.transform.translate;

    for vertex in &city_json.vertices {
        let x = (vertex[0] as Float * scale[0]) + translate[0];
        let y = (vertex[1] as Float * scale[1]) + translate[1];
        let z = (vertex[2] as Float * scale[2]) + translate[2];

        writeln!(writer, "v {} {} {}", x, y, z)?;
    }

    writeln!(writer)?;

    // Process all CityObjects and their geometries
    for (_id, city_object) in &city_json.city_objects {
        if let Some(geometries) = &city_object.geometry {
            // Find highest LOD geometry
            let highest_lod_geometry = find_highest_lod_geometry(geometries);

            // Process geometry boundaries
            for geometry in highest_lod_geometry {
                convert_geometry_to_obj(&geometry.boundaries, writer)?;
            }
        }
    }

    Ok(())
}

/// Convert a CityJSONSeq file to a CityJSON object and then to OBJ
///
/// # Arguments
///
/// * `path` - The path to the CityJSONSeq file
/// * `output_path` - The path to write the OBJ file
///
/// # Returns
///
/// An IoResult indicating success or failure
pub fn jsonseq_file_to_obj(path: impl AsRef<Path>, output_path: impl AsRef<Path>) -> IoResult<()> {
    use std::io::{BufRead, BufReader};

    let f = File::open(path)?;
    let br = BufReader::new(f);
    let mut cjj = CityJSON::new();

    // Process file similar to collect_from_file in main.rs
    for (i, line) in br.lines().enumerate() {
        let l = line?;
        if i == 0 {
            cjj = CityJSON::from_str(&l)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        } else {
            let mut cjf = CityJSONFeature::from_str(&l)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            cjj.add_cjfeature(&mut cjf);
        }
    }

    // Process like collect_from_file
    cjj.remove_duplicate_vertices();
    cjj.update_transform();

    // Write to OBJ
    to_obj_file(&cjj, output_path)
}

/// Finds geometries with the highest LOD value.
///
/// # Arguments
///
/// * `geometries` - A vector of geometries to search through.
///
/// # Returns
///
/// A vector of references to the geometries with the highest LOD.
fn find_highest_lod_geometry(geometries: &[Geometry]) -> Vec<&Geometry> {
    // Extract LOD values and find the maximum
    let mut max_lod: Option<Float> = None;

    for geometry in geometries {
        if let Some(lod_str) = &geometry.lod {
            if let Ok(lod) = lod_str.parse::<Float>() {
                max_lod = Some(max_lod.map_or(lod, |max| max.max(lod)));
            }
        }
    }

    // If no valid LOD is found, return all geometries
    if max_lod.is_none() {
        return geometries.iter().collect();
    }

    // Filter geometries with the highest LOD
    let max_lod_value = max_lod.unwrap();
    geometries
        .iter()
        .filter(|g| {
            if let Some(lod_str) = &g.lod {
                if let Ok(lod) = lod_str.parse::<Float>() {
                    return (lod - max_lod_value).abs() < Float::EPSILON;
                }
            }
            false
        })
        .collect()
}

/// Converts geometry boundaries to OBJ faces.
///
/// # Arguments
///
/// * `boundaries` - The boundaries to convert.
/// * `writer` - The writer to output OBJ format to.
///
/// # Returns
///
/// A result indicating success or an I/O error.
fn convert_geometry_to_obj<W: Write>(
    boundaries: &crate::Boundaries,
    writer: &mut W,
) -> IoResult<()> {
    match boundaries {
        crate::Boundaries::Indices(indices) => {
            // For a simple list of indices, assume it's a face
            write_obj_face(indices, writer)?;
        }
        crate::Boundaries::Nested(nested) => {
            // Process each nested boundary
            for boundary in nested {
                convert_geometry_to_obj(boundary, writer)?;
            }
        }
    }

    Ok(())
}

/// Writes a single OBJ face from indices.
///
/// # Arguments
///
/// * `indices` - The vertex indices for the face.
/// * `writer` - The writer to output OBJ format to.
///
/// # Returns
///
/// A result indicating success or an I/O error.
fn write_obj_face<W: Write>(indices: &[u32], writer: &mut W) -> IoResult<()> {
    if indices.is_empty() {
        return Ok(());
    }

    // OBJ format uses 1-based indices, while CityJSON uses 0-based
    write!(writer, "f")?;
    for idx in indices {
        write!(writer, " {}", idx + 1)?; // +1 to convert from 0-based to 1-based
    }
    writeln!(writer)?;

    Ok(())
}
