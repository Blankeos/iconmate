use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_add_command_creates_folder_and_files() {
    // Create a temporary directory for testing
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_folder = temp_dir.path().join("src/assets/icons");

    // Build the iconmate binary
    let binary_path = env!("CARGO_BIN_EXE_iconmate");

    // Run the iconmate add command
    let output = Command::new(binary_path)
        .args(&[
            "add",
            "--folder",
            test_folder.to_str().unwrap(),
            "--icon",
            "heroicons:heart",
            "--name",
            "Heart",
        ])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute command");

    // Check that the command executed successfully
    assert!(
        output.status.success(),
        "Command failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify the folder was created
    assert!(test_folder.exists(), "Folder should be created");

    // Verify the files exist
    let index_file = test_folder.join("index.ts");
    let svg_file = test_folder.join("heroicons:heart.svg");

    assert!(index_file.exists(), "index.ts should be created");
    assert!(svg_file.exists(), "heroicons:heart.svg should be created");

    // Verify the content of index.ts
    let index_content = std::fs::read_to_string(&index_file).expect("Failed to read index.ts");

    assert!(
        index_content.contains("export { default as IconHeart } from './heroicons:heart.svg';"),
        "index.ts should contain the correct export statement"
    );

    // Verify the SVG file is not empty and contains SVG content
    let svg_content = std::fs::read_to_string(&svg_file).expect("Failed to read SVG file");

    assert!(!svg_content.is_empty(), "SVG file should not be empty");
    assert!(
        svg_content.contains("<svg"),
        "SVG file should contain SVG tag"
    );
}

#[test]
fn test_add_command_with_existing_folder() {
    // Create a temporary directory and pre-create the target folder
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_folder = temp_dir.path().join("src/assets/icons");

    // Pre-create the folder
    std::fs::create_dir_all(&test_folder).expect("Failed to create test folder");

    // Create an existing index.ts file
    let existing_index = test_folder.join("index.ts");
    std::fs::write(
        &existing_index,
        "export { default as IconExisting } from './existing.svg';\n",
    )
    .expect("Failed to create existing index.ts");

    let binary_path = env!("CARGO_BIN_EXE_iconmate");

    // Run the iconmate add command
    let output = Command::new(binary_path)
        .args(&[
            "add",
            "--folder",
            test_folder.to_str().unwrap(),
            "--icon",
            "heroicons:heart",
            "--name",
            "Heart",
        ])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute command");

    // Check that the command executed successfully
    assert!(
        output.status.success(),
        "Command failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify both old and new exports exist
    let index_content = std::fs::read_to_string(&existing_index).expect("Failed to read index.ts");

    assert!(
        index_content.contains("export { default as IconExisting } from './existing.svg';"),
        "index.ts should still contain the existing export"
    );
    assert!(
        index_content.contains("export { default as IconHeart } from './heroicons:heart.svg';"),
        "index.ts should contain the new export"
    );
}

#[test]
fn test_add_command_duplicate_icon() {
    // Test adding the same icon twice
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_folder = temp_dir.path().join("src/assets/icons");

    let binary_path = env!("CARGO_BIN_EXE_iconmate");

    // First add
    let output1 = Command::new(binary_path)
        .args(&[
            "add",
            "--folder",
            test_folder.to_str().unwrap(),
            "--icon",
            "heroicons:heart",
            "--name",
            "Heart",
        ])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute first command");

    assert!(output1.status.success(), "First command should succeed");

    // Second add of the same icon
    let output2 = Command::new(binary_path)
        .args(&[
            "add",
            "--folder",
            test_folder.to_str().unwrap(),
            "--icon",
            "heroicons:heart",
            "--name",
            "Heart",
        ])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute second command");

    assert!(output2.status.success(), "Second command should succeed");

    // Verify only one export exists (no duplicates)
    let index_file = test_folder.join("index.ts");
    let index_content = std::fs::read_to_string(&index_file).expect("Failed to read index.ts");

    let heart_exports = index_content
        .matches("export { default as IconHeart }")
        .count();
    assert_eq!(heart_exports, 1, "Should only have one Heart export");
}

#[tokio::test]
async fn test_add_command_invalid_icon() {
    // Test with an icon that likely doesn't exist
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_folder = temp_dir.path().join("src/assets/icons");

    let binary_path = env!("CARGO_BIN_EXE_iconmate");

    let output = Command::new(binary_path)
        .args(&[
            "add",
            "--folder",
            test_folder.to_str().unwrap(),
            "--icon",
            "nonexistent:icon",
            "--name",
            "NonExistent",
        ])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute command");

    // This should fail since the icon doesn't exist
    assert!(
        !output.status.success(),
        "Command should fail for non-existent icon"
    );
}

#[test]
fn test_add_command_preset_empty_svg() {
    // Test using the preset instead of fetching an icon
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_folder = temp_dir.path().join("src/assets/icons");

    let binary_path = env!("CARGO_BIN_EXE_iconmate");

    let output = Command::new(binary_path)
        .args(&[
            "add",
            "--folder",
            test_folder.to_str().unwrap(),
            "--preset",
            "emptysvg",
            "--name",
            "EmptyIcon",
        ])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify the files exist
    let index_file = test_folder.join("index.ts");
    let svg_file = test_folder.join("EmptyIcon.svg");

    assert!(index_file.exists(), "index.ts should be created");
    assert!(svg_file.exists(), "EmptyIcon.svg should be created");

    // Verify the SVG content is the empty SVG template
    let svg_content = std::fs::read_to_string(&svg_file).expect("Failed to read SVG file");

    assert!(
        svg_content.contains(r#"svg xmlns="http://www.w3.org/2000/svg""#),
        "SVG should contain the empty SVG template"
    );
}

#[test]
fn test_add_command_preset_normal_with_icon() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_folder = temp_dir.path().join("src/assets/icons");

    let binary_path = env!("CARGO_BIN_EXE_iconmate");

    let output = Command::new(binary_path)
        .args(&[
            "add",
            "--folder",
            test_folder.to_str().unwrap(),
            "--preset",
            "normal",
            "--icon",
            "heroicons:heart",
            "--name",
            "Heart",
        ])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let svg_file = test_folder.join("heroicons:heart.svg");
    assert!(svg_file.exists(), "heroicons:heart.svg should be created");
}

#[test]
fn test_add_command_preset_normal_requires_icon() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_folder = temp_dir.path().join("src/assets/icons");

    let binary_path = env!("CARGO_BIN_EXE_iconmate");

    let output = Command::new(binary_path)
        .args(&[
            "add",
            "--folder",
            test_folder.to_str().unwrap(),
            "--preset",
            "normal",
            "--name",
            "NoIcon",
        ])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success(), "Command should fail without icon");
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("--icon argument is required when --preset is normal"),
        "stderr should explain normal preset needs an icon"
    );
}
