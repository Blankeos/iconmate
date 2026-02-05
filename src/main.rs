mod app_state;
mod form_input;
mod iconify;
mod tui;
mod utils;
mod views;

use crate::iconify::{IconifyClient, IconifyCollectionResponse, IconifySearchResponse};
use crate::utils::{
    _determine_icon_source_type, _icon_source_to_svg, _make_svg_filename, IconEntry,
    IconSourceType, Preset,
};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

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

    /// Custom template for the export line. Use %name% for the icon alias and %icon% for the filename stem.
    /// Variables: %icon%, %name%
    /// Normally for complex usecases where for example you might need url suffixes for imports i.e. `?react`.
    #[arg(
        long,
        default_value = "export { default as Icon%name% } from './%icon%%ext%';"
    )]
    output_line_template: String,
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
        #[arg(long)]
        name: String,

        /// The name of the icon (e.g., "stash:chevron") or a full URL to the icon (e.g., "https://api.iconify.design/stash:chevron.svg") or an SVG.
        #[arg(long)]
        icon: Option<String>,

        /// Optional custom filename for the SVG file (without extension). Defaults to the icon name.
        #[arg(long)]
        filename: Option<String>,

        /// Custom template for the export line. Use %name% for the icon alias and %icon% for the filename stem.
        /// Variables: %icon%, %name%
        /// Normally for complex usecases where for example you might need url suffixes for imports i.e. `?react`.
        #[arg(
            long,
            default_value = "export { default as Icon%name% } from './%icon%%ext%';"
        )]
        output_line_template: String,
    },

    /// Start an interactive prompt to add icons.
    Tui {},

    /// Delete an icon from your collection of icons
    Delete {
        /// Pathname of the folder where all the icons are saved.
        #[arg(long)]
        folder: Option<PathBuf>,
    },

    /// Query Iconify collections, search results, and raw SVGs.
    Iconify {
        #[command(subcommand)]
        command: IconifyCommands,
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
    name: String,
    icon: Option<String>,
    filename: Option<String>,
    output_line_template: String,
    preset: Option<Preset>,
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

/// The main logic of the application.
/// Fetches an icon, saves it, and updates the index file.
async fn run_app(config: AppConfig) -> anyhow::Result<()> {
    let folder_path = &config.folder;
    let icon_alias = &config.name;

    // Ensure the folder exists
    fs::create_dir_all(folder_path)?;

    // Debug: print the current AppConfig
    // eprintln!("DEBUG: AppConfig {{");
    // eprintln!("  folder: {:?}", folder_path);
    // eprintln!("  name: {:?}", icon_alias);
    // eprintln!("  icon: {:?}", config.icon);
    // eprintln!("  filename: {:?}", config.filename);
    // eprintln!("  output_line_template: {:?}", config.output_line_template);
    // eprintln!("  preset: {:?}", config.preset);
    // eprintln!("}}");

    // Determine SVG content and filename stem based on a valid combination of arguments.
    let (svg_content, file_stem_str, ext, output_line_template) = match (
        &config.icon,
        &config.preset,
    ) {
        // Case 1: Icon is provided AND the preset is EmptySvg. This is the only mutual exclusivity.
        (Some(_), Some(Preset::Svg)) => {
            anyhow::bail!(
                "The --icon argument cannot be used with the --preset emptysvg. Please provide only one or the other."
            );
        }

        // Case 2: Only a preset is provided.
        (None, Some(Preset::Svg)) => {
            let content = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"></svg>"#.to_string();
            let (file_stem, ext) = _make_svg_filename(
                config.filename.as_ref(),
                ".svg",
                config.icon.as_ref(),
                &config.name,
            );
            Ok::<(String, String, &'static str, String), anyhow::Error>((
                content,
                file_stem,
                ext,
                config.output_line_template.clone(),
            ))
        }

        // Case 3: React
        (icon_source, Some(Preset::React)) => {
            let content = _icon_source_to_svg(icon_source, Some("{...props}"), true).await?;

            // Wrap the SVG in a React component template
            let content = format!(
                "import type {{ SVGProps }} from 'react';\n\nexport default function Icon(props: SVGProps<SVGSVGElement>) {{\n  return (\n{}\n  );\n}}",
                content
            );

            let (file_stem, ext) = _make_svg_filename(
                config.filename.as_ref(),
                ".tsx",
                config.icon.as_ref(),
                &config.name,
            );
            Ok::<(String, String, &'static str, String), anyhow::Error>((
                content,
                file_stem,
                ext,
                config.output_line_template.clone(),
            ))
        }

        // Case 4: Svelte
        (icon_source, Some(Preset::Svelte)) => {
            let content = _icon_source_to_svg(icon_source, Some("{...props}"), false).await?;

            // Wrap the SVG in a Svelte component template
            let content = format!(
                "<script lang=\"ts\">\n  import type {{ SVGAttributes }} from 'svelte/elements';\n\n  let {{ ...props }}: SVGAttributes<SVGSVGElement> = $props();\n</script>\n\n{}",
                content
            );

            let (file_stem, ext) = _make_svg_filename(
                config.filename.as_ref(),
                ".svelte",
                config.icon.as_ref(),
                &config.name,
            );
            Ok::<(String, String, &'static str, String), anyhow::Error>((
                content,
                file_stem,
                ext,
                config.output_line_template.clone(),
            ))
        }

        // Case 5: Solid
        (icon_source, Some(Preset::Solid)) => {
            let content = _icon_source_to_svg(icon_source, Some("{...props}"), true).await?;

            // Wrap the SVG in a Solid component template
            let content = format!(
                "import {{ type JSX }} from 'solid-js';\n\nexport default function Icon(props: JSX.SvgSVGAttributes<SVGSVGElement>) {{\n  return ({});\n}}",
                content
            );

            let (file_stem, ext) = _make_svg_filename(
                config.filename.as_ref(),
                ".tsx",
                config.icon.as_ref(),
                &config.name,
            );
            Ok::<(String, String, &'static str, String), anyhow::Error>((
                content,
                file_stem,
                ext,
                config.output_line_template.clone(),
            ))
        }

        // Case 6: Vue
        (icon_source, Some(Preset::Vue)) => {
            let content = _icon_source_to_svg(icon_source, Some("v-bind=\"$props\""), true).await?;

            // Wrap the SVG in a Vue component template
            let content = format!(
                "<template>\n  <template>\n    {}\n  </template>\n</template>\n\n<script setup lang=\"ts\">\nimport type {{ SVGAttributes }} from 'vue'\n\ndefineProps<SVGAttributes>()\n</script>",
                content
            );

            let (file_stem, ext) = _make_svg_filename(
                config.filename.as_ref(),
                ".vue",
                config.icon.as_ref(),
                &config.name,
            );
            Ok::<(String, String, &'static str, String), anyhow::Error>((
                content,
                file_stem,
                ext,
                config.output_line_template.clone(),
            ))
        }

        // Case 7: Only an icon is provided.
        (Some(icon_source), None) => {
            let content = _icon_source_to_svg(&Some(icon_source.clone()), None, false).await?;

            let (file_stem, ext) = _make_svg_filename(
                config.filename.as_ref(),
                ".svg",
                config.icon.as_ref(),
                &config.name,
            );
            Ok((content, file_stem, ext, config.output_line_template.clone()))
        }

        // Case 8: Neither icon nor preset is provided.
        (None, None) => {
            anyhow::bail!("Either an --icon or a --preset must be provided.");
        }
    }?;

    // The rest of the function can now safely assume it has the content and a filename stem.
    let svg_file_name = format!("{}{}", file_stem_str, ext);
    let svg_file_path = folder_path.join(&svg_file_name);

    // Save the SVG content to the file
    fs::write(&svg_file_path, &svg_content)?;
    println!("Successfully saved icon to: {}", svg_file_path.display());

    // Update or create index.ts
    let index_ts_path = folder_path.join("index.ts");
    let export_line = format!(
        "{}\n",
        output_line_template
            .replace("%name%", icon_alias)
            .replace("%icon%", &file_stem_str)
            .replace("%ext%", ext)
    );

    if index_ts_path.exists() {
        let mut contents = fs::read_to_string(&index_ts_path)?;
        if !contents.contains(&export_line) {
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
            println!("> ✦ Preset: emptysvg");
            Some(p.clone())
        }
        None => {
            #[derive(Debug, Copy, Clone)]
            struct PresetOpt {
                key: &'static str,
                desc: &'static str,
            }

            impl std::fmt::Display for PresetOpt {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "{} — {}", self.key, self.desc)
                }
            }

            let preset_opts = vec![
                PresetOpt {
                    key: "normal",
                    desc: "Plain svg (.svg)",
                },
                PresetOpt {
                    key: "emptysvg",
                    desc: "A blank SVG file (.svg)",
                },
                PresetOpt {
                    key: "react",
                    desc: "React Component (.tsx)",
                },
                PresetOpt {
                    key: "svelte",
                    desc: "Svelte Component (.svelte)",
                },
                PresetOpt {
                    key: "solid",
                    desc: "Solid Component (.tsx)",
                },
                PresetOpt {
                    key: "vue",
                    desc: "Vue Component (.vue)",
                },
            ];
            let preset_raw = Select::new("✦ Preset", preset_opts)
                .with_render_config(render_config.clone())
                .prompt()?;

            // My rust skill issue doesn't know how to return this as just 1 item.
            match preset_raw.key {
                "emptysvg" => Some(Preset::Svg),
                "react" => Some(Preset::React),
                "svelte" => Some(Preset::Svelte),
                "solid" => Some(Preset::Solid),
                "vue" => Some(Preset::Vue),
                _ => None,
            }
        }
    };

    let icon = match &cli.icon {
        Some(i) => {
            println!("> 🚀 Icon: {}", i);
            Some(i.clone())
        }
        None => {
            if matches!(preset, Some(Preset::Svg)) {
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

    let name = match &cli.name {
        Some(n) => {
            println!("> ✧ Name: {}", n);
            n.clone()
        }
        None => Text::new("✧ Name (required, e.g., Heart)")
            .with_render_config(render_config)
            .with_validator(inquire::validator::ValueRequiredValidator::new(
                "Name is required.",
            ))
            .prompt()?,
    };

    let config = AppConfig {
        folder,
        name,
        icon,
        filename,
        output_line_template: cli.output_line_template.clone(),
        preset,
    };
    run_app(config).await
}

impl std::fmt::Display for IconEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} — {}", self.name, self.file_path)
    }
}

/// Interactive mode: deleting an icon from a select list of icons.
async fn run_delete_prompt_mode(cli: &CliArgs) -> anyhow::Result<()> {
    use inquire::{Confirm, MultiSelect, Text, ui::RenderConfig};

    let render_config = RenderConfig::default().with_prompt_prefix(inquire::ui::Styled::new("●"));

    // Step 1: Get the folder path
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

    // Step 2: Check if folder is valid and has index.ts
    let index_ts_path = folder.join("index.ts");
    if !index_ts_path.exists() {
        anyhow::bail!(
            "No index.ts found in the specified folder. Are you sure this is an icons folder?"
        );
    }

    // Step 3: Read and parse index.ts
    let contents = fs::read_to_string(&index_ts_path)?;
    let mut icons = Vec::new();

    for line in contents.lines() {
        if let Some(icon_entry) = crate::utils::parse_export_line_ts(line) {
            icons.push(icon_entry);
        }
    }

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

    // Step 7: Delete the icons
    let mut updated_index_content = contents.clone();

    for icon_to_delete in &selected_icons {
        let full_path = folder.join(&icon_to_delete.file_path);

        // Delete the file
        if full_path.exists() {
            if let Err(e) = fs::remove_file(&full_path) {
                eprintln!("Failed to delete {}: {}", full_path.display(), e);
            } else {
                eprintln!("Deleted: {}", full_path.display());
            }
        } else {
            eprintln!("File not found: {}", full_path.display());
        }

        // Remove the export line from index.ts content
        if let Some(line_to_remove) = crate::utils::parse_export_line_ts(&contents) {
            if line_to_remove.name == icon_to_delete.name
                && line_to_remove.file_path == icon_to_delete.file_path
            {
                if let Some(line_start) = contents.find(&line_to_remove.name) {
                    // Find the start of the line
                    let mut start = line_start;
                    while start > 0 && contents.chars().nth(start - 1) != Some('\n') {
                        start -= 1;
                    }

                    // Find the end of the line
                    let mut end = start;
                    let chars: Vec<char> = contents.chars().collect();
                    while end < chars.len() && chars[end] != '\n' {
                        end += 1;
                    }
                    if end < chars.len() {
                        end += 1; // Include the newline
                    }

                    // Remove the line
                    let before = &updated_index_content[..start];
                    let after = &updated_index_content[end..];
                    updated_index_content = format!("{}{}", before, after);
                }
            }
        }
    }

    // Write the updated index.ts
    fs::write(&index_ts_path, updated_index_content)?;

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
            output_line_template,
            preset,
        }) => {
            let config = AppConfig {
                folder,
                icon,
                name,
                filename,
                output_line_template,
                preset,
            };
            run_app(config).await
        }
        Some(Commands::Tui {}) => run_prompt_mode(&args).await,
        Some(Commands::Delete { folder: _ }) => run_delete_prompt_mode(&args).await,
        Some(Commands::Iconify { command }) => run_iconify_command(command).await,
        None => {
            let config = app_state::AppConfig {
                folder: args
                    .folder
                    .unwrap_or_else(|| PathBuf::from("src/assets/icons"))
                    .display()
                    .to_string(),
                preset: match args.preset {
                    Some(p) => Some(format!("{:?}", p)),
                    None => None,
                },
                template: Some(args.output_line_template),
            };
            tui::run(config).await
        }
    }
}
