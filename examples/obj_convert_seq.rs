use cjseq2::conv::obj;
use std::path::Path;

fn main() -> std::io::Result<()> {
    // Path to the sample CityJSONSeq file
    let file_path = Path::new("data/3dbag_b2.city.jsonl");

    // Ensure the file exists
    if !file_path.exists() {
        println!("Sample file not found: {:?}", file_path);
        println!("Available files in data directory:");
        for entry in std::fs::read_dir("data")? {
            let entry = entry?;
            println!("  {:?}", entry.path());
        }
        return Ok(());
    }

    println!("Converting {} to OBJ format...", file_path.display());

    // Output file path
    let output_path = "output_seq.obj";

    // Convert to OBJ and save to file
    obj::jsonseq_file_to_obj(file_path, output_path)?;

    println!("Conversion complete. OBJ file saved to: {}", output_path);

    // Print some stats about the OBJ file
    let metadata = std::fs::metadata(output_path)?;
    println!("OBJ file size: {} bytes", metadata.len());

    // Count number of vertices and faces in the OBJ file
    let obj_contents = std::fs::read_to_string(output_path)?;
    let vertex_count = obj_contents
        .lines()
        .filter(|line| line.starts_with("v "))
        .count();
    let face_count = obj_contents
        .lines()
        .filter(|line| line.starts_with("f "))
        .count();

    println!("OBJ statistics:");
    println!("  Vertices: {}", vertex_count);
    println!("  Faces: {}", face_count);

    Ok(())
}
