use clap::ValueEnum;
use ratatui::layout::Rect;
use reqwest::Url;

use crate::iconify::IconifyClient;

#[derive(ValueEnum, Clone, Debug, PartialEq, Hash)]
pub enum Preset {
    /// Use the icon source as a regular SVG.
    #[value(name = "normal")]
    Normal,

    /// Use a blank SVG.
    #[value(name = "emptysvg")]
    EmptySvg,

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
            Preset::Normal => "normal",
            Preset::EmptySvg => "emptysvg",
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

impl std::fmt::Display for PresetOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.preset.to_str(), self.description)
    }
}

pub const PRESETS_OPTIONS: &[PresetOption] = &[
    PresetOption {
        preset: Preset::Normal,
        description: "Outputs an svg from an icon source (.svg)",
    },
    PresetOption {
        preset: Preset::EmptySvg,
        description: "Outputs a blank svg placeholder (.svg)",
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

fn decode_icon_candidate(value: &str) -> String {
    value
        .replace("%3A", ":")
        .replace("%3a", ":")
        .replace("%2F", "/")
        .replace("%2f", "/")
}

fn is_iconify_name(value: &str) -> bool {
    let Some((prefix, icon)) = value.split_once(':') else {
        return false;
    };

    if prefix.trim().is_empty() || icon.trim().is_empty() {
        return false;
    }

    !value.chars().any(char::is_whitespace)
}

fn to_pascal_case(input: &str) -> String {
    input
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut built = String::new();
                    built.extend(first.to_uppercase());
                    built.push_str(&chars.as_str().to_ascii_lowercase());
                    built
                }
                None => String::new(),
            }
        })
        .collect::<String>()
}

pub fn iconify_name_from_icon_source(icon_source: &str) -> Option<String> {
    let trimmed = icon_source.trim();
    if trimmed.is_empty() || trimmed.trim_start().starts_with("<svg") {
        return None;
    }

    if !trimmed.contains("://") {
        let normalized = decode_icon_candidate(trimmed);
        if is_iconify_name(&normalized) {
            return Some(normalized);
        }
    }

    let url = Url::parse(trimmed).ok()?;
    let host = url.host_str()?.to_ascii_lowercase();

    if host.contains("icones.js.org") {
        for (key, value) in url.query_pairs() {
            if key == "icon" || key == "i" {
                let candidate = decode_icon_candidate(value.as_ref());
                if is_iconify_name(&candidate) {
                    return Some(candidate);
                }
            }
        }

        let segments: Vec<_> = url.path_segments()?.collect();
        if segments.len() >= 4 && segments[0] == "collection" && segments[2] == "icon" {
            let candidate = format!(
                "{}:{}",
                decode_icon_candidate(segments[1]),
                decode_icon_candidate(segments[3])
            );
            if is_iconify_name(&candidate) {
                return Some(candidate);
            }
        }

        if segments.len() >= 3 && segments[0] == "icon" {
            let candidate = format!(
                "{}:{}",
                decode_icon_candidate(segments[1]),
                decode_icon_candidate(segments[2])
            );
            if is_iconify_name(&candidate) {
                return Some(candidate);
            }
        }
    }

    if host.contains("api.iconify.design") {
        let path = url.path().trim_start_matches('/').trim_end_matches('/');

        if path.ends_with(".json") {
            let prefix = path.trim_end_matches(".json");
            for (key, value) in url.query_pairs() {
                if key == "icons" {
                    let icon = value.split(',').next().unwrap_or("").trim();
                    let candidate = format!(
                        "{}:{}",
                        decode_icon_candidate(prefix),
                        decode_icon_candidate(icon)
                    );
                    if is_iconify_name(&candidate) {
                        return Some(candidate);
                    }
                }
            }
        }

        let without_ext = path.trim_end_matches(".svg");
        let decoded = decode_icon_candidate(without_ext);
        if is_iconify_name(&decoded) {
            return Some(decoded);
        }

        if let Some((prefix, icon)) = decoded.split_once('/') {
            let candidate = format!("{}:{}", prefix, icon);
            if is_iconify_name(&candidate) {
                return Some(candidate);
            }
        }
    }

    None
}

pub fn default_name_and_filename_from_icon_source(icon_source: &str) -> Option<(String, String)> {
    let iconify_name = iconify_name_from_icon_source(icon_source)?;
    let icon_name = iconify_name
        .split_once(':')
        .map(|(_, icon)| icon)
        .unwrap_or(iconify_name.as_str());
    let component_name = to_pascal_case(icon_name);

    if component_name.is_empty() {
        return None;
    }

    Some((component_name, iconify_name))
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
            let client = IconifyClient::from_env()?;
            client.svg(icon_source).await?
        }
        IconSourceType::Url => {
            if let Some(iconify_name) = iconify_name_from_icon_source(icon_source) {
                let client = IconifyClient::from_env()?;
                client.svg(&iconify_name).await?
            } else {
                // Already a full URL
                let icon_url = Url::parse(icon_source)?;
                println!("Fetching icon from: {}", icon_url);

                // Fetch the SVG content
                let client = reqwest::Client::new();
                let response = client.get(icon_url).send().await?.error_for_status()?;
                response.text().await?
            }
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
            IconSourceType::IconifyName => {
                iconify_name_from_icon_source(icon).unwrap_or(icon.clone())
            }
            IconSourceType::Url => iconify_name_from_icon_source(icon)
                .unwrap_or_else(|| name_from_cli.to_string().to_lowercase()),
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
            Preset::Normal => "svg",
            Preset::EmptySvg => "svg",
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
/// Returns a vector of `IconEntry` with the export alias and import file path.
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

        // Parse one or more exports per physical line.
        // This keeps discovery resilient if exports were accidentally concatenated.
        for statement in line.split(';') {
            let statement = statement.trim();
            if statement.is_empty() || statement.starts_with("//") {
                continue;
            }

            if let Some(icon_entry) = parse_export_line_ts(statement) {
                icons.push(icon_entry);
            }
        }
    }

    Ok(icons)
}

/// For parsing a single export line in typescript.
pub fn parse_export_line_ts(line: &str) -> Option<IconEntry> {
    let line = line.trim();

    // Skip empty lines and comments
    if line.is_empty() || line.trim_start().starts_with("//") {
        return None;
    }

    // Example lines:
    // export { default as IconGitHub } from "./devicon:github.svg";
    // export { default as IconGitHub } from './devicon:github.svg';
    if !line.starts_with("export") {
        return None;
    }

    let open_brace_idx = line.find('{')?;
    let close_brace_idx = line[open_brace_idx + 1..].find('}')? + open_brace_idx + 1;

    let inside_braces = line[open_brace_idx + 1..close_brace_idx].trim();
    let mut tokens = inside_braces.split_whitespace();
    if tokens.next()? != "default" || tokens.next()? != "as" {
        return None;
    }

    let name = tokens.next()?.trim_end_matches(',');
    if name.is_empty() {
        return None;
    }

    let after_brace = line[close_brace_idx + 1..].trim_start();
    let after_from = after_brace.strip_prefix("from")?.trim_start();
    let quote_char = after_from.chars().next()?;
    if quote_char != '"' && quote_char != '\'' {
        return None;
    }

    let path_start = quote_char.len_utf8();
    let path_end = after_from[path_start..].find(quote_char)?;
    let relative_path = &after_from[path_start..path_start + path_end];
    let mut import_path_end = relative_path.len();
    if let Some(index) = relative_path.find('?') {
        import_path_end = import_path_end.min(index);
    }
    if let Some(index) = relative_path.find('#') {
        import_path_end = import_path_end.min(index);
    }
    let import_path = relative_path[..import_path_end].trim();
    if import_path.is_empty() {
        return None;
    }

    Some(IconEntry {
        name: name.to_string(),
        file_path: import_path.to_string(),
    })
}

// FUTURE:
// pub fn _parse_export_line_dart(line: &str) -> Option<IconEntry> {}

/// Deletes an IconEntry based on its file path
pub fn delete_icon_entry(file_path: &str) -> anyhow::Result<()> {
    use std::fs;
    use std::path::Path;

    let path = Path::new(file_path);

    // Delete the icon file when present. We still continue to clean index.ts
    // if the file is already missing (stale export entry).
    if path.exists() {
        fs::remove_file(path)?;
    }

    // Find the parent folder and index.ts
    if let Some(parent) = path.parent() {
        let index_path = parent.join("index.ts");

        if index_path.exists() {
            // Read the current index.ts
            let contents = fs::read_to_string(&index_path)?;

            // Generate the file path relative to the parent folder
            let relative_path = path
                .strip_prefix(parent)
                .ok()
                .and_then(|value| value.to_str())
                .unwrap_or(file_path);

            // Create the normalized relative path for comparison
            let normalized_relative_path = normalize_icon_relative_path(relative_path);
            // Remove all lines that export this file
            let mut lines_to_keep = Vec::<String>::new();
            let mut found_export = false;

            for line in contents.lines() {
                let mut parsed_export_in_line = false;

                for statement in line.split(';') {
                    let statement = statement.trim();
                    if statement.is_empty() {
                        continue;
                    }

                    let Some(entry) = parse_export_line_ts(statement) else {
                        continue;
                    };

                    parsed_export_in_line = true;
                    let should_remove =
                        normalize_icon_relative_path(&entry.file_path) == normalized_relative_path;

                    if should_remove {
                        found_export = true;
                        continue;
                    }

                    lines_to_keep.push(format!("{statement};"));
                }

                if !parsed_export_in_line {
                    lines_to_keep.push(line.to_string());
                }
            }

            if found_export {
                // Write the updated content back
                let mut updated_content = lines_to_keep.join("\n");
                if contents.ends_with('\n') {
                    updated_content.push('\n');
                }
                fs::write(&index_path, updated_content)?;
                // println!("Updated index.ts");
            }
        }
    }

    Ok(())
}

fn normalize_icon_relative_path(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_string()
}

fn split_import_path_suffix(value: &str) -> (&str, &str) {
    let query_index = value.find('?');
    let hash_index = value.find('#');

    let split_index = match (query_index, hash_index) {
        (Some(query), Some(hash)) => Some(query.min(hash)),
        (Some(index), None) | (None, Some(index)) => Some(index),
        (None, None) => None,
    };

    match split_index {
        Some(index) => (&value[..index], &value[index..]),
        None => (value, ""),
    }
}

fn replace_import_path_in_export_statement(
    statement: &str,
    current_relative_path: &str,
    new_relative_path: &str,
) -> Option<String> {
    let from_pos = statement.find("from ")?;
    let path_start_search = from_pos + "from ".len();
    let statement_after_from = &statement[path_start_search..];
    let first_quote_offset = statement_after_from.find(['"', '\''])?;
    let first_quote_idx = path_start_search + first_quote_offset;
    let quote_char = statement.as_bytes()[first_quote_idx] as char;
    let path_start_idx = first_quote_idx + 1;
    let second_quote_offset = statement[path_start_idx..].find(quote_char)?;
    let path_end_idx = path_start_idx + second_quote_offset;
    let matched_path = &statement[path_start_idx..path_end_idx];
    let (matched_base_path, matched_suffix) = split_import_path_suffix(matched_path);

    if normalize_icon_relative_path(matched_base_path) != current_relative_path {
        return None;
    }

    let with_dot_prefix = matched_base_path.starts_with("./");
    let replacement_path = if with_dot_prefix {
        format!("./{}", new_relative_path)
    } else {
        new_relative_path.to_string()
    };

    Some(format!(
        "{}{}{}{}{}",
        &statement[..path_start_idx],
        replacement_path,
        matched_suffix,
        quote_char,
        &statement[path_end_idx + 1..]
    ))
}

pub fn rename_icon_entry(
    folder_path: &str,
    current_file_path: &str,
    new_file_path_input: &str,
) -> anyhow::Result<()> {
    use std::fs;
    use std::path::{Component, Path};

    let current_relative_path = normalize_icon_relative_path(current_file_path);
    if current_relative_path.is_empty() {
        anyhow::bail!("Current icon path is empty");
    }

    let mut new_relative_path = normalize_icon_relative_path(new_file_path_input);
    if new_relative_path.is_empty() {
        anyhow::bail!("New filename cannot be empty");
    }

    let new_path = Path::new(&new_relative_path);
    if new_path.is_absolute() {
        anyhow::bail!("Please provide a relative filename");
    }
    if new_path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        anyhow::bail!("Parent directory traversals are not allowed");
    }

    if new_path.extension().is_none() {
        if let Some(extension) = Path::new(&current_relative_path)
            .extension()
            .and_then(|ext| ext.to_str())
        {
            new_relative_path = format!("{}.{}", new_relative_path, extension);
        }
    }

    if new_relative_path == current_relative_path {
        anyhow::bail!("Filename is unchanged");
    }

    let folder = Path::new(folder_path);
    let current_abs_path = folder.join(&current_relative_path);
    if !current_abs_path.exists() {
        anyhow::bail!("Icon file not found: {}", current_abs_path.display());
    }

    let new_abs_path = folder.join(&new_relative_path);
    if new_abs_path.exists() {
        anyhow::bail!("Target file already exists: {}", new_abs_path.display());
    }

    let index_path = folder.join("index.ts");
    if !index_path.exists() {
        anyhow::bail!("No index.ts found in folder: {}", folder.display());
    }

    let index_contents = fs::read_to_string(&index_path)?;
    let mut replaced_count = 0usize;
    let mut updated_lines = Vec::<String>::new();
    for line in index_contents.lines() {
        let mut parsed_export_in_line = false;

        for statement in line.split(';') {
            let statement = statement.trim();
            if statement.is_empty() {
                continue;
            }

            if parse_export_line_ts(statement).is_none() {
                continue;
            }

            parsed_export_in_line = true;

            if let Some(updated_statement) = replace_import_path_in_export_statement(
                statement,
                &current_relative_path,
                &new_relative_path,
            ) {
                updated_lines.push(format!("{updated_statement};"));
                replaced_count += 1;
            } else {
                updated_lines.push(format!("{statement};"));
            }
        }

        if !parsed_export_in_line {
            updated_lines.push(line.to_string());
        }
    }

    if replaced_count == 0 {
        anyhow::bail!(
            "Could not find an export path for '{}' in index.ts",
            current_file_path
        );
    }

    if let Some(parent) = new_abs_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::rename(&current_abs_path, &new_abs_path)?;

    let mut updated_index = updated_lines.join("\n");
    if index_contents.ends_with('\n') {
        updated_index.push('\n');
    }
    if let Err(write_error) = fs::write(&index_path, updated_index) {
        let _ = fs::rename(&new_abs_path, &current_abs_path);
        anyhow::bail!(
            "Failed to update index.ts after rename: {}. Rolled back file rename.",
            write_error
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn parses_iconify_name_from_plain_value() {
        assert_eq!(
            iconify_name_from_icon_source("mdi:heart-outline"),
            Some("mdi:heart-outline".to_string())
        );
    }

    #[test]
    fn parses_iconify_name_from_iconify_api_url() {
        assert_eq!(
            iconify_name_from_icon_source("https://api.iconify.design/mdi%3Aheart.svg"),
            Some("mdi:heart".to_string())
        );
    }

    #[test]
    fn parses_iconify_name_from_icones_collection_url() {
        assert_eq!(
            iconify_name_from_icon_source("https://icones.js.org/collection/lucide/icon/heart"),
            Some("lucide:heart".to_string())
        );
    }

    #[test]
    fn parses_iconify_name_from_icones_query_url() {
        assert_eq!(
            iconify_name_from_icon_source("https://icones.js.org/?icon=tabler:home"),
            Some("tabler:home".to_string())
        );
    }

    #[test]
    fn derives_name_and_filename_defaults() {
        assert_eq!(
            default_name_and_filename_from_icon_source(
                "https://api.iconify.design/mdi:arrow-left.svg"
            ),
            Some(("ArrowLeft".to_string(), "mdi:arrow-left".to_string()))
        );
    }

    #[test]
    fn parses_typescript_export_with_double_quotes() {
        let parsed =
            parse_export_line_ts("export { default as IconGithub } from \"./mdi:github.svg\";")
                .expect("export with double quotes should parse");

        assert_eq!(parsed.name, "IconGithub");
        assert_eq!(parsed.file_path, "./mdi:github.svg");
    }

    #[test]
    fn parses_typescript_export_with_single_quotes() {
        let parsed =
            parse_export_line_ts("export { default as IconGithub } from './mdi:github.svg';")
                .expect("export with single quotes should parse");

        assert_eq!(parsed.name, "IconGithub");
        assert_eq!(parsed.file_path, "./mdi:github.svg");
    }

    #[test]
    fn parses_typescript_export_with_extra_spacing() {
        let parsed = parse_export_line_ts(
            "export {  default   as   IconGithub } from   './mdi:github.svg';",
        )
        .expect("export with variable spacing should parse");

        assert_eq!(parsed.name, "IconGithub");
        assert_eq!(parsed.file_path, "./mdi:github.svg");
    }

    #[test]
    fn parses_typescript_export_and_strips_import_query() {
        let parsed = parse_export_line_ts(
            "export { default as IconGithub } from './mdi:github.svg?react#hash';",
        )
        .expect("export with query suffix should parse");

        assert_eq!(parsed.name, "IconGithub");
        assert_eq!(parsed.file_path, "./mdi:github.svg");
    }

    #[test]
    fn renames_file_and_updates_index_path() {
        let temp_dir = TempDir::new().expect("temp dir should be created");
        let icons_folder = temp_dir.path().join("icons");
        std::fs::create_dir_all(&icons_folder).expect("icons folder should be created");

        let old_file = icons_folder.join("old-name.svg");
        std::fs::write(&old_file, "<svg></svg>").expect("old file should be created");

        let index_path = icons_folder.join("index.ts");
        std::fs::write(
            &index_path,
            "export { default as IconHeart } from \"./old-name.svg\";\n",
        )
        .expect("index.ts should be created");

        rename_icon_entry(
            icons_folder.to_string_lossy().as_ref(),
            "./old-name.svg",
            "new-name",
        )
        .expect("rename should succeed");

        assert!(!old_file.exists(), "old file should be removed");
        assert!(
            icons_folder.join("new-name.svg").exists(),
            "new file should exist"
        );

        let index_contents =
            std::fs::read_to_string(&index_path).expect("index.ts should be readable");
        assert!(
            index_contents.contains("export { default as IconHeart } from \"./new-name.svg\";"),
            "index.ts should point to the renamed file"
        );
    }

    #[test]
    fn renames_path_without_touching_alias() {
        let temp_dir = TempDir::new().expect("temp dir should be created");
        let icons_folder = temp_dir.path().join("icons");
        std::fs::create_dir_all(&icons_folder).expect("icons folder should be created");

        let old_file = icons_folder.join("foo.svg");
        std::fs::write(&old_file, "<svg></svg>").expect("old file should be created");

        let index_path = icons_folder.join("index.ts");
        std::fs::write(
            &index_path,
            "export { default as IconAliasStays } from './foo.svg';\n",
        )
        .expect("index.ts should be created");

        rename_icon_entry(
            icons_folder.to_string_lossy().as_ref(),
            "./foo.svg",
            "bar.svg",
        )
        .expect("rename should succeed");

        let index_contents =
            std::fs::read_to_string(&index_path).expect("index.ts should be readable");
        assert!(
            index_contents.contains("export { default as IconAliasStays } from './bar.svg';"),
            "index.ts should update only the path"
        );
        assert!(
            !index_contents.contains("Iconbar") && index_contents.contains("IconAliasStays"),
            "icon alias should stay unchanged"
        );
    }

    #[test]
    fn renames_file_and_preserves_import_suffix() {
        let temp_dir = TempDir::new().expect("temp dir should be created");
        let icons_folder = temp_dir.path().join("icons");
        std::fs::create_dir_all(&icons_folder).expect("icons folder should be created");

        let old_file = icons_folder.join("foo.svg");
        std::fs::write(&old_file, "<svg></svg>").expect("old file should be created");

        let index_path = icons_folder.join("index.ts");
        std::fs::write(
            &index_path,
            "export { default as IconAliasStays } from './foo.svg?react#hash';\n",
        )
        .expect("index.ts should be created");

        rename_icon_entry(
            icons_folder.to_string_lossy().as_ref(),
            "./foo.svg",
            "bar.svg",
        )
        .expect("rename should succeed");

        let index_contents =
            std::fs::read_to_string(&index_path).expect("index.ts should be readable");
        assert!(
            index_contents
                .contains("export { default as IconAliasStays } from './bar.svg?react#hash';"),
            "index.ts should preserve import suffixes"
        );
    }

    #[test]
    fn get_existing_icons_reads_multiple_exports_on_same_line() {
        let temp_dir = TempDir::new().expect("temp dir should be created");
        let icons_folder = temp_dir.path().join("icons");
        std::fs::create_dir_all(&icons_folder).expect("icons folder should be created");

        let index_path = icons_folder.join("index.ts");
        std::fs::write(
            &index_path,
            "export { default as IconGithub } from './mdi:github.svg';export { default as IconDarkTheme24Filled } from './fluent:dark-theme-24-filled.svg';\n",
        )
        .expect("index.ts should be created");

        let icons = get_existing_icons(icons_folder.to_string_lossy().as_ref())
            .expect("icons should be discovered");

        assert_eq!(icons.len(), 2, "both exports should be discovered");
        assert!(icons.iter().any(|icon| icon.name == "IconGithub"));
        assert!(
            icons
                .iter()
                .any(|icon| icon.name == "IconDarkTheme24Filled")
        );
    }

    #[test]
    fn delete_icon_entry_preserves_trailing_newline() {
        let temp_dir = TempDir::new().expect("temp dir should be created");
        let icons_folder = temp_dir.path().join("icons");
        std::fs::create_dir_all(&icons_folder).expect("icons folder should be created");

        let keep_file = icons_folder.join("keep.svg");
        let remove_file = icons_folder.join("remove.svg");
        std::fs::write(&keep_file, "<svg></svg>").expect("keep icon should be created");
        std::fs::write(&remove_file, "<svg></svg>").expect("remove icon should be created");

        let index_path = icons_folder.join("index.ts");
        std::fs::write(
            &index_path,
            "export { default as IconKeep } from './keep.svg';\nexport { default as IconRemove } from './remove.svg';\n",
        )
        .expect("index.ts should be created");

        delete_icon_entry(remove_file.to_string_lossy().as_ref())
            .expect("delete should remove icon entry");

        let updated_index =
            std::fs::read_to_string(&index_path).expect("index.ts should be readable");
        assert!(
            updated_index.ends_with('\n'),
            "index.ts should retain trailing newline"
        );
        assert!(updated_index.contains("IconKeep"));
        assert!(!updated_index.contains("IconRemove"));
    }

    #[test]
    fn delete_icon_entry_removes_only_exact_export_path() {
        let temp_dir = TempDir::new().expect("temp dir should be created");
        let icons_folder = temp_dir.path().join("icons");
        std::fs::create_dir_all(&icons_folder).expect("icons folder should be created");

        let remove_file = icons_folder.join("remove.svg");
        let keep_file = icons_folder.join("remove-filled.svg");
        std::fs::write(&remove_file, "<svg></svg>").expect("remove icon should be created");
        std::fs::write(&keep_file, "<svg></svg>").expect("keep icon should be created");

        let index_path = icons_folder.join("index.ts");
        std::fs::write(
            &index_path,
            "export { default as IconRemove } from './remove.svg';\nexport { default as IconRemoveFilled } from './remove-filled.svg';\n",
        )
        .expect("index.ts should be created");

        delete_icon_entry(remove_file.to_string_lossy().as_ref())
            .expect("delete should remove only the exact icon entry");

        let updated_index =
            std::fs::read_to_string(&index_path).expect("index.ts should be readable");
        assert!(updated_index.contains("IconRemoveFilled"));
        assert!(!updated_index.contains("IconRemove }"));
    }

    #[test]
    fn delete_icon_entry_handles_absolute_dot_segment_path() {
        let temp_dir = TempDir::new().expect("temp dir should be created");
        let icons_folder = temp_dir.path().join("icons");
        std::fs::create_dir_all(&icons_folder).expect("icons folder should be created");

        let remove_file = icons_folder.join("remove.svg");
        std::fs::write(&remove_file, "<svg></svg>").expect("remove icon should be created");

        let index_path = icons_folder.join("index.ts");
        std::fs::write(
            &index_path,
            "export { default as IconRemove } from './remove.svg';\n",
        )
        .expect("index.ts should be created");

        let absolute_with_dot = icons_folder.join("./remove.svg");
        delete_icon_entry(absolute_with_dot.to_string_lossy().as_ref())
            .expect("delete should accept absolute paths with dot segments");

        let updated_index =
            std::fs::read_to_string(&index_path).expect("index.ts should be readable");
        assert!(!updated_index.contains("IconRemove"));
    }

    #[test]
    fn delete_icon_entry_updates_index_even_if_file_is_missing() {
        let temp_dir = TempDir::new().expect("temp dir should be created");
        let icons_folder = temp_dir.path().join("icons");
        std::fs::create_dir_all(&icons_folder).expect("icons folder should be created");

        let missing_file = icons_folder.join("fluent:dark-theme-24-regular.svg");

        let index_path = icons_folder.join("index.ts");
        std::fs::write(
            &index_path,
            "export { default as IconDarkTheme24Regular } from './fluent:dark-theme-24-regular.svg';\nexport { default as IconKeep } from './keep.svg';\n",
        )
        .expect("index.ts should be created");

        delete_icon_entry(missing_file.to_string_lossy().as_ref())
            .expect("delete should remove stale export when file is missing");

        let updated_index =
            std::fs::read_to_string(&index_path).expect("index.ts should be readable");
        assert!(!updated_index.contains("IconDarkTheme24Regular"));
        assert!(updated_index.contains("IconKeep"));
    }

    #[test]
    fn delete_icon_entry_removes_target_from_concatenated_export_line() {
        let temp_dir = TempDir::new().expect("temp dir should be created");
        let icons_folder = temp_dir.path().join("icons");
        std::fs::create_dir_all(&icons_folder).expect("icons folder should be created");

        let keep_file = icons_folder.join("keep.svg");
        let remove_file = icons_folder.join("remove.svg");
        std::fs::write(&keep_file, "<svg></svg>").expect("keep icon should be created");
        std::fs::write(&remove_file, "<svg></svg>").expect("remove icon should be created");

        let index_path = icons_folder.join("index.ts");
        std::fs::write(
            &index_path,
            "export { default as IconKeep } from './keep.svg';export { default as IconRemove } from './remove.svg';\n",
        )
        .expect("index.ts should be created");

        delete_icon_entry(remove_file.to_string_lossy().as_ref())
            .expect("delete should remove export from concatenated line");

        let updated_index =
            std::fs::read_to_string(&index_path).expect("index.ts should be readable");
        assert!(updated_index.contains("IconKeep"));
        assert!(!updated_index.contains("IconRemove"));
    }

    #[test]
    fn renames_target_from_concatenated_export_line() {
        let temp_dir = TempDir::new().expect("temp dir should be created");
        let icons_folder = temp_dir.path().join("icons");
        std::fs::create_dir_all(&icons_folder).expect("icons folder should be created");

        let keep_file = icons_folder.join("keep.svg");
        let rename_file = icons_folder.join("rename.svg");
        std::fs::write(&keep_file, "<svg></svg>").expect("keep icon should be created");
        std::fs::write(&rename_file, "<svg></svg>").expect("rename icon should be created");

        let index_path = icons_folder.join("index.ts");
        std::fs::write(
            &index_path,
            "export { default as IconKeep } from './keep.svg';export { default as IconRename } from './rename.svg?react#hash';\n",
        )
        .expect("index.ts should be created");

        rename_icon_entry(
            icons_folder.to_string_lossy().as_ref(),
            "./rename.svg",
            "renamed.svg",
        )
        .expect("rename should update export from concatenated line");

        assert!(!rename_file.exists(), "old file should be removed");
        assert!(
            icons_folder.join("renamed.svg").exists(),
            "renamed file should exist"
        );

        let index_contents =
            std::fs::read_to_string(&index_path).expect("index.ts should be readable");
        assert!(index_contents.contains("IconKeep"));
        assert!(
            index_contents
                .contains("export { default as IconRename } from './renamed.svg?react#hash';"),
            "index.ts should keep suffix while updating path"
        );
    }
}
