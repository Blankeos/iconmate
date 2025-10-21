use clap::ValueEnum;
use ratatui::layout::{Constraint, Rect};
use reqwest::Url;

#[derive(ValueEnum, Clone, Debug, PartialEq)]
pub enum Preset {
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
pub const PRESETS: &[Preset] = &[
    Preset::EmptySvg,
    Preset::React,
    Preset::Svelte,
    Preset::Solid,
    Preset::Vue,
];

/// helper function to create a centered rect using up certain percentage of the available rect `r`
pub fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = ratatui::layout::Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(ratatui::layout::Flex::Center);
    let horizontal = ratatui::layout::Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(ratatui::layout::Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

/// Struct to hold icon information for deletion
#[derive(Debug)]
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
