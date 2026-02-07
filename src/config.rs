use anyhow::Context;
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

use crate::utils::{PRESETS_OPTIONS, Preset};

pub const DEFAULT_FOLDER: &str = "src/assets/icons";
pub const DEFAULT_OUTPUT_LINE_TEMPLATE: &str =
    "export { default as Icon%name% } from './%icon%%ext%';";

#[derive(Debug, Clone, Default)]
struct LocalConfigFile {
    folder: Option<String>,
    preset: Option<String>,
    output_line_template: Option<String>,
    svg_viewer_cmd: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct GlobalConfigFile {
    svg_viewer_cmd: Option<String>,
}

#[derive(Debug, Clone)]
struct LoadedConfigFile<T> {
    path: PathBuf,
    value: T,
}

#[derive(Debug, Clone)]
pub struct ResolvedTuiConfig {
    pub folder: String,
    pub preset: String,
    pub output_line_template: String,
    pub svg_viewer_cmd: Option<String>,
    pub svg_viewer_cmd_source: String,
    pub global_config_loaded: bool,
    pub project_config_loaded: bool,
    pub warnings: Vec<String>,
    pub info: Vec<String>,
}

pub fn resolve_tui_config(
    cli_folder: Option<&PathBuf>,
    cli_preset: Option<&Preset>,
    cli_output_line_template: Option<&String>,
) -> anyhow::Result<ResolvedTuiConfig> {
    let mut warnings = Vec::new();
    let mut info = Vec::new();

    let local = load_local_config(&mut warnings)?;
    let global = load_global_config(&mut warnings)?;

    if let Some(config) = &local {
        info.push(format!(
            "Loaded local config from {}",
            config.path.display()
        ));
    }
    if let Some(config) = &global {
        info.push(format!(
            "Loaded global config from {}",
            config.path.display()
        ));
    }

    let folder = cli_folder
        .map(|path| path.display().to_string())
        .or_else(|| {
            local
                .as_ref()
                .and_then(|config| config.value.folder.clone())
        })
        .unwrap_or_else(|| DEFAULT_FOLDER.to_string());

    let preset = cli_preset
        .map(|preset| preset.to_str().to_string())
        .or_else(|| {
            local
                .as_ref()
                .and_then(|config| config.value.preset.clone())
        })
        .unwrap_or_else(|| "normal".to_string());

    let output_line_template = cli_output_line_template
        .cloned()
        .or_else(|| {
            local
                .as_ref()
                .and_then(|config| config.value.output_line_template.clone())
        })
        .unwrap_or_else(|| DEFAULT_OUTPUT_LINE_TEMPLATE.to_string());

    let (svg_viewer_cmd, svg_viewer_cmd_source) = if let Some(config) = &local {
        if let Some(command) = config.value.svg_viewer_cmd.clone() {
            (
                Some(command),
                format!("local config ({})", config.path.display()),
            )
        } else if let Some(global_config) = &global {
            if let Some(command) = global_config.value.svg_viewer_cmd.clone() {
                (
                    Some(command),
                    format!("global config ({})", global_config.path.display()),
                )
            } else {
                (None, "OS default".to_string())
            }
        } else {
            (None, "OS default".to_string())
        }
    } else if let Some(global_config) = &global {
        if let Some(command) = global_config.value.svg_viewer_cmd.clone() {
            (
                Some(command),
                format!("global config ({})", global_config.path.display()),
            )
        } else {
            (None, "OS default".to_string())
        }
    } else {
        (None, "OS default".to_string())
    };

    info.push(format!(
        "Resolved svg_viewer_cmd source: {}",
        svg_viewer_cmd_source
    ));

    Ok(ResolvedTuiConfig {
        folder,
        preset,
        output_line_template,
        svg_viewer_cmd,
        svg_viewer_cmd_source,
        global_config_loaded: global.is_some(),
        project_config_loaded: local.is_some(),
        warnings,
        info,
    })
}

fn load_local_config(
    warnings: &mut Vec<String>,
) -> anyhow::Result<Option<LoadedConfigFile<LocalConfigFile>>> {
    let current_dir =
        std::env::current_dir().context("Failed to resolve current working directory")?;
    let candidates = [
        current_dir.join("iconmate.config.jsonc"),
        current_dir.join("iconmate.config.json"),
        current_dir.join("iconmate.jsonc"),
        current_dir.join("iconmate.json"),
    ];

    let Some(path) = candidates.into_iter().find(|candidate| candidate.exists()) else {
        return Ok(None);
    };

    let value = parse_jsonc_file(&path)?;
    let parsed = parse_local_value(value, &path, warnings)?;
    Ok(Some(LoadedConfigFile {
        path,
        value: parsed,
    }))
}

fn load_global_config(
    warnings: &mut Vec<String>,
) -> anyhow::Result<Option<LoadedConfigFile<GlobalConfigFile>>> {
    let mut candidates = Vec::<PathBuf>::new();

    if let Some(config_dir) = dirs::config_dir() {
        candidates.push(config_dir.join("iconmate.jsonc"));
        candidates.push(config_dir.join("iconmate.json"));
        candidates.push(config_dir.join("iconmate").join("config.jsonc"));
        candidates.push(config_dir.join("iconmate").join("config.json"));
    }

    if let Some(home_dir) = dirs::home_dir() {
        candidates.push(home_dir.join("iconmate.jsonc"));
        candidates.push(home_dir.join("iconmate.json"));
    }

    candidates.dedup();
    let Some(path) = candidates.into_iter().find(|candidate| candidate.exists()) else {
        return Ok(None);
    };

    let value = parse_jsonc_file(&path)?;
    let parsed = parse_global_value(value, &path, warnings)?;
    Ok(Some(LoadedConfigFile {
        path,
        value: parsed,
    }))
}

fn parse_jsonc_file(path: &Path) -> anyhow::Result<Value> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file {}", path.display()))?;
    json5::from_str::<Value>(&raw).with_context(|| {
        format!(
            "Invalid config format in {}. Expected JSON/JSONC-compatible object.",
            path.display()
        )
    })
}

fn parse_local_value(
    value: Value,
    path: &Path,
    warnings: &mut Vec<String>,
) -> anyhow::Result<LocalConfigFile> {
    let object = as_object(value, path)?;
    warn_unknown_keys(
        &object,
        &[
            "folder",
            "preset",
            "output_line_template",
            "svg_view_cmd",
            "svg_viewer_cmd",
        ],
        path,
        warnings,
    );

    let folder = read_string_field(&object, path, "folder", false)?;

    let mut preset = read_string_field(&object, path, "preset", true)?;
    if matches!(preset.as_deref(), Some("")) {
        warnings.push(format!(
            "Config key 'preset' in {} uses deprecated empty value; use 'normal' instead.",
            path.display()
        ));
        preset = Some("normal".to_string());
    }

    if let Some(value) = preset.as_deref() {
        let valid_presets = PRESETS_OPTIONS
            .iter()
            .map(|option| option.preset.to_str())
            .collect::<Vec<_>>();

        if !valid_presets.contains(&value) {
            anyhow::bail!(
                "Invalid config at {}: key 'preset' must be one of [{}], got '{}'.",
                path.display(),
                valid_presets.join(", "),
                value
            );
        }
    }

    let output_line_template = read_string_field(&object, path, "output_line_template", false)?;
    let svg_viewer_cmd = read_svg_viewer_cmd(&object, path, warnings)?;

    Ok(LocalConfigFile {
        folder,
        preset,
        output_line_template,
        svg_viewer_cmd,
    })
}

fn parse_global_value(
    value: Value,
    path: &Path,
    warnings: &mut Vec<String>,
) -> anyhow::Result<GlobalConfigFile> {
    let object = as_object(value, path)?;
    warn_unknown_keys(&object, &["svg_view_cmd", "svg_viewer_cmd"], path, warnings);

    let svg_viewer_cmd = read_svg_viewer_cmd(&object, path, warnings)?;
    Ok(GlobalConfigFile { svg_viewer_cmd })
}

fn as_object(value: Value, path: &Path) -> anyhow::Result<Map<String, Value>> {
    value.as_object().cloned().ok_or_else(|| {
        anyhow::anyhow!(
            "Invalid config at {}: expected top-level object.",
            path.display()
        )
    })
}

fn warn_unknown_keys(
    object: &Map<String, Value>,
    allowed_keys: &[&str],
    path: &Path,
    warnings: &mut Vec<String>,
) {
    for key in object.keys() {
        if !allowed_keys.iter().any(|allowed| *allowed == key) {
            warnings.push(format!(
                "Ignoring unknown key '{}' in {}",
                key,
                path.display()
            ));
        }
    }
}

fn read_string_field(
    object: &Map<String, Value>,
    path: &Path,
    key: &str,
    allow_empty: bool,
) -> anyhow::Result<Option<String>> {
    let Some(value) = object.get(key) else {
        return Ok(None);
    };

    let Some(value) = value.as_str() else {
        anyhow::bail!(
            "Invalid config at {}: key '{}' must be a string.",
            path.display(),
            key
        );
    };

    if !allow_empty && value.trim().is_empty() {
        anyhow::bail!(
            "Invalid config at {}: key '{}' cannot be empty.",
            path.display(),
            key
        );
    }

    Ok(Some(value.to_string()))
}

fn read_svg_viewer_cmd(
    object: &Map<String, Value>,
    path: &Path,
    warnings: &mut Vec<String>,
) -> anyhow::Result<Option<String>> {
    let legacy = read_string_field(object, path, "svg_view_cmd", false)?;
    let modern = read_string_field(object, path, "svg_viewer_cmd", false)?;

    match (legacy, modern) {
        (Some(old_value), Some(new_value)) => {
            if old_value != new_value {
                warnings.push(format!(
                    "Both 'svg_view_cmd' and 'svg_viewer_cmd' are set in {}; using 'svg_viewer_cmd'.",
                    path.display()
                ));
            }
            Ok(Some(new_value))
        }
        (Some(value), None) => Ok(Some(value)),
        (None, Some(value)) => Ok(Some(value)),
        (None, None) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_global_svg_viewer_cmd_alias() {
        let value: Value = serde_json::json!({
            "svg_viewer_cmd": "code %filename%"
        });
        let mut warnings = Vec::new();
        let parsed =
            parse_global_value(value, Path::new("/tmp/iconmate.jsonc"), &mut warnings).unwrap();
        assert_eq!(parsed.svg_viewer_cmd, Some("code %filename%".to_string()));
        assert!(warnings.is_empty());
    }

    #[test]
    fn warns_on_unknown_global_key() {
        let value: Value = serde_json::json!({
            "svg_view_cmd": "open %filename%",
            "extra": true
        });
        let mut warnings = Vec::new();
        let _ = parse_global_value(value, Path::new("/tmp/iconmate.jsonc"), &mut warnings).unwrap();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("Ignoring unknown key 'extra'"));
    }

    #[test]
    fn validates_local_preset_values() {
        let value: Value = serde_json::json!({
            "preset": "invalid"
        });
        let mut warnings = Vec::new();
        let error = parse_local_value(
            value,
            Path::new("/tmp/iconmate.config.jsonc"),
            &mut warnings,
        )
        .expect_err("invalid preset should fail validation");
        assert!(error.to_string().contains("key 'preset' must be one of"));
    }

    #[test]
    fn normalizes_empty_local_preset_to_normal_with_warning() {
        let value: Value = serde_json::json!({
            "preset": ""
        });
        let mut warnings = Vec::new();
        let parsed = parse_local_value(
            value,
            Path::new("/tmp/iconmate.config.jsonc"),
            &mut warnings,
        )
        .expect("empty preset should be normalized");

        assert_eq!(parsed.preset.as_deref(), Some("normal"));
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("deprecated empty value"));
    }
}
