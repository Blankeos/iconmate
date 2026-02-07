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
        .args([
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
        .args([
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
fn test_add_command_appends_after_non_newline_terminated_index() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_folder = temp_dir.path().join("src/assets/icons");

    std::fs::create_dir_all(&test_folder).expect("Failed to create test folder");

    let existing_index = test_folder.join("index.ts");
    std::fs::write(
        &existing_index,
        "export { default as IconExisting } from './existing.svg';",
    )
    .expect("Failed to create existing index.ts");

    let binary_path = env!("CARGO_BIN_EXE_iconmate");

    let output = Command::new(binary_path)
        .args([
            "add",
            "--folder",
            test_folder.to_str().unwrap(),
            "--preset",
            "emptysvg",
            "--name",
            "Fresh",
        ])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let index_content = std::fs::read_to_string(&existing_index).expect("Failed to read index.ts");

    assert!(
        index_content.contains("export { default as IconExisting } from './existing.svg';\n"),
        "existing export should end with a newline before appending"
    );
    assert!(
        index_content.contains("export { default as IconFresh } from './fresh.svg';"),
        "index.ts should contain the new export"
    );
    assert!(
        !index_content.contains("existing.svg';export"),
        "new export should not be concatenated on the same line"
    );
}

#[test]
fn test_add_command_rejects_duplicate_icon() {
    // Test adding the same icon twice now fails on conflict
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_folder = temp_dir.path().join("src/assets/icons");

    let binary_path = env!("CARGO_BIN_EXE_iconmate");

    // First add
    let output1 = Command::new(binary_path)
        .args([
            "add",
            "--folder",
            test_folder.to_str().unwrap(),
            "--preset",
            "emptysvg",
            "--name",
            "Heart",
            "--filename",
            "heart",
        ])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute first command");

    assert!(output1.status.success(), "First command should succeed");

    // Second add of the same icon
    let output2 = Command::new(binary_path)
        .args([
            "add",
            "--folder",
            test_folder.to_str().unwrap(),
            "--preset",
            "emptysvg",
            "--name",
            "Heart",
            "--filename",
            "heart",
        ])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute second command");

    assert!(!output2.status.success(), "Second command should fail");
    assert!(
        String::from_utf8_lossy(&output2.stderr).contains("already exists"),
        "stderr should explain export/file conflict"
    );

    // Verify only one export exists
    let index_file = test_folder.join("index.ts");
    let index_content = std::fs::read_to_string(&index_file).expect("Failed to read index.ts");

    let heart_exports = index_content
        .matches("export { default as IconHeart }")
        .count();
    assert_eq!(heart_exports, 1, "Should only have one Heart export");
}

#[test]
fn test_add_command_rejects_duplicate_name_with_different_target() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_folder = temp_dir.path().join("src/assets/icons");

    let binary_path = env!("CARGO_BIN_EXE_iconmate");

    let first = Command::new(binary_path)
        .args([
            "add",
            "--folder",
            test_folder.to_str().unwrap(),
            "--preset",
            "emptysvg",
            "--name",
            "Heart",
            "--filename",
            "heart-a",
        ])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute first command");
    assert!(first.status.success(), "First command should succeed");

    let second = Command::new(binary_path)
        .args([
            "add",
            "--folder",
            test_folder.to_str().unwrap(),
            "--preset",
            "emptysvg",
            "--name",
            "Heart",
            "--filename",
            "heart-b",
        ])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute second command");

    assert!(
        !second.status.success(),
        "Second command should fail on duplicate export alias"
    );
    assert!(
        String::from_utf8_lossy(&second.stderr).contains("Icon alias"),
        "stderr should mention duplicate alias"
    );

    assert!(!test_folder.join("heart-b.svg").exists());
}

#[test]
fn test_add_command_rejects_duplicate_target_with_different_name() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_folder = temp_dir.path().join("src/assets/icons");

    let binary_path = env!("CARGO_BIN_EXE_iconmate");

    let first = Command::new(binary_path)
        .args([
            "add",
            "--folder",
            test_folder.to_str().unwrap(),
            "--preset",
            "emptysvg",
            "--name",
            "Heart",
            "--filename",
            "shared-target",
        ])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute first command");
    assert!(first.status.success(), "First command should succeed");

    let second = Command::new(binary_path)
        .args([
            "add",
            "--folder",
            test_folder.to_str().unwrap(),
            "--preset",
            "emptysvg",
            "--name",
            "Star",
            "--filename",
            "shared-target",
        ])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute second command");

    assert!(
        !second.status.success(),
        "Second command should fail on duplicate export target"
    );
    assert!(
        String::from_utf8_lossy(&second.stderr).contains("Export target"),
        "stderr should mention duplicate export target"
    );

    let index_file = test_folder.join("index.ts");
    let index_content = std::fs::read_to_string(&index_file).expect("Failed to read index.ts");
    assert!(index_content.contains("IconHeart"));
    assert!(!index_content.contains("IconStar"));
}

#[tokio::test]
async fn test_add_command_invalid_icon() {
    // Test with an icon that likely doesn't exist
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_folder = temp_dir.path().join("src/assets/icons");

    let binary_path = env!("CARGO_BIN_EXE_iconmate");

    let output = Command::new(binary_path)
        .args([
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
        .args([
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
        .args([
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
        .args([
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

#[test]
fn test_list_command_prints_existing_icons() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_folder = temp_dir.path().join("src/assets/icons");
    std::fs::create_dir_all(&test_folder).expect("Failed to create icons folder");

    let index_file = test_folder.join("index.ts");
    std::fs::write(
        &index_file,
        "export { default as IconHeart } from './heart.svg';\nexport { default as IconStar } from './star.svg';\n",
    )
    .expect("Failed to write index.ts");

    let binary_path = env!("CARGO_BIN_EXE_iconmate");
    let output = Command::new(binary_path)
        .args(["list", "--folder", test_folder.to_str().unwrap()])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("IconHeart\t./heart.svg"),
        "stdout should include IconHeart row"
    );
    assert!(
        stdout.contains("IconStar\t./star.svg"),
        "stdout should include IconStar row"
    );
}

#[test]
fn test_list_command_uses_default_folder_when_no_flag_is_passed() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let default_folder = temp_dir.path().join("src/assets/icons");
    std::fs::create_dir_all(&default_folder).expect("Failed to create icons folder");

    let index_file = default_folder.join("index.ts");
    std::fs::write(
        &index_file,
        "export { default as IconHouse } from './house.svg';\n",
    )
    .expect("Failed to write index.ts");

    let binary_path = env!("CARGO_BIN_EXE_iconmate");
    let output = Command::new(binary_path)
        .args(["list"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("IconHouse\t./house.svg"),
        "stdout should include icon from default folder"
    );
}

#[test]
fn test_list_command_reports_no_icons_when_index_is_missing() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_folder = temp_dir.path().join("src/assets/icons");
    std::fs::create_dir_all(&test_folder).expect("Failed to create icons folder");

    let binary_path = env!("CARGO_BIN_EXE_iconmate");
    let output = Command::new(binary_path)
        .args(["list", "--folder", test_folder.to_str().unwrap()])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "Command failed with stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No icons found in"),
        "stdout should explain that no icons were found"
    );
}
