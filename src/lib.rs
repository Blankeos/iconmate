// Core functionality that can be tested
use anyhow::Result;
use std::fs;
use std::path::Path;

/// Utilities for validating icon operations
pub mod validation {
    use super::*;

    /// Verifies that the expected files were created by the add command
    pub fn verify_files_created(
        folder_path: &Path,
        icon_name: &str,
        file_stem: &str,
    ) -> Result<()> {
        // Verify the folder exists
        assert!(
            folder_path.exists(),
            "Folder should be created at {:?}",
            folder_path
        );

        // Verify the files exist
        let index_file = folder_path.join("index.ts");
        let svg_file = folder_path.join(format!("{}.svg", file_stem));

        assert!(index_file.exists(), "index.ts should be created");
        assert!(svg_file.exists(), "{}.svg should be created", file_stem);

        // Verify the content of index.ts
        let index_content = fs::read_to_string(&index_file)?;
        let expected_export = format!(
            "export {{ default as Icon{} }} from './{}.svg';",
            icon_name, file_stem
        );
        if !index_content.contains(&expected_export) {
            panic!(
                "index.ts should contain the correct export statement: {}\nActual content:\n{}",
                expected_export, index_content
            );
        }

        // Verify the SVG file is not empty and contains SVG content
        let svg_content = fs::read_to_string(&svg_file)?;
        assert!(!svg_content.is_empty(), "SVG file should not be empty");
        assert!(
            svg_content.contains("<svg"),
            "SVG file should contain SVG tag"
        );

        Ok(())
    }

    /// Verifies the content of the index.ts file
    pub fn verify_index_content(index_path: &Path, expected_exports: &[&str]) -> Result<()> {
        let content = fs::read_to_string(index_path)?;

        for expected_export in expected_exports {
            if !content.contains(expected_export) {
                panic!(
                    "index.ts should contain export: {}\nActual content:\n{}",
                    expected_export, content
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_validation_functions() {
        // This test verifies our validation functions work correctly
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let test_folder = temp_dir.path().join("src/assets/icons");

        // Create the test folder
        fs::create_dir_all(&test_folder).expect("Failed to create test folder");

        // Create some test files
        let svg_path = test_folder.join("test.svg");
        let index_path = test_folder.join("index.ts");

        fs::write(&svg_path, "<svg>test</svg>").expect("Failed to write SVG");
        fs::write(
            &index_path,
            "export { default as IconTest } from './test.svg';",
        )
        .expect("Failed to write index.ts");

        // Test our validation functions
        validation::verify_files_created(&test_folder, "Test", "test")
            .expect("Files should be verified");

        validation::verify_index_content(
            &index_path,
            &["export { default as IconTest } from './test.svg';"],
        )
        .expect("Index content should be verified");
    }
}
