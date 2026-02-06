use anyhow::Context;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub enum OpenSvgOutcome {
    OpenedWithCustomCommand,
    OpenedWithOsDefault,
    OpenedWithOsDefaultAfterCustomFailure,
    OpenedWithWebPreview(String),
}

pub fn open_svg_with_fallback(
    svg_path: &Path,
    svg_viewer_cmd: Option<&str>,
) -> anyhow::Result<OpenSvgOutcome> {
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
        return spawn_background("open", &open_args)
            .context("Failed to open icon via macOS Quick Look/open");
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

fn open_url_in_browser(url: &str) -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    {
        return spawn_background("open", &[url.to_string()]).context("Failed to open URL via open");
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
}
