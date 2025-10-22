use clap::ValueEnum;
use ratatui::layout::{Constraint, Rect};
use reqwest::Url;

#[derive(ValueEnum, Clone, Debug, PartialEq, Hash)]
pub enum Preset {
    /// Use a blank SVG.
    #[value(name = "emptysvg")]
    Svg,

    /// React Component .tsx
    #[value(name = "react")]
    React,

    /// Svelte Component .svelte
    #[value(name = "svelte")]
    Svelte,

    /// Solid Component .tsx
    #[value(name = "solid")]
    Solid,

    /// Vue
    #[value(name = "vue")]
    Vue,
}

impl Preset {
    pub fn to_str(&self) -> &'static str {
        match self {
            Preset::Svg => "emptysvg",
            Preset::React => "react",
            Preset::Svelte => "svelte",
            Preset::Solid => "solid",
            Preset::Vue => "vue",
        }
    }
}

/// A helper struct that pairs a preset with its human-readable description
#[derive(Debug, Clone)]
pub struct PresetOption {
    pub preset: Preset,
    pub description: &'static str,
}

pub const PRESETS_OPTIONS: &[PresetOption] = &[
    PresetOption {
        preset: Preset::Svg,
        description: "Outputs an svg (.svg)",
    },
    PresetOption {
        preset: Preset::React,
        description: "Outputs a React component (.tsx)",
    },
    PresetOption {
        preset: Preset::Svelte,
        description: "Outputs a Svelte component (.svelte)",
    },
    PresetOption {
        preset: Preset::Solid,
        description: "Outputs a SolidJS component (.tsx)",
    },
    PresetOption {
        preset: Preset::Vue,
        description: "Outputs a Vue component (.vue)",
    },
];

/// helper function to create a centered rect using up certain maximum dimensions `r`
pub fn popup_area(area: Rect, max_width: u16, max_height: u16) -> Rect {
    let width = max_width.min(area.width);
    let height = max_height.min(area.height);
    let horizontal_margin = area.width.saturating_sub(width) / 2;
    let vertical_margin = area.height.saturating_sub(height) / 2;
    Rect {
        x: area.x + horizontal_margin,
        y: area.y + vertical_margin,
        width,
        height,
    }
}

/// Struct to hold icon information for deletion
#[derive(Debug, Clone)]
pub struct IconEntry {
    pub name: String,
    pub file_path: String,
}

/// Enum representing the type of icon source
#[derive(Debug, PartialEq)]
pub enum IconSourceType {
    /// A plain iconify name (e.g., "stash:chevron")
    IconifyName,
    /// A full HTTP/HTTPS URL
    Url,
    /// Raw SVG content
    SvgContent,
    /// None provided
    None,
}

/// Util: Determines the type of icon source
pub fn _determine_icon_source_type(icon_source: Option<&String>) -> IconSourceType {
    match icon_source {
        Some(icon) => {
            if icon.trim_start().starts_with("<svg") {
                IconSourceType::SvgContent
            } else if icon.starts_with("http://") || icon.starts_with("https://") {
                IconSourceType::Url
            } else {
                IconSourceType::IconifyName
            }
        }
        None => IconSourceType::None,
    }
}

/// Util: Converts any icon_source into an SVG
pub async fn _icon_source_to_svg(
    icon_source: &Option<String>,
    append_attribute: Option<&'static str>,
    remove_comments: bool,
) -> anyhow::Result<String> {
    // If icon_source is missing, return a minimal SVG (Note: rust skill issue idk how else to just reuse the last clause in the match below)
    let Some(icon_source) = icon_source else {
        return Ok(r#"<svg></svg>"#.to_string());
    };

    let mut content = match _determine_icon_source_type(Some(icon_source)) {
        IconSourceType::SvgContent => {
            // Already an SVG document
            icon_source.clone()
        }
        IconSourceType::IconifyName => {
            // Construct the full URL for the icon
            let icon_url = Url::parse(&format!("https://api.iconify.design/{}.svg", icon_source))?;
            println!("Fetching icon from: {}", icon_url);

            // Fetch the SVG content
            let client = reqwest::Client::new();
            let response = client.get(icon_url).send().await?.error_for_status()?;
            response.text().await?
        }
        IconSourceType::Url => {
            // Already a full URL
            let icon_url = Url::parse(icon_source)?;
            println!("Fetching icon from: {}", icon_url);

            // Fetch the SVG content
            let client = reqwest::Client::new();
            let response = client.get(icon_url).send().await?.error_for_status()?;
            response.text().await?
        }
        IconSourceType::None => {
            return Ok(r#"<svg></svg>"#.to_string());
        }
    };

    // -- Transformations if applicable ---

    // 1. Append attribute (i.e. for jsx,svelte,vue)
    if let Some(attr) = append_attribute {
        // Find the first occurrence of "<svg" and append the attribute right before the closing ">"
        if let Some(svg_start) = content.find("<svg") {
            if let Some(svg_tag_end) = content[svg_start..].find('>') {
                let insert_pos = svg_start + svg_tag_end;
                let before = &content[..insert_pos];
                let after = &content[insert_pos..];
                content = format!("{} {}{}", before, attr, after);
            }
        }
    }

    // 2. Remove Comments
    if remove_comments {
        // Remove HTML comments from SVG content
        // This regex matches <!-- ... --> including any content in between
        let re = regex::Regex::new(r"<!--.*?-->").unwrap();
        content = re.replace_all(&content, "").to_string();
    }

    Ok(content)
}

/// Util: Reused in all cases, for appending the filename of svg, i.e. add .tsx or .svg or .svelte.
/// Returns a file_stem and an ext
pub fn _make_svg_filename(
    stem_from_cli: Option<&String>,
    ext: &'static str,
    icon_source: Option<&String>,
    name_from_cli: &str,
) -> (String, &'static str) {
    let stem = if let Some(stem) = stem_from_cli {
        stem.clone()
    } else if let Some(icon) = icon_source {
        // Only use icon_source if it's a plain iconify name (no http/https, no <svg)
        match _determine_icon_source_type(icon_source) {
            IconSourceType::IconifyName => icon.clone(),
            _ => name_from_cli.to_string().to_lowercase(),
        }
    } else {
        name_from_cli.to_string().to_lowercase()
    };

    if stem.ends_with(ext) {
        (stem.replace(ext, ""), ext)
    } else {
        (stem, ext)
    }
}

// Util for tui view in add.
pub fn filename_from_preset(file_name: Option<String>, preset: Option<Preset>) -> String {
    if let Some(preset) = preset {
        let ext = match preset {
            Preset::Svg => "svg",
            Preset::React => "tsx",
            Preset::Svelte => "svelte",
            Preset::Solid => "tsx",
            Preset::Vue => "vue",
        };

        if let Some(name) = file_name {
            if name.contains('.') {
                return name;
            } else {
                return format!("{}.{}", name, ext);
            }
        } else {
            return format!("component.{}", ext);
        }
    }

    if let Some(name) = file_name {
        return name;
    }

    "".to_string()
}

/// Util: Reads a file line-by-line and extracts every icon entry that matches
/// the template used by the current project.
/// Returns a vector of `IconEntry` with the name and absolute file path.
pub fn get_existing_icons(folder_path: &str) -> anyhow::Result<Vec<IconEntry>> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use std::path::Path;

    let index_path = Path::new(folder_path).join("index.ts"); // FUTURE: for flutter suport, make sure to configure this + the parsing of it.

    let file = File::open(&index_path)?;

    let reader = BufReader::new(file);

    let mut icons = Vec::new();

    for line in reader.lines() {
        let line = line?;
        // Skip empty lines and comments
        if line.trim().is_empty() || line.trim_start().starts_with("//") {
            continue;
        }

        if let Some(icon_entry) = parse_export_line_ts(&line) {
            icons.push(icon_entry);
        }
    }

    Ok(icons)
}

/// For parsing a single export line in typescript.
pub fn parse_export_line_ts(line: &str) -> Option<IconEntry> {
    use std::path::Path;

    // Log the line
    let line = line.trim();

    // Skip empty lines and comments
    if line.is_empty() || line.trim_start().starts_with("//") {
        return None;
    }

    // Example line:   export { default as IconGitHub } from "./devicon:github.svg";
    // We look for:    export { default as <Name> } from "<file_path>";
    let parts: Vec<&str> = line.splitn(5, ' ').collect();
    if parts.len() >= 5
        && parts[0] == "export"
        && parts[1] == "{"
        && parts[3] == "as"
        && parts[2] == "default"
    {
        if let Some(name_end) = line.find('}') {
            let name = line["export { default as ".len()..name_end].trim();
            if let Some(from_start) = line[name_end..].find("from \"") {
                let from_start = name_end + from_start + "from \"".len();
                if let Some(path_end) = line[from_start..].find('"') {
                    let relative_path = &line[from_start..from_start + path_end];
                    let abs_path = Path::new(relative_path)
                        .canonicalize()
                        .unwrap_or_else(|_| Path::new(relative_path).to_path_buf());

                    return Some(IconEntry {
                        name: name.to_string(),
                        file_path: abs_path.to_string_lossy().to_string(),
                    });
                }
            }
        }
    }
    None
}

// FUTURE:
// pub fn _parse_export_line_dart(line: &str) -> Option<IconEntry> {}

/// Deletes an IconEntry based on its file path
pub fn delete_icon_entry(file_path: &str) -> anyhow::Result<()> {
    use std::fs;
    use std::path::Path;

    let path = Path::new(file_path);

    // Delete the icon file
    if path.exists() {
        fs::remove_file(path)?;
        println!("Deleted icon file: {}", path.display());
    } else {
        eprintln!("Icon file not found: {}", path.display());
        return Ok(());
    }

    // Find the parent folder and index.ts
    if let Some(parent) = path.parent() {
        let index_path = parent.join("index.ts");

        if index_path.exists() {
            // Read the current index.ts
            let contents = fs::read_to_string(&index_path)?;

            // Generate the file path relative to the parent folder
            let relative_path = if file_path.starts_with(parent.to_string_lossy().as_ref()) {
                &file_path[parent.to_string_lossy().len() + 1..]
            } else {
                file_path
            };

            // Create the normalized relative path for comparison
            let normalized_relative_path = relative_path.replace('\\', "/");
            let file_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

            // Remove all lines that export this file
            let mut lines_to_keep = Vec::new();
            let mut found_export = false;

            for line in contents.lines() {
                // Check if this line exports our file
                if line.contains(&normalized_relative_path)
                    || (line.contains(file_name) && line.contains("export"))
                {
                    found_export = true;
                    continue; // Skip this line (remove it)
                }
                lines_to_keep.push(line);
            }

            if found_export {
                // Write the updated content back
                let updated_content = lines_to_keep.join("\n");
                fs::write(&index_path, updated_content)?;
                println!("Updated index.ts");
            }
        }
    }

    Ok(())
}
