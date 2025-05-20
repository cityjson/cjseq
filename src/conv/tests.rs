#[cfg(test)]
mod tests {
    use super::super::obj;
    use crate::{Boundaries, CityJSON, CityObject, Geometry, GeometryType, Transform};

    #[test]
    fn test_to_obj_simple() {
        // Create a simple CityJSON object with a single cube
        let mut city_json = CityJSON::new();

        // Set transform
        city_json.transform = Transform {
            scale: vec![1.0, 1.0, 1.0],
            translate: vec![0.0, 0.0, 0.0],
        };

        // Add vertices for a cube (8 vertices)
        city_json.vertices = vec![
            vec![0, 0, 0], // 0: bottom front left
            vec![1, 0, 0], // 1: bottom front right
            vec![1, 1, 0], // 2: bottom back right
            vec![0, 1, 0], // 3: bottom back left
            vec![0, 0, 1], // 4: top front left
            vec![1, 0, 1], // 5: top front right
            vec![1, 1, 1], // 6: top back right
            vec![0, 1, 1], // 7: top back left
        ];

        // Create a solid with 6 faces (a cube)
        let boundaries = Boundaries::Nested(vec![
            // Exterior shell with 6 faces
            Boundaries::Nested(vec![
                // Bottom face
                Boundaries::Nested(vec![Boundaries::Indices(vec![0, 1, 2, 3])]),
                // Front face
                Boundaries::Nested(vec![Boundaries::Indices(vec![0, 1, 5, 4])]),
                // Right face
                Boundaries::Nested(vec![Boundaries::Indices(vec![1, 2, 6, 5])]),
                // Back face
                Boundaries::Nested(vec![Boundaries::Indices(vec![2, 3, 7, 6])]),
                // Left face
                Boundaries::Nested(vec![Boundaries::Indices(vec![3, 0, 4, 7])]),
                // Top face
                Boundaries::Nested(vec![Boundaries::Indices(vec![4, 5, 6, 7])]),
            ]),
        ]);

        // Create geometry
        let geometry = Geometry {
            thetype: GeometryType::Solid,
            lod: Some("2.0".to_string()),
            boundaries,
            semantics: None,
            material: None,
            texture: None,
            template: None,
            transformation_matrix: None,
        };

        // Create city object
        let city_object = CityObject::new(
            "Building".to_string(),
            None,
            None,
            Some(vec![geometry]),
            None,
            None,
            None,
            None,
        );

        // Add city object to city_json
        city_json
            .city_objects
            .insert("Building1".to_string(), city_object);

        // Convert to OBJ and check the result
        let obj_string = obj::to_obj_string(&city_json);

        // Print the entire output for debugging
        println!("Generated OBJ:\n{}", obj_string);

        // Check for expected OBJ content - vertex coordinates
        assert!(obj_string.contains("v 0 0 0"));
        assert!(obj_string.contains("v 1 0 0"));
        assert!(obj_string.contains("v 1 1 0"));
        assert!(obj_string.contains("v 0 1 0"));
        assert!(obj_string.contains("v 0 0 1"));
        assert!(obj_string.contains("v 1 0 1"));
        assert!(obj_string.contains("v 1 1 1"));
        assert!(obj_string.contains("v 0 1 1"));

        // Check face declarations (1-indexed)
        assert!(obj_string.contains("f 1 2 3 4"));
        assert!(obj_string.contains("f 1 2 6 5"));
        assert!(obj_string.contains("f 2 3 7 6"));
        assert!(obj_string.contains("f 3 4 8 7"));
        assert!(obj_string.contains("f 4 1 5 8"));
        assert!(obj_string.contains("f 5 6 7 8"));
    }
}
