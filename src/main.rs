mod app_state;
mod config;
mod flutter;
mod iconify;
mod scroll;
mod sync;
mod tui;
mod utils;
mod viewer;
mod views;

use crate::iconify::{IconifyClient, IconifyCollectionResponse, IconifySearchResponse};
use crate::utils::{
    _determine_icon_source_type, _icon_source_to_svg, _make_svg_filename, IconEntry,
    IconSourceType, PRESETS_OPTIONS, Preset, default_name_and_filename_from_icon_source,
    render_js_export_line,
};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// A CLI tool to fetch icons and save them into your Vite, NextJS, or similar project.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Pathname of the folder where the icon will be saved and index.ts updated.
    #[arg(long)]
    folder: Option<PathBuf>,

    /// Optional preset to use instead of fetching an icon.
    #[arg(long)]
    preset: Option<Preset>,

    /// The alias for the SVG, used in the index.ts export (e.g., "Chevron").
    #[arg(long)]
    name: Option<String>,

    /// The name of the icon (e.g., "stash:chevron") or a full URL to the icon (e.g., "https://api.iconify.design/stash:chevron.svg") or an SVG.
    #[arg(long)]
    icon: Option<String>,

    /// Optional custom filename for the SVG file (without extension). Defaults to the icon name.
    #[arg(long)]
    filename: Option<String>,

    /// Flutter preset only: path to the Dart barrel file (project-root-relative).
    #[arg(long)]
    flutter_barrel_file: Option<PathBuf>,

    /// Flutter preset only: Dart class name in the barrel.
    #[arg(long)]
    flutter_barrel_class: Option<String>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Add an icon by specifying its details via command-line arguments.
    Add {
        /// Optional preset to use instead of fetching an icon.
        #[arg(long)]
        preset: Option<Preset>,

        /// Pathname of the folder where the icon will be saved and index.ts updated.
        #[arg(long)]
        folder: PathBuf,

        /// The alias for the SVG, used in the index.ts export (e.g., "Chevron").
        /// Optional when --icon is a URL or iconify id — iconmate auto-infers from the icon name.
        #[arg(long)]
        name: Option<String>,

        /// The name of the icon (e.g., "stash:chevron") or a full URL to the icon (e.g., "https://api.iconify.design/stash:chevron.svg") or an SVG.
        #[arg(long)]
        icon: Option<String>,

        /// Optional custom filename for the SVG file (without extension). Defaults to the icon name.
        #[arg(long)]
        filename: Option<String>,

        /// Flutter preset only: path to the Dart barrel file (project-root-relative). Default: lib/icons.dart
        #[arg(long)]
        flutter_barrel_file: Option<PathBuf>,

        /// Flutter preset only: Dart class name in the barrel. Default: AppIcons
        #[arg(long)]
        flutter_barrel_class: Option<String>,
    },

    /// Start an interactive prompt to add icons.
    Tui {},

    /// Delete an icon from your collection of icons
    Delete {
        /// Pathname of the folder where all the icons are saved.
        #[arg(long)]
        folder: Option<PathBuf>,

        /// Delete by export alias (e.g. "Chevron" matching `export { default as IconChevron }`).
        /// Can be passed multiple times. When provided, runs non-interactively.
        #[arg(long = "name")]
        names: Vec<String>,

        /// Delete by file path as it appears in index.ts (e.g. "./stash-chevron.svg").
        /// Can be passed multiple times. When provided, runs non-interactively.
        #[arg(long = "filename")]
        filenames: Vec<String>,

        /// Skip the confirmation prompt. Required for non-interactive deletes.
        #[arg(long, short = 'y')]
        yes: bool,
    },

    /// List all icons currently exported in the icons folder.
    #[command(visible_alias = "ls")]
    List {
        /// Pathname of the folder where all the icons are saved.
        #[arg(long)]
        folder: Option<PathBuf>,
    },

    /// Query Iconify collections, search results, and raw SVGs.
    Iconify {
        #[command(subcommand)]
        command: IconifyCommands,
    },

    /// Reconcile the barrel file (index.ts / lib/icons.dart) with the SVGs on disk.
    /// Dry-run by default. Never touches SVG assets.
    Sync {
        /// Pathname of the folder where icons live.
        #[arg(long)]
        folder: Option<PathBuf>,

        /// Actually write changes. Without this flag, sync prints the plan and exits.
        #[arg(long)]
        apply: bool,

        /// Also remove orphan entries (barrel entries whose SVG is missing).
        /// Requires --apply.
        #[arg(long)]
        prune: bool,

        /// Override an inferred identifier. Repeatable. Format: `--rename old=new`.
        #[arg(long = "rename", value_name = "OLD=NEW")]
        renames: Vec<String>,
    },
}

#[derive(Clone, Debug, ValueEnum, PartialEq)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Clone, Debug, ValueEnum)]
enum GetFormat {
    Svg,
    Json,
}

#[derive(Debug, Subcommand)]
enum IconifyCommands {
    /// Search Iconify by keyword.
    Search {
        /// Search query, such as "heart".
        query: String,

        /// Maximum number of records.
        #[arg(long)]
        limit: Option<u32>,

        /// Start offset for pagination.
        #[arg(long)]
        start: Option<u32>,

        /// Output format.
        #[arg(long, value_enum, default_value = "text")]
        format: OutputFormat,

        /// Include collection metadata (JSON mode only).
        #[arg(long)]
        include_collections: bool,
    },

    /// List available Iconify collections.
    Collections {
        /// Output format.
        #[arg(long, value_enum, default_value = "text")]
        format: OutputFormat,
    },

    /// List all icons in a collection prefix.
    Collection {
        /// Collection prefix, such as "mdi".
        prefix: String,

        /// Output format.
        #[arg(long, value_enum, default_value = "text")]
        format: OutputFormat,
    },

    /// Fetch one icon by Iconify name (<prefix:icon>).
    Get {
        /// Iconify icon name, such as "mdi:heart".
        icon: String,

        /// Output format.
        #[arg(long, value_enum, default_value = "svg")]
        format: GetFormat,
    },
}

/// Configuration for the icon fetching and saving logic.
struct AppConfig {
    folder: PathBuf,
    name: Option<String>,
    icon: Option<String>,
    filename: Option<String>,
    preset: Option<Preset>,
    flutter_barrel_file: Option<PathBuf>,
    flutter_barrel_class: Option<String>,
}

#[derive(Serialize)]
struct SearchJsonOutput {
    icons: Vec<String>,
    total: u32,
    limit: u32,
    start: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    collections: Option<std::collections::HashMap<String, serde_json::Value>>,
}

#[derive(Serialize)]
struct CollectionsJsonOutput {
    prefix: String,
    name: String,
    total: u32,
}

#[derive(Serialize)]
struct CollectionJsonOutput {
    prefix: String,
    icons: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    uncategorized: Option<Vec<String>>,
}

fn print_json<T: Serialize>(value: &T) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn iconify_error_to_anyhow(error: crate::iconify::IconifyError) -> anyhow::Error {
    match error {
        crate::iconify::IconifyError::HttpStatus {
            status,
            endpoint,
            body,
        } => {
            if status == reqwest::StatusCode::NOT_FOUND && endpoint.contains("/collection?prefix=")
            {
                return anyhow::anyhow!(
                    "Iconify collection not found. Use a collection prefix like 'mdi' (not 'mdi:home')."
                );
            }

            let body = body.trim();
            if body.is_empty() {
                anyhow::anyhow!("Iconify request failed ({status}) for {endpoint}")
            } else {
                anyhow::anyhow!(
                    "Iconify request failed ({status}) for {endpoint}. Response: {body}"
                )
            }
        }
        other => anyhow::Error::new(other),
    }
}

fn into_collection_output(response: IconifyCollectionResponse) -> CollectionJsonOutput {
    CollectionJsonOutput {
        prefix: response.prefix,
        icons: response.icons,
        uncategorized: response.uncategorized,
    }
}

async fn run_iconify_command(command: IconifyCommands) -> anyhow::Result<()> {
    let client = IconifyClient::from_env().map_err(iconify_error_to_anyhow)?;

    match command {
        IconifyCommands::Search {
            query,
            limit,
            start,
            format,
            include_collections,
        } => {
            if include_collections && format != OutputFormat::Json {
                anyhow::bail!("--include-collections can only be used with --format json");
            }

            let response: IconifySearchResponse = client
                .search(&query, limit, start, include_collections)
                .await
                .map_err(iconify_error_to_anyhow)?;

            match format {
                OutputFormat::Text => {
                    for icon in response.icons {
                        println!("{icon}");
                    }
                }
                OutputFormat::Json => {
                    let payload = SearchJsonOutput {
                        icons: response.icons,
                        total: response.total,
                        limit: response.limit,
                        start: response.start,
                        collections: response.collections,
                    };
                    print_json(&payload)?;
                }
            }
        }
        IconifyCommands::Collections { format } => {
            let response = client
                .collections()
                .await
                .map_err(iconify_error_to_anyhow)?;

            let mut rows: Vec<CollectionsJsonOutput> = response
                .collections
                .into_iter()
                .map(|(prefix, meta)| CollectionsJsonOutput {
                    name: meta.display_name(&prefix),
                    total: meta.total.unwrap_or(0),
                    prefix,
                })
                .collect();

            rows.sort_by(|a, b| a.prefix.cmp(&b.prefix));

            match format {
                OutputFormat::Text => {
                    for row in rows {
                        println!("{}\t{}\t{}", row.prefix, row.name, row.total);
                    }
                }
                OutputFormat::Json => {
                    print_json(&rows)?;
                }
            }
        }
        IconifyCommands::Collection { prefix, format } => {
            let prefix = prefix
                .split_once(':')
                .map(|(collection_prefix, _)| collection_prefix)
                .unwrap_or(&prefix)
                .to_string();

            let response = client
                .collection(&prefix)
                .await
                .map_err(iconify_error_to_anyhow)?;

            match format {
                OutputFormat::Text => {
                    for icon in &response.icons {
                        println!("{}:{icon}", response.prefix);
                    }
                }
                OutputFormat::Json => {
                    let payload = into_collection_output(response);
                    print_json(&payload)?;
                }
            }
        }
        IconifyCommands::Get { icon, format } => match format {
            GetFormat::Svg => {
                let svg = client.svg(&icon).await.map_err(iconify_error_to_anyhow)?;
                println!("{svg}");
            }
            GetFormat::Json => {
                let payload = client
                    .icon_json_by_name(&icon)
                    .await
                    .map_err(iconify_error_to_anyhow)?;
                print_json(&payload)?;
            }
        },
    }

    Ok(())
}

/// Resolve the final component/identifier name from CLI input + the icon
/// source. For every preset, `--name` is optional as long as the icon source
/// is a URL or iconify id we can derive a default from.
///
/// `collection_hint` (e.g. "mdi" from "mdi:heart") is used as the fallback
/// segment when the primary name collides with an existing entry.
fn resolve_icon_alias(
    cli_name: Option<&str>,
    icon_source: Option<&str>,
) -> anyhow::Result<(String, Option<String>)> {
    if let Some(name) = cli_name {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            let collection = icon_source
                .and_then(crate::utils::iconify_name_from_icon_source)
                .and_then(|iconify| iconify.split_once(':').map(|(p, _)| p.to_string()));
            return Ok((trimmed.to_string(), collection));
        }
    }

    let Some(icon) = icon_source else {
        anyhow::bail!("--name is required when no icon source is provided.");
    };

    let Some((default_name, _default_filename)) =
        crate::utils::default_name_and_filename_from_icon_source(icon)
    else {
        anyhow::bail!(
            "Could not infer --name from icon source '{}'. Pass --name explicitly.",
            icon
        );
    };
    let collection = crate::utils::iconify_name_from_icon_source(icon)
        .and_then(|iconify| iconify.split_once(':').map(|(p, _)| p.to_string()));
    Ok((default_name, collection))
}

/// The main logic of the application.
/// Fetches an icon, saves it, and updates the index (or Dart barrel).
async fn run_app(config: AppConfig) -> anyhow::Result<()> {
    let folder_path = &config.folder;
    let effective_preset = config.preset.clone().unwrap_or(Preset::Normal);

    // For Flutter, --name may be lowerCamelCase from user; for JS presets
    // PascalCase is conventional. Either way, `resolve_icon_alias` returns the
    // raw string — sanitization per-preset happens below.
    let (raw_alias, collection_hint) =
        resolve_icon_alias(config.name.as_deref(), config.icon.as_deref())?;

    fs::create_dir_all(folder_path)?;

    if matches!(effective_preset, Preset::Flutter) {
        return run_app_flutter(config, raw_alias, collection_hint).await;
    }

    let icon_alias = raw_alias.clone();

    // Determine SVG content and filename stem based on a valid combination of arguments.
    let (svg_content, file_stem_str, ext) = match (&config.icon, effective_preset) {
        // Case 1: Icon is provided AND the preset is EmptySvg. This is the only mutual exclusivity.
        (Some(_), Preset::EmptySvg) => {
            anyhow::bail!(
                "The --icon argument cannot be used with the --preset emptysvg. Please provide only one or the other."
            );
        }

        // Case 2: Only a preset is provided.
        (None, Preset::EmptySvg) => {
            let content = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"></svg>"#.to_string();
            let (file_stem, ext) = _make_svg_filename(
                config.filename.as_ref(),
                ".svg",
                config.icon.as_ref(),
                &icon_alias,
            );
            Ok::<(String, String, &'static str), anyhow::Error>((content, file_stem, ext))
        }

        // Case 3: React
        (icon_source, Preset::React) => {
            let content = _icon_source_to_svg(icon_source, Some("{...props}"), true).await?;
            let content = format!(
                "import type {{ SVGProps }} from 'react';\n\nexport default function Icon(props: SVGProps<SVGSVGElement>) {{\n  return (\n{}\n  );\n}}",
                content
            );
            let (file_stem, ext) = _make_svg_filename(
                config.filename.as_ref(),
                ".tsx",
                config.icon.as_ref(),
                &icon_alias,
            );
            Ok::<(String, String, &'static str), anyhow::Error>((content, file_stem, ext))
        }

        // Case 4: Svelte
        (icon_source, Preset::Svelte) => {
            let content = _icon_source_to_svg(icon_source, Some("{...props}"), false).await?;
            let content = format!(
                "<script lang=\"ts\">\n  import type {{ SVGAttributes }} from 'svelte/elements';\n\n  let {{ ...props }}: SVGAttributes<SVGSVGElement> = $props();\n</script>\n\n{}",
                content
            );
            let (file_stem, ext) = _make_svg_filename(
                config.filename.as_ref(),
                ".svelte",
                config.icon.as_ref(),
                &icon_alias,
            );
            Ok::<(String, String, &'static str), anyhow::Error>((content, file_stem, ext))
        }

        // Case 5: Solid
        (icon_source, Preset::Solid) => {
            let content = _icon_source_to_svg(icon_source, Some("{...props}"), true).await?;
            let content = format!(
                "import {{ type JSX }} from 'solid-js';\n\nexport default function Icon(props: JSX.SvgSVGAttributes<SVGSVGElement>) {{\n  return ({});\n}}",
                content
            );
            let (file_stem, ext) = _make_svg_filename(
                config.filename.as_ref(),
                ".tsx",
                config.icon.as_ref(),
                &icon_alias,
            );
            Ok::<(String, String, &'static str), anyhow::Error>((content, file_stem, ext))
        }

        // Case 6: Vue
        (icon_source, Preset::Vue) => {
            let content = _icon_source_to_svg(icon_source, Some("v-bind=\"$props\""), true).await?;
            let content = format!(
                "<template>\n  <template>\n    {}\n  </template>\n</template>\n\n<script setup lang=\"ts\">\nimport type {{ SVGAttributes }} from 'vue'\n\ndefineProps<SVGAttributes>()\n</script>",
                content
            );
            let (file_stem, ext) = _make_svg_filename(
                config.filename.as_ref(),
                ".vue",
                config.icon.as_ref(),
                &icon_alias,
            );
            Ok::<(String, String, &'static str), anyhow::Error>((content, file_stem, ext))
        }

        // Case 7: Only an icon is provided in `normal` mode.
        (Some(icon_source), Preset::Normal) => {
            let content = _icon_source_to_svg(&Some(icon_source.clone()), None, false).await?;
            let (file_stem, ext) = _make_svg_filename(
                config.filename.as_ref(),
                ".svg",
                config.icon.as_ref(),
                &icon_alias,
            );
            Ok((content, file_stem, ext))
        }

        // Case 8: Normal mode still requires an icon source.
        (None, Preset::Normal) => {
            anyhow::bail!("The --icon argument is required when --preset is normal.");
        }

        // Case 9: Flutter — handled above via run_app_flutter, unreachable here.
        (_, Preset::Flutter) => unreachable!("Flutter handled in run_app_flutter"),
    }?;

    // The rest of the function can now safely assume it has the content and a filename stem.
    let svg_file_name = format!("{}{}", file_stem_str, ext);
    let svg_file_path = folder_path.join(&svg_file_name);

    // Update or create index.ts
    let index_ts_path = folder_path.join("index.ts");
    let existing_index = if index_ts_path.exists() {
        Some(fs::read_to_string(&index_ts_path)?)
    } else {
        None
    };
    let rendered_export_statement = render_js_export_line(
        existing_index.as_deref(),
        folder_path,
        &icon_alias,
        &file_stem_str,
        ext,
    );
    let export_line = format!("{}\n", rendered_export_statement);

    if let Some(existing_index) = existing_index.as_deref() {
        validate_new_export_conflicts(existing_index, &rendered_export_statement, &index_ts_path)?;
    }

    if svg_file_path.exists() {
        anyhow::bail!(
            "Target icon file already exists: {}. Choose a different --filename (or --name when filename is omitted).",
            svg_file_path.display()
        );
    }

    fs::write(&svg_file_path, &svg_content)?;
    println!("Successfully saved icon to: {}", svg_file_path.display());

    if index_ts_path.exists() {
        let mut contents = fs::read_to_string(&index_ts_path)?;
        let export_line_trimmed = export_line.trim_end();
        let export_already_exists = contents
            .lines()
            .any(|line| line.trim_end() == export_line_trimmed);

        if !export_already_exists {
            if !contents.is_empty() && !contents.ends_with('\n') {
                contents.push('\n');
            }
            contents.push_str(&export_line);
            fs::write(&index_ts_path, contents)?;
            println!("Added export to: {}", index_ts_path.display());
        } else {
            println!(
                "Export for {} already exists in: {}",
                icon_alias,
                index_ts_path.display()
            );
        }
    } else {
        let mut file = fs::File::create(&index_ts_path)?;
        file.write_all(export_line.as_bytes())?;
        println!("Created and wrote export to: {}", index_ts_path.display());
    }

    Ok(())
}

/// Flutter preset add flow: write the SVG + regenerate (or create) the Dart
/// barrel file. iconmate owns the barrel entirely.
async fn run_app_flutter(
    config: AppConfig,
    raw_alias: String,
    collection_hint: Option<String>,
) -> anyhow::Result<()> {
    let folder_path = &config.folder;
    let folder_str = folder_path.to_string_lossy().replace('\\', "/");

    let barrel_path: PathBuf = config
        .flutter_barrel_file
        .clone()
        .unwrap_or_else(|| PathBuf::from(crate::flutter::DEFAULT_FLUTTER_BARREL_FILE));
    let barrel_class = config
        .flutter_barrel_class
        .clone()
        .unwrap_or_else(|| crate::flutter::DEFAULT_FLUTTER_BARREL_CLASS.to_string());

    // Resolve SVG content from the icon source. `--icon` is required.
    let Some(icon_source) = config.icon.as_ref() else {
        anyhow::bail!("The --icon argument is required for --preset flutter.");
    };
    let svg_content = _icon_source_to_svg(&Some(icon_source.clone()), None, false).await?;

    // Resolve SVG filename on disk. Prefer --filename, otherwise derive a
    // snake_case-ish stem from the icon source or name.
    let (file_stem, ext) = _make_svg_filename(
        config.filename.as_ref(),
        ".svg",
        config.icon.as_ref(),
        &raw_alias,
    );
    let file_name = format!("{}{}", file_stem, ext);
    let svg_file_path = folder_path.join(&file_name);

    if svg_file_path.exists() {
        anyhow::bail!(
            "Target icon file already exists: {}. Choose a different --filename.",
            svg_file_path.display()
        );
    }

    // Parse the existing barrel (or start empty) and resolve a unique Dart
    // identifier with the collision fallback.
    let existing_entries = crate::flutter::read_barrel_entries(&barrel_path)?;
    let fallback_name = collection_hint
        .as_deref()
        .map(|prefix| format!("{}{}", prefix, raw_alias));
    let identifier = crate::flutter::resolve_unique_identifier(
        &existing_entries,
        &raw_alias,
        fallback_name.as_deref(),
    )?;

    let asset_path = crate::flutter::asset_path_for(&folder_str, &file_name);
    let updated = crate::flutter::add_entry(&existing_entries, &identifier, &asset_path)?;

    // Write the SVG first, then the barrel. If the barrel write fails we roll
    // back the SVG so partial state doesn't leak.
    fs::write(&svg_file_path, &svg_content)?;
    println!("Successfully saved icon to: {}", svg_file_path.display());

    if let Err(err) = crate::flutter::write_barrel(&barrel_path, &barrel_class, &updated) {
        let _ = fs::remove_file(&svg_file_path);
        return Err(err);
    }

    println!(
        "Updated barrel at {}: added {}.{}",
        barrel_path.display(),
        barrel_class,
        identifier
    );

    if let Some(project) = crate::flutter::detect_flutter_project(
        &std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    ) {
        println!(
            "Flutter project detected ({}). Make sure `{}` is registered under `flutter: assets:` in pubspec.yaml at {}.",
            project.package_name.as_deref().unwrap_or("unknown"),
            folder_str,
            project.root.display()
        );
    }

    Ok(())
}

fn normalize_export_target(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_string()
}

fn validate_new_export_conflicts(
    index_contents: &str,
    rendered_export_statement: &str,
    index_path: &Path,
) -> anyhow::Result<()> {
    let Some(new_entry) = crate::utils::parse_export_line_ts(rendered_export_statement) else {
        return Ok(());
    };

    let new_target = normalize_export_target(&new_entry.file_path);
    for existing in collect_icons_from_index_contents(index_contents) {
        if existing.name == new_entry.name {
            anyhow::bail!(
                "Icon alias '{}' already exists in {}. Choose a different --name or rename the existing export.",
                new_entry.name,
                index_path.display()
            );
        }

        if normalize_export_target(&existing.file_path) == new_target {
            anyhow::bail!(
                "Export target '{}' already exists in {}. Choose a different --filename (or --name when filename is omitted).",
                new_entry.file_path,
                index_path.display()
            );
        }
    }

    Ok(())
}

/// Interactive mode: prompts the user for required values and builds an AppConfig.
async fn run_prompt_mode(cli: &CliArgs) -> anyhow::Result<()> {
    use inquire::{Select, Text, ui::RenderConfig};

    let render_config = RenderConfig::default().with_prompt_prefix(inquire::ui::Styled::new("●"));

    let folder_raw = match &cli.folder {
        Some(f) => {
            println!(">   Folder: {}", f.display());
            f.display().to_string()
        }
        None => Text::new("  Folder")
            .with_render_config(render_config.clone())
            .with_default("src/assets/icons/")
            .prompt()?,
    };
    let folder = PathBuf::from(folder_raw);

    let preset = match &cli.preset {
        Some(p) => {
            println!("> ✦ Preset: {}", p.to_str());
            Some(p.clone())
        }
        None => {
            let preset_opt = Select::new("✦ Preset", PRESETS_OPTIONS.to_vec())
                .with_render_config(render_config.clone())
                .prompt()?;
            Some(preset_opt.preset)
        }
    };

    let icon = match &cli.icon {
        Some(i) => {
            println!("> 🚀 Icon: {}", i);
            Some(i.clone())
        }
        None => {
            if matches!(preset, Some(Preset::EmptySvg)) {
                None
            } else {
                let icon_raw = Text::new(
                    "🚀 Icon (name like 'heroicons:heart' from https://icones.js.org, full URL, any SVG, or leave empty)\n",
                )
                .with_render_config(render_config.clone())
                .prompt()?;
                if icon_raw.is_empty() {
                    None
                } else {
                    Some(icon_raw)
                }
            }
        }
    };

    let filename = match &cli.filename {
        Some(f) => {
            println!(">  Filename: {}", f);
            Some(f.clone())
        }
        None => match _determine_icon_source_type(icon.as_ref()) {
            IconSourceType::None | IconSourceType::SvgContent => {
                let f = Text::new(" Filename (without extension like .svg, or leave empty)")
                    .with_render_config(render_config.clone())
                    .prompt()?;
                if f.is_empty() {
                    // Empty filename is allowed, will use the name instead
                    println!("  Filename left empty, will use the name as filename...");
                    None
                } else {
                    Some(f)
                }
            }
            _ => None,
        },
    };

    let inferred_name = icon
        .as_ref()
        .and_then(|icon_source| default_name_and_filename_from_icon_source(icon_source))
        .map(|(name, _)| name);

    let name: Option<String> = match &cli.name {
        Some(n) => {
            println!("> ✧ Name: {}", n);
            Some(n.clone())
        }
        None => {
            let mut prompt = Text::new("✧ Name (leave empty to auto-infer from icon)")
                .with_render_config(render_config);

            if let Some(default_name) = inferred_name.as_deref() {
                prompt = prompt.with_default(default_name);
            }

            let raw = prompt.prompt()?;
            if raw.trim().is_empty() {
                None
            } else {
                Some(raw)
            }
        }
    };

    let config = AppConfig {
        folder,
        name,
        icon,
        filename,
        preset,
        flutter_barrel_file: cli.flutter_barrel_file.clone(),
        flutter_barrel_class: cli.flutter_barrel_class.clone(),
    };
    run_app(config).await
}

impl std::fmt::Display for IconEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} — {}", self.name, self.file_path)
    }
}

fn collect_icons_from_index_contents(contents: &str) -> Vec<IconEntry> {
    let mut icons = Vec::new();

    for line in contents.lines() {
        for statement in line.split(';') {
            let statement = statement.trim();
            if statement.is_empty() {
                continue;
            }

            if let Some(icon_entry) = crate::utils::parse_export_line_ts(statement) {
                icons.push(icon_entry);
            }
        }
    }

    icons
}

#[cfg(test)]
fn remove_selected_exports_from_index(contents: &str, selected_icons: &[IconEntry]) -> String {
    use std::collections::HashSet;

    let selected = selected_icons
        .iter()
        .map(|icon| (icon.name.clone(), icon.file_path.clone()))
        .collect::<HashSet<_>>();

    let mut kept_lines = Vec::<String>::new();
    for line in contents.lines() {
        let mut parsed_export_in_line = false;

        for statement in line.split(';') {
            let statement = statement.trim();
            if statement.is_empty() {
                continue;
            }

            let Some(entry) = crate::utils::parse_export_line_ts(statement) else {
                continue;
            };

            parsed_export_in_line = true;
            if selected.contains(&(entry.name, entry.file_path)) {
                continue;
            }

            kept_lines.push(format!("{statement};"));
        }

        if !parsed_export_in_line {
            kept_lines.push(line.to_string());
        }
    }

    let mut updated = kept_lines.join("\n");
    if contents.ends_with('\n') {
        updated.push('\n');
    }

    updated
}

fn resolve_list_folder<'a>(
    cli: &'a CliArgs,
    command_folder: Option<&'a PathBuf>,
) -> Option<&'a PathBuf> {
    command_folder.or(cli.folder.as_ref())
}

fn run_list_mode(cli: &CliArgs, command_folder: Option<&PathBuf>) -> anyhow::Result<()> {
    let resolved = config::resolve_tui_config(
        resolve_list_folder(cli, command_folder),
        cli.preset.as_ref(),
    )?;

    let folder = PathBuf::from(&resolved.folder);

    if resolved.preset == "flutter" {
        let icons = crate::utils::get_existing_icons_for_preset(
            folder.to_string_lossy().as_ref(),
            &resolved.preset,
            resolved.flutter_barrel_file.as_deref(),
        )?;
        if icons.is_empty() {
            let barrel = resolved
                .flutter_barrel_file
                .unwrap_or_else(|| crate::flutter::DEFAULT_FLUTTER_BARREL_FILE.to_string());
            println!("No icons found in {}", barrel);
            return Ok(());
        }
        for icon in icons {
            println!("{}\t{}", icon.name, icon.file_path);
        }
        return Ok(());
    }

    let index_ts_path = folder.join("index.ts");
    if !index_ts_path.exists() {
        println!("No icons found in {}", index_ts_path.display());
        return Ok(());
    }

    let icons = crate::utils::get_existing_icons(folder.to_string_lossy().as_ref())?;
    if icons.is_empty() {
        println!("No icons found in {}", index_ts_path.display());
        return Ok(());
    }

    for icon in icons {
        println!("{}\t{}", icon.name, icon.file_path);
    }

    Ok(())
}

/// Interactive mode: deleting an icon from a select list of icons.
fn resolve_delete_folder<'a>(
    cli: &'a CliArgs,
    command_folder: Option<&'a PathBuf>,
) -> Option<&'a PathBuf> {
    command_folder.or(cli.folder.as_ref())
}

fn run_delete_flutter(
    folder: &Path,
    resolved: &config::ResolvedTuiConfig,
    names: &[String],
    filenames: &[String],
) -> anyhow::Result<()> {
    let barrel_path: PathBuf = resolved
        .flutter_barrel_file
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(crate::flutter::DEFAULT_FLUTTER_BARREL_FILE));
    let class = resolved
        .flutter_barrel_class
        .clone()
        .unwrap_or_else(|| crate::flutter::DEFAULT_FLUTTER_BARREL_CLASS.to_string());

    if !barrel_path.exists() {
        anyhow::bail!("No barrel file found at {}", barrel_path.display());
    }

    let entries = crate::flutter::read_barrel_entries(&barrel_path)?;
    let folder_str = folder.to_string_lossy().replace('\\', "/");
    let mut missing: Vec<String> = Vec::new();
    let mut to_remove: Vec<crate::flutter::DartBarrelEntry> = Vec::new();

    for name in names {
        match entries.iter().find(|e| &e.identifier == name) {
            Some(entry) => to_remove.push(entry.clone()),
            None => missing.push(format!("name={name}")),
        }
    }
    for filename in filenames {
        let needle_a = crate::flutter::asset_path_for(&folder_str, filename);
        let needle_b = filename.clone();
        match entries
            .iter()
            .find(|e| e.asset_path == needle_a || e.asset_path == needle_b)
        {
            Some(entry) => to_remove.push(entry.clone()),
            None => missing.push(format!("filename={filename}")),
        }
    }

    if !missing.is_empty() {
        anyhow::bail!("No matching icon(s) found for: {}", missing.join(", "));
    }

    to_remove.sort_by(|a, b| a.identifier.cmp(&b.identifier));
    to_remove.dedup_by(|a, b| a.identifier == b.identifier);

    let mut current = entries;
    for entry in &to_remove {
        let (updated, _) = crate::flutter::remove_entry_by_path(&current, &entry.asset_path);
        current = updated;

        // Also delete the SVG on disk if it resolves inside the configured folder.
        let asset_norm = entry.asset_path.replace('\\', "/");
        let rel = if !folder_str.is_empty() && asset_norm.starts_with(&format!("{folder_str}/")) {
            asset_norm[folder_str.len() + 1..].to_string()
        } else {
            asset_norm
        };
        let svg_abs = folder.join(&rel);
        if svg_abs.exists() {
            if let Err(e) = fs::remove_file(&svg_abs) {
                eprintln!("Failed to delete {}: {}", svg_abs.display(), e);
            } else {
                eprintln!("Deleted: {}", svg_abs.display());
            }
        }
    }

    crate::flutter::write_barrel(&barrel_path, &class, &current)?;
    eprintln!(
        "Updated barrel at {} ({} entr{} removed).",
        barrel_path.display(),
        to_remove.len(),
        if to_remove.len() == 1 { "y" } else { "ies" }
    );
    Ok(())
}

fn apply_deletions(
    folder: &Path,
    _index_ts_path: &Path,
    to_delete: &[IconEntry],
) -> anyhow::Result<()> {
    for icon in to_delete {
        let full_path = folder.join(&icon.file_path);
        crate::utils::delete_icon_entry(full_path.to_string_lossy().as_ref())?;
        eprintln!("Deleted: {}", full_path.display());
    }
    Ok(())
}

fn run_delete_non_interactive(
    cli: &CliArgs,
    command_folder: Option<&PathBuf>,
    names: &[String],
    filenames: &[String],
    yes: bool,
) -> anyhow::Result<()> {
    if !yes {
        anyhow::bail!(
            "Non-interactive delete requires --yes (-y) to confirm. Refusing to delete without explicit confirmation."
        );
    }

    let resolved = config::resolve_tui_config(
        resolve_delete_folder(cli, command_folder),
        cli.preset.as_ref(),
    )?;
    let folder = PathBuf::from(&resolved.folder);

    if resolved.preset == "flutter" {
        return run_delete_flutter(&folder, &resolved, names, filenames);
    }

    let index_ts_path = folder.join("index.ts");
    if !index_ts_path.exists() {
        anyhow::bail!(
            "No index.ts found in {}. Are you sure this is an icons folder?",
            folder.display()
        );
    }

    let contents = fs::read_to_string(&index_ts_path)?;
    let icons = collect_icons_from_index_contents(&contents);

    if icons.is_empty() {
        println!("No icons found in index.ts");
        return Ok(());
    }

    let mut to_delete: Vec<IconEntry> = Vec::new();
    let mut missing: Vec<String> = Vec::new();

    for name in names {
        let matches: Vec<&IconEntry> = icons.iter().filter(|i| &i.name == name).collect();
        match matches.len() {
            0 => missing.push(format!("name={name}")),
            1 => to_delete.push(matches[0].clone()),
            _ => anyhow::bail!(
                "Ambiguous --name '{name}': {} exports match. Use --filename to disambiguate.",
                matches.len()
            ),
        }
    }

    for filename in filenames {
        let matches: Vec<&IconEntry> = icons.iter().filter(|i| &i.file_path == filename).collect();
        match matches.len() {
            0 => missing.push(format!("filename={filename}")),
            1 => to_delete.push(matches[0].clone()),
            _ => anyhow::bail!(
                "Ambiguous --filename '{filename}': {} exports match.",
                matches.len()
            ),
        }
    }

    if !missing.is_empty() {
        anyhow::bail!("No matching icon(s) found for: {}", missing.join(", "));
    }

    // Deduplicate (a name and filename arg can resolve to the same entry).
    to_delete.sort_by(|a, b| a.file_path.cmp(&b.file_path));
    to_delete.dedup_by(|a, b| a.name == b.name && a.file_path == b.file_path);

    apply_deletions(&folder, &index_ts_path, &to_delete)
}

async fn run_delete_prompt_mode(
    cli: &CliArgs,
    command_folder: Option<&PathBuf>,
) -> anyhow::Result<()> {
    use inquire::{Confirm, MultiSelect, Text, ui::RenderConfig};

    let render_config = RenderConfig::default().with_prompt_prefix(inquire::ui::Styled::new("●"));

    // Step 1: Get the folder path
    let folder_raw = match resolve_delete_folder(cli, command_folder) {
        Some(f) => {
            println!(">   Folder: {}", f.display());
            f.display().to_string()
        }
        None => Text::new("  Folder")
            .with_render_config(render_config.clone())
            .with_default("src/assets/icons/")
            .prompt()?,
    };
    let folder = PathBuf::from(folder_raw);

    // Detect Flutter projects up-front; prompt-mode delete only supports
    // the JS preset path. Flutter users should use the TUI or pass
    // --name/--filename for non-interactive delete.
    let resolved = config::resolve_tui_config(Some(&folder), cli.preset.as_ref())?;
    if resolved.preset == "flutter" {
        anyhow::bail!(
            "Interactive delete for the Flutter preset isn't supported here. Use the TUI (just run `iconmate`) or pass --name / --filename with --yes."
        );
    }

    // Step 2: Check if folder is valid and has index.ts
    let index_ts_path = folder.join("index.ts");
    if !index_ts_path.exists() {
        anyhow::bail!(
            "No index.ts found in the specified folder. Are you sure this is an icons folder?"
        );
    }

    // Step 3: Read and parse index.ts
    let contents = fs::read_to_string(&index_ts_path)?;
    let icons = collect_icons_from_index_contents(&contents);

    if icons.is_empty() {
        println!("No icons found in index.ts");
        return Ok(());
    }

    // Step 5: Let user select which icons to delete
    let selected_icons = MultiSelect::new("🗑️  (Select icons to delete:", icons)
        .with_render_config(render_config.clone())
        .prompt()?;

    if selected_icons.is_empty() {
        println!("No icons selected for deletion.");
        return Ok(());
    }

    // Step 6: Confirm deletion
    let confirm = Confirm::new(&format!(
        "We will delete {} number of icons",
        selected_icons.len()
    ))
    .with_default(true)
    .with_render_config(render_config)
    .prompt()?;

    if !confirm {
        println!("Deletion cancelled.");
        return Ok(());
    }

    apply_deletions(&folder, &index_ts_path, &selected_icons)
}

fn run_sync_command(
    cli: &CliArgs,
    command_folder: Option<&PathBuf>,
    apply: bool,
    prune: bool,
    renames: &[String],
) -> anyhow::Result<()> {
    use std::collections::HashMap;

    if prune && !apply {
        anyhow::bail!("--prune requires --apply.");
    }

    let folder_override = command_folder.or(cli.folder.as_ref());
    let resolved = config::resolve_tui_config(folder_override, cli.preset.as_ref())?;
    let folder = PathBuf::from(&resolved.folder);

    let mut rename_map: HashMap<String, String> = HashMap::new();
    for raw in renames {
        let (old, new) = raw
            .split_once('=')
            .ok_or_else(|| anyhow::anyhow!("--rename expects `old=new`, got `{raw}`"))?;
        let old = old.trim();
        let new = new.trim();
        if old.is_empty() || new.is_empty() {
            anyhow::bail!("--rename expects a non-empty old and new identifier");
        }
        rename_map.insert(old.to_string(), new.to_string());
    }

    let flutter_barrel_file = resolved.flutter_barrel_file.as_deref().map(Path::new);
    let ctx = sync::SyncContext {
        folder: &folder,
        preset: &resolved.preset,
        flutter_barrel_file,
        flutter_barrel_class: resolved.flutter_barrel_class.as_deref(),
        renames: &rename_map,
    };

    let plan = sync::compute_sync_plan(&ctx)?;
    let use_color = std::io::IsTerminal::is_terminal(&std::io::stdout())
        && std::env::var_os("NO_COLOR").is_none();
    print!("{}", sync::render_plan_text(&plan, use_color));

    if !apply {
        if !plan.collisions.is_empty() {
            std::process::exit(1);
        }
        return Ok(());
    }

    if !plan.collisions.is_empty() {
        anyhow::bail!(
            "Cannot --apply: {} collision(s). Resolve with --rename or rename the SVG on disk.",
            plan.collisions.len()
        );
    }

    let summary = sync::apply_sync_plan(&plan, &ctx, sync::ApplyOptions { prune })?;
    println!(
        "\nApplied: +{} added, -{} removed.",
        summary.added, summary.removed
    );
    if !prune && !plan.removals.is_empty() {
        println!(
            "Note: {} orphan entr{} left in place. Re-run with --prune to remove them.",
            plan.removals.len(),
            if plan.removals.len() == 1 { "y" } else { "ies" }
        );
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = CliArgs::parse();

    match args.command {
        Some(Commands::Add {
            folder,
            icon,
            name,
            filename,
            preset,
            flutter_barrel_file,
            flutter_barrel_class,
        }) => {
            let resolved = config::resolve_tui_config(Some(&folder), preset.as_ref())?;
            let config = AppConfig {
                folder,
                icon,
                name,
                filename,
                preset: Some(Preset::from_str(&resolved.preset).ok_or_else(|| {
                    anyhow::anyhow!("Invalid resolved preset '{}'.", resolved.preset)
                })?),
                flutter_barrel_file: flutter_barrel_file
                    .or_else(|| resolved.flutter_barrel_file.map(PathBuf::from)),
                flutter_barrel_class: flutter_barrel_class.or(resolved.flutter_barrel_class),
            };
            run_app(config).await
        }
        Some(Commands::Tui {}) => run_prompt_mode(&args).await,
        Some(Commands::Delete {
            ref folder,
            ref names,
            ref filenames,
            yes,
        }) => {
            if !names.is_empty() || !filenames.is_empty() {
                run_delete_non_interactive(&args, folder.as_ref(), names, filenames, yes)
            } else {
                run_delete_prompt_mode(&args, folder.as_ref()).await
            }
        }
        Some(Commands::List { ref folder }) => run_list_mode(&args, folder.as_ref()),
        Some(Commands::Iconify { command }) => run_iconify_command(command).await,
        Some(Commands::Sync {
            ref folder,
            apply,
            prune,
            ref renames,
        }) => run_sync_command(&args, folder.as_ref(), apply, prune, renames),
        None => {
            let resolved = config::resolve_tui_config(args.folder.as_ref(), args.preset.as_ref())?;

            for warning in &resolved.warnings {
                eprintln!("Warning: {warning}");
            }
            for info in &resolved.info {
                eprintln!("{info}");
            }

            let config = app_state::AppConfig {
                folder: resolved.folder,
                preset: resolved.preset,
                svg_viewer_cmd: resolved.svg_viewer_cmd,
                svg_viewer_cmd_source: resolved.svg_viewer_cmd_source,
                global_config_loaded: resolved.global_config_loaded,
                project_config_loaded: resolved.project_config_loaded,
                flutter_barrel_file: resolved.flutter_barrel_file,
                flutter_barrel_class: resolved.flutter_barrel_class,
            };
            tui::run(config).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_selected_exports_removes_each_selected_line() {
        let contents = "export { default as IconOne } from './one.svg';\nexport { default as IconTwo } from './two.svg?react';\nexport { default as IconThree } from './three.svg';\n";

        let selected_icons = vec![
            crate::utils::parse_export_line_ts("export { default as IconOne } from './one.svg';")
                .expect("line should parse"),
            crate::utils::parse_export_line_ts(
                "export { default as IconTwo } from './two.svg?react';",
            )
            .expect("line should parse"),
        ];

        let updated = remove_selected_exports_from_index(contents, &selected_icons);

        assert!(!updated.contains("IconOne"));
        assert!(!updated.contains("IconTwo"));
        assert!(updated.contains("IconThree"));
    }

    #[test]
    fn collect_icons_reads_multiple_exports_on_same_line() {
        let contents = "export { default as IconOne } from './one.svg';export { default as IconTwo } from './two.svg';\n";

        let icons = collect_icons_from_index_contents(contents);

        assert_eq!(icons.len(), 2);
        assert!(icons.iter().any(|icon| icon.name == "IconOne"));
        assert!(icons.iter().any(|icon| icon.name == "IconTwo"));
    }

    #[test]
    fn remove_selected_exports_removes_selected_from_concatenated_line() {
        let contents = "export { default as IconOne } from './one.svg';export { default as IconTwo } from './two.svg';\n";

        let selected_icons = vec![
            crate::utils::parse_export_line_ts("export { default as IconTwo } from './two.svg';")
                .expect("line should parse"),
        ];

        let updated = remove_selected_exports_from_index(contents, &selected_icons);

        assert!(updated.contains("IconOne"));
        assert!(!updated.contains("IconTwo"));
    }

    #[test]
    fn resolve_delete_folder_prefers_subcommand_folder() {
        let cli_folder = PathBuf::from("src/assets/icons");
        let command_folder = PathBuf::from("icons/from/delete");
        let cli = CliArgs {
            command: None,
            folder: Some(cli_folder),
            preset: None,
            name: None,
            icon: None,
            filename: None,
            flutter_barrel_file: None,
            flutter_barrel_class: None,
        };

        let resolved = resolve_delete_folder(&cli, Some(&command_folder));
        assert_eq!(resolved, Some(&command_folder));
    }

    #[test]
    fn resolve_delete_folder_falls_back_to_global_folder() {
        let cli_folder = PathBuf::from("src/assets/icons");
        let cli = CliArgs {
            command: None,
            folder: Some(cli_folder.clone()),
            preset: None,
            name: None,
            icon: None,
            filename: None,
            flutter_barrel_file: None,
            flutter_barrel_class: None,
        };

        let resolved = resolve_delete_folder(&cli, None);
        assert_eq!(resolved, Some(&cli_folder));
    }

    #[test]
    fn resolve_list_folder_prefers_subcommand_folder() {
        let cli_folder = PathBuf::from("src/assets/icons");
        let command_folder = PathBuf::from("icons/from/list");
        let cli = CliArgs {
            command: None,
            folder: Some(cli_folder),
            preset: None,
            name: None,
            icon: None,
            filename: None,
            flutter_barrel_file: None,
            flutter_barrel_class: None,
        };

        let resolved = resolve_list_folder(&cli, Some(&command_folder));
        assert_eq!(resolved, Some(&command_folder));
    }

    #[test]
    fn resolve_list_folder_falls_back_to_global_folder() {
        let cli_folder = PathBuf::from("src/assets/icons");
        let cli = CliArgs {
            command: None,
            folder: Some(cli_folder.clone()),
            preset: None,
            name: None,
            icon: None,
            filename: None,
            flutter_barrel_file: None,
            flutter_barrel_class: None,
        };

        let resolved = resolve_list_folder(&cli, None);
        assert_eq!(resolved, Some(&cli_folder));
    }

    #[test]
    fn validate_new_export_conflicts_rejects_duplicate_alias() {
        let existing = "export { default as IconHeart } from './heart.svg';\n";
        let error = validate_new_export_conflicts(
            existing,
            "export { default as IconHeart } from './star.svg';",
            Path::new("src/assets/icons/index.ts"),
        )
        .expect_err("duplicate alias should fail");

        assert!(
            error
                .to_string()
                .contains("Icon alias 'IconHeart' already exists")
        );
    }

    #[test]
    fn validate_new_export_conflicts_rejects_duplicate_target() {
        let existing = "export { default as IconHeart } from './heart.svg';\n";
        let error = validate_new_export_conflicts(
            existing,
            "export { default as IconStar } from './heart.svg';",
            Path::new("src/assets/icons/index.ts"),
        )
        .expect_err("duplicate target should fail");

        assert!(
            error
                .to_string()
                .contains("Export target './heart.svg' already exists")
        );
    }

    #[test]
    fn validate_new_export_conflicts_allows_distinct_alias_and_target() {
        let existing = "export { default as IconHeart } from './heart.svg';\n";
        validate_new_export_conflicts(
            existing,
            "export { default as IconStar } from './star.svg';",
            Path::new("src/assets/icons/index.ts"),
        )
        .expect("distinct alias and target should be accepted");
    }
}
