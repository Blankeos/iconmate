use anyhow::Context;
use regex::Regex;
use reqwest::Url;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub enum OpenSvgOutcome {
    OpenedWithCustomCommand,
    OpenedWithOsDefault,
    OpenedWithOsDefaultAfterCustomFailure,
    OpenedWithWebPreview(String),
}

fn svg_preview_contents(contents: &str) -> anyhow::Result<String> {
    let svg = extract_svg_fragment(contents)
        .ok_or_else(|| anyhow::anyhow!("No <svg> element found in selected icon."))?;
    Ok(ensure_svg_xmlns(&sanitize_svg_for_browser(svg)))
}

fn extract_svg_fragment(contents: &str) -> Option<&str> {
    let full_svg = Regex::new(r"(?is)<svg\b[^>]*>.*?</svg>").ok()?;
    if let Some(found) = full_svg.find(contents) {
        return Some(found.as_str());
    }

    let self_closing_svg = Regex::new(r"(?is)<svg\b[^>]*/>").ok()?;
    self_closing_svg.find(contents).map(|found| found.as_str())
}

fn sanitize_svg_for_browser(svg: &str) -> String {
    let mut output = svg.to_string();

    for (from, to) in JSX_SVG_ATTRIBUTE_REPLACEMENTS {
        output = output.replace(from, to);
    }

    output = replace_jsx_static_expression_attributes(&output);

    for pattern in [
        r"(?s)\s*\{\s*/\*.*?\*/\s*\}",
        r"(?s)\s+\{\s*\.\.\.[^}]+\}",
        r#"\s+v-bind(?::[A-Za-z0-9_.:-]+)?\s*=\s*("[^"]*"|'[^']*')"#,
        r#"\s+:[A-Za-z0-9_.:-]+\s*=\s*("[^"]*"|'[^']*')"#,
        r#"\s+(?:v-[A-Za-z0-9_.:-]+|@[A-Za-z0-9_.:-]+)\s*=\s*("[^"]*"|'[^']*')"#,
        r"(?s)\s+[A-Za-z_:][A-Za-z0-9_:.-]*\s*=\s*\{[^}]*\}",
    ] {
        let re = Regex::new(pattern).expect("valid preview sanitizer regex");
        output = re.replace_all(&output, "").to_string();
    }

    output
}

fn replace_jsx_static_expression_attributes(svg: &str) -> String {
    let mut output = svg.to_string();

    for pattern in [r#"=\{\s*"([^"]*)"\s*\}"#, r#"=\{\s*'([^']*)'\s*\}"#] {
        let re = Regex::new(pattern).expect("valid JSX string attribute regex");
        output = re
            .replace_all(&output, |captures: &regex::Captures<'_>| {
                format!("=\"{}\"", escape_xml_attribute(&captures[1]))
            })
            .to_string();
    }

    let scalar_re = Regex::new(r"=\{\s*([0-9]+(?:\.[0-9]+)?|true|false)\s*\}")
        .expect("valid JSX scalar attribute regex");
    scalar_re
        .replace_all(&output, |captures: &regex::Captures<'_>| {
            format!("=\"{}\"", &captures[1])
        })
        .to_string()
}

fn escape_xml_attribute(value: &str) -> String {
    value.replace('&', "&amp;").replace('"', "&quot;")
}

fn ensure_svg_xmlns(svg: &str) -> String {
    let Some(tag_end) = svg.find('>') else {
        return svg.to_string();
    };
    let opening_tag = &svg[..tag_end];
    if opening_tag.contains("xmlns=") || opening_tag.contains("xmlns:") {
        return svg.to_string();
    }

    format!(
        "{} xmlns=\"http://www.w3.org/2000/svg\"{}",
        opening_tag,
        &svg[tag_end..]
    )
}

fn preview_file_path(source_path: &Path, preview_svg: &str) -> std::path::PathBuf {
    let mut hasher = DefaultHasher::new();
    source_path.hash(&mut hasher);
    preview_svg.hash(&mut hasher);
    std::env::temp_dir().join(format!(
        "iconmate-preview-{}-{:x}.svg",
        std::process::id(),
        hasher.finish()
    ))
}

const JSX_SVG_ATTRIBUTE_REPLACEMENTS: &[(&str, &str)] = &[
    ("accentHeight=", "accent-height="),
    ("alignmentBaseline=", "alignment-baseline="),
    ("baselineShift=", "baseline-shift="),
    ("clipPath=", "clip-path="),
    ("clipRule=", "clip-rule="),
    ("colorInterpolation=", "color-interpolation="),
    ("colorInterpolationFilters=", "color-interpolation-filters="),
    ("colorProfile=", "color-profile="),
    ("colorRendering=", "color-rendering="),
    ("dominantBaseline=", "dominant-baseline="),
    ("enableBackground=", "enable-background="),
    ("fillOpacity=", "fill-opacity="),
    ("fillRule=", "fill-rule="),
    ("floodColor=", "flood-color="),
    ("floodOpacity=", "flood-opacity="),
    ("fontFamily=", "font-family="),
    ("fontSize=", "font-size="),
    ("fontSizeAdjust=", "font-size-adjust="),
    ("fontStretch=", "font-stretch="),
    ("fontStyle=", "font-style="),
    ("fontVariant=", "font-variant="),
    ("fontWeight=", "font-weight="),
    (
        "glyphOrientationHorizontal=",
        "glyph-orientation-horizontal=",
    ),
    ("glyphOrientationVertical=", "glyph-orientation-vertical="),
    ("imageRendering=", "image-rendering="),
    ("letterSpacing=", "letter-spacing="),
    ("lightingColor=", "lighting-color="),
    ("markerEnd=", "marker-end="),
    ("markerMid=", "marker-mid="),
    ("markerStart=", "marker-start="),
    ("overlinePosition=", "overline-position="),
    ("overlineThickness=", "overline-thickness="),
    ("paintOrder=", "paint-order="),
    ("pointerEvents=", "pointer-events="),
    ("shapeRendering=", "shape-rendering="),
    ("stopColor=", "stop-color="),
    ("stopOpacity=", "stop-opacity="),
    ("strikethroughPosition=", "strikethrough-position="),
    ("strikethroughThickness=", "strikethrough-thickness="),
    ("strokeDasharray=", "stroke-dasharray="),
    ("strokeDashoffset=", "stroke-dashoffset="),
    ("strokeLinecap=", "stroke-linecap="),
    ("strokeLinejoin=", "stroke-linejoin="),
    ("strokeMiterlimit=", "stroke-miterlimit="),
    ("strokeOpacity=", "stroke-opacity="),
    ("strokeWidth=", "stroke-width="),
    ("textAnchor=", "text-anchor="),
    ("textDecoration=", "text-decoration="),
    ("textRendering=", "text-rendering="),
    ("transformOrigin=", "transform-origin="),
    ("underlinePosition=", "underline-position="),
    ("underlineThickness=", "underline-thickness="),
    ("unicodeBidi=", "unicode-bidi="),
    ("vectorEffect=", "vector-effect="),
    ("wordSpacing=", "word-spacing="),
    ("writingMode=", "writing-mode="),
    ("xHeight=", "x-height="),
    ("xlinkActuate=", "xlink:actuate="),
    ("xlinkArcrole=", "xlink:arcrole="),
    ("xlinkHref=", "xlink:href="),
    ("xlinkRole=", "xlink:role="),
    ("xlinkShow=", "xlink:show="),
    ("xlinkTitle=", "xlink:title="),
    ("xlinkType=", "xlink:type="),
    ("xmlBase=", "xml:base="),
    ("xmlLang=", "xml:lang="),
    ("xmlSpace=", "xml:space="),
    ("xmlnsXlink=", "xmlns:xlink="),
    ("className=", "class="),
];

pub fn preview_svg_in_browser(svg_path: &Path) -> anyhow::Result<()> {
    let resolved_path = crate::utils::resolve_existing_icon_path(svg_path);
    let svg_path = resolved_path.as_path();

    if !svg_path.exists() {
        anyhow::bail!("Icon file not found: {}", svg_path.display());
    }

    let contents = fs::read_to_string(svg_path)
        .with_context(|| format!("Failed to read icon file {}", svg_path.display()))?;
    let preview_svg = svg_preview_contents(&contents)?;
    let preview_path = preview_file_path(svg_path, &preview_svg);

    fs::write(&preview_path, preview_svg)
        .with_context(|| format!("Failed to write preview SVG {}", preview_path.display()))?;

    let preview_url = Url::from_file_path(&preview_path)
        .map_err(|_| anyhow::anyhow!("Failed to build file URL for {}", preview_path.display()))?;
    open_url_in_browser(preview_url.as_str())
        .with_context(|| format!("Failed to open preview SVG {}", preview_path.display()))
}

pub fn open_svg_with_fallback(
    svg_path: &Path,
    svg_viewer_cmd: Option<&str>,
) -> anyhow::Result<OpenSvgOutcome> {
    let resolved_path = crate::utils::resolve_existing_icon_path(svg_path);
    let svg_path = resolved_path.as_path();

    if !svg_path.exists() {
        anyhow::bail!("Icon file not found: {}", svg_path.display());
    }

    let mut errors = Vec::<String>::new();

    if let Some(command_template) = svg_viewer_cmd {
        match open_with_custom_command(command_template, svg_path) {
            Ok(()) => return Ok(OpenSvgOutcome::OpenedWithCustomCommand),
            Err(error) => errors.push(format!("custom svg_viewer_cmd failed: {error}")),
        }
    }

    match open_with_os_default(svg_path) {
        Ok(()) => {
            if svg_viewer_cmd.is_some() {
                return Ok(OpenSvgOutcome::OpenedWithOsDefaultAfterCustomFailure);
            }
            return Ok(OpenSvgOutcome::OpenedWithOsDefault);
        }
        Err(error) => errors.push(format!("OS default open failed: {error}")),
    }

    if let Some(web_preview_url) = iconify_web_preview_url(svg_path) {
        open_url_in_browser(&web_preview_url).with_context(|| {
            format!(
                "Failed to open web preview URL after local open failures: {}",
                web_preview_url
            )
        })?;
        return Ok(OpenSvgOutcome::OpenedWithWebPreview(web_preview_url));
    }

    if errors.is_empty() {
        anyhow::bail!("Failed to open icon file {}", svg_path.display());
    }

    anyhow::bail!(
        "Failed to open icon file {}. {}",
        svg_path.display(),
        errors.join(" | ")
    )
}

fn open_with_custom_command(command_template: &str, svg_path: &Path) -> anyhow::Result<()> {
    let mut parts = shlex::split(command_template).ok_or_else(|| {
        anyhow::anyhow!(
            "Could not parse svg_viewer_cmd. Check quoting in '{}'.",
            command_template
        )
    })?;

    if parts.is_empty() {
        anyhow::bail!("svg_viewer_cmd is empty");
    }

    let file_name = svg_path.to_string_lossy().to_string();
    let mut used_placeholder = false;
    for part in &mut parts {
        if part.contains("%filename%") {
            *part = part.replace("%filename%", &file_name);
            used_placeholder = true;
        }
    }

    if !used_placeholder {
        parts.push(file_name);
    }

    let executable = parts.remove(0);
    spawn_background(&executable, &parts)
        .with_context(|| format!("Failed to run svg_viewer_cmd '{}'.", command_template))
}

fn spawn_background(executable: &str, args: &[String]) -> anyhow::Result<()> {
    Command::new(executable)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .with_context(|| {
            if args.is_empty() {
                format!("Failed to spawn '{}'", executable)
            } else {
                format!("Failed to spawn '{}' with args {:?}", executable, args)
            }
        })?;
    Ok(())
}

fn open_with_os_default(svg_path: &Path) -> anyhow::Result<()> {
    let file_name = svg_path.to_string_lossy().to_string();

    #[cfg(target_os = "macos")]
    {
        let ql_args = vec!["-p".to_string(), file_name.clone()];
        if spawn_background("qlmanage", &ql_args).is_ok() {
            return Ok(());
        }

        let open_args = vec![file_name];
        spawn_background("open", &open_args)
            .context("Failed to open icon via macOS Quick Look/open")
    }

    #[cfg(target_os = "linux")]
    {
        let args = vec![file_name];
        return spawn_background("xdg-open", &args).context("Failed to open icon via xdg-open");
    }

    #[cfg(target_os = "windows")]
    {
        let args = vec![
            "/C".to_string(),
            "start".to_string(),
            "".to_string(),
            file_name,
        ];
        return spawn_background("cmd", &args).context("Failed to open icon via cmd start");
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        anyhow::bail!("Unsupported OS for default icon opener")
    }
}

fn iconify_web_preview_url(svg_path: &Path) -> Option<String> {
    let stem = svg_path.file_stem()?.to_string_lossy();
    let icon_name = crate::utils::iconify_name_from_icon_source(stem.as_ref())?;
    let encoded = icon_name.replace(':', "%3A");
    Some(format!("https://api.iconify.design/{encoded}.svg"))
}

pub fn open_url_in_browser(url: &str) -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    {
        spawn_background("open", &[url.to_string()]).context("Failed to open URL via open")
    }

    #[cfg(target_os = "linux")]
    {
        return spawn_background("xdg-open", &[url.to_string()])
            .context("Failed to open URL via xdg-open");
    }

    #[cfg(target_os = "windows")]
    {
        let args = vec![
            "/C".to_string(),
            "start".to_string(),
            "".to_string(),
            url.to_string(),
        ];
        return spawn_background("cmd", &args).context("Failed to open URL via cmd start");
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        anyhow::bail!("Unsupported OS for URL opener")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_iconify_web_preview_url_from_iconify_stem() {
        let path = Path::new("/tmp/mdi:heart.svg");
        assert_eq!(
            iconify_web_preview_url(path),
            Some("https://api.iconify.design/mdi%3Aheart.svg".to_string())
        );
    }

    #[test]
    fn returns_none_for_non_iconify_stem() {
        let path = Path::new("/tmp/logo.svg");
        assert_eq!(iconify_web_preview_url(path), None);
    }

    #[test]
    fn extracts_and_sanitizes_react_svg_component() {
        let contents = r#"
import type { SVGProps } from 'react';

export default function Icon(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...props} className="size-4" strokeWidth={2} fillRule="evenodd" viewBox="0 0 24 24"><path d="M0 0" /></svg>
  );
}
"#;

        let svg = svg_preview_contents(contents).unwrap();
        assert!(svg.starts_with("<svg "));
        assert!(svg.contains("xmlns=\"http://www.w3.org/2000/svg\""));
        assert!(svg.contains("class=\"size-4\""));
        assert!(svg.contains("stroke-width=\"2\""));
        assert!(svg.contains("fill-rule=\"evenodd\""));
        assert!(!svg.contains("{...props}"));
        assert!(!svg.contains("className"));
        assert!(!svg.contains("strokeWidth"));
    }

    #[test]
    fn sanitizes_svelte_and_vue_dynamic_bindings() {
        let svelte = r#"
<script lang="ts">
  let { ...props } = $props();
</script>

<svg {...props} viewBox="0 0 24 24"><path strokeLinecap="round" /></svg>
"#;
        let vue = r#"
<template>
  <svg v-bind="$props" :class="size" @click="onClick" viewBox="0 0 24 24"><path :d="path" /></svg>
</template>
"#;

        let svelte_svg = svg_preview_contents(svelte).unwrap();
        assert!(svelte_svg.contains("stroke-linecap=\"round\""));
        assert!(!svelte_svg.contains("{...props}"));

        let vue_svg = svg_preview_contents(vue).unwrap();
        assert!(!vue_svg.contains("v-bind"));
        assert!(!vue_svg.contains(":class"));
        assert!(!vue_svg.contains("@click"));
        assert!(!vue_svg.contains(":d"));
    }

    #[test]
    fn errors_when_no_svg_fragment_exists() {
        let error = svg_preview_contents("export default null").unwrap_err();
        assert!(error.to_string().contains("No <svg> element"));
    }
}
