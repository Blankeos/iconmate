use clap::{Parser, Subcommand, ValueEnum};
use reqwest::Url;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[derive(ValueEnum, Clone, Debug)]
enum Preset {
    /// Use a blank SVG.
    #[value(name = "emptysvg")]
    EmptySvg,
}

/// A CLI tool to fetch icons and save them into your Vite, NextJS, or similar project.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Add an icon by specifying its details via command-line arguments.
    Add {
        /// Pathname of the folder where the icon will be saved and index.ts updated.
        #[arg(long)]
        folder: PathBuf,

        /// The alias for the SVG, used in the index.ts export (e.g., "Chevron").
        #[arg(long)]
        name: String,

        /// The name of the icon (e.g., "stash:chevron") or a full URL to the icon (e.g., "https://api.iconify.design/stash:chevron.svg").
        #[arg(long)]
        icon: Option<String>,

        /// Optional custom filename for the SVG file (without extension). Defaults to the icon name.
        #[arg(long)]
        filename: Option<String>,

        /// Custom template for the export line. Use %name% for the icon alias and %icon% for the filename stem.
        #[arg(
            long,
            default_value = "export { default as Icon%name% } from './%icon%.svg';"
        )]
        output_line_template: String,

        /// Optional preset to use instead of fetching an icon.
        #[arg(long)]
        preset: Option<Preset>,
    },
    /// Start an interactive prompt to add icons.
    Prompt {},
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

/// The main logic of the application.
/// Fetches an icon, saves it, and updates the index file.
async fn run_app(config: AppConfig) -> anyhow::Result<()> {
    let folder_path = &config.folder;
    let icon_alias = &config.name;

    // Ensure the folder exists
    fs::create_dir_all(folder_path)?;

    // Determine SVG content and filename stem based on a valid combination of arguments.
    let (svg_content, file_stem_str) = match (&config.icon, &config.preset) {
        // Case 1: Icon is provided AND the preset is EmptySvg. This is the only mutual exclusivity.
        (Some(_), Some(Preset::EmptySvg)) => {
            anyhow::bail!(
                "The --icon argument cannot be used with the --preset emptysvg. Please provide only one or the other."
            );
        }

        // Case 2: Only a preset is provided.
        (None, Some(preset)) => {
            let content = match preset {
                Preset::EmptySvg => {
                    println!("Using preset: emptysvg");
                    r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"></svg>"#.to_string()
                }
            };
            // Default filename to the alias (`--name`) if not specified.
            let stem = config
                .filename
                .clone()
                .unwrap_or_else(|| config.name.clone());
            Ok::<(String, String), anyhow::Error>((content, stem))
        }

        // Case 3: Only an icon is provided.
        (Some(icon_source), None) => {
            // Construct the full URL for the icon
            let icon_url =
                if icon_source.starts_with("http://") || icon_source.starts_with("https://") {
                    Url::parse(icon_source)?
                } else {
                    // Assume it's a shorthand like "stash:chevron"
                    Url::parse(&format!("https://api.iconify.design/{}.svg", icon_source))?
                };

            println!("Fetching icon from: {}", icon_url);

            // Fetch the SVG content
            let client = reqwest::Client::new();
            let response = client.get(icon_url).send().await?.error_for_status()?;
            let content = response.text().await?;

            // Use the custom filename if provided, otherwise default to the icon source string.
            let stem = config
                .filename
                .as_deref()
                .unwrap_or(icon_source)
                .to_string();
            Ok((content, stem))
        }

        // Case 4: Neither icon nor preset is provided.
        (None, None) => {
            anyhow::bail!("Either an --icon or a --preset must be provided.");
        }
    }?;

    // The rest of the function can now safely assume it has the content and a filename stem.
    let file_stem = &file_stem_str;
    let svg_file_name = format!("{}.svg", file_stem);
    let svg_file_path = folder_path.join(&svg_file_name);

    // Save the SVG content to the file
    fs::write(&svg_file_path, &svg_content)?;
    println!("Successfully saved icon to: {}", svg_file_path.display());

    // Update or create index.ts
    let index_ts_path = folder_path.join("index.ts");
    let export_line = format!(
        "{}\n",
        config
            .output_line_template
            .replace("%name%", icon_alias)
            .replace("%icon%", file_stem)
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
async fn run_prompt_mode() -> anyhow::Result<()> {
    use inquire::{Select, Text};

    let folder_raw = Text::new("  Folder")
        .with_default("src/assets/icons/")
        .prompt()?;
    let folder = PathBuf::from(folder_raw);

    let preset_opts = vec!["none", "emptysvg"];
    let preset_raw = Select::new("✦ Preset", preset_opts).prompt()?;
    let preset = match preset_raw {
        "emptysvg" => Some(Preset::EmptySvg),
        _ => None,
    };

    let icon = if preset.is_some() {
        None
    } else {
        let icon_raw = Text::new(
            "🚀 Icon (name like 'stash:chevron' from icones.js.org, full URL, or leave empty)",
        )
        .prompt()?;
        if icon_raw.is_empty() {
            None
        } else {
            Some(icon_raw)
        }
    };

    let filename = if icon.is_none() {
        let f = Text::new(" Filename (without .svg)").prompt()?;
        if f.is_empty() {
            anyhow::bail!("Filename is required when no icon is provided or emptysvg preset.");
        } else {
            Some(f)
        }
    } else {
        None
    };

    let name = Text::new("✧ Name (required, e.g., Chevron)")
        .with_validator(inquire::validator::ValueRequiredValidator::new(
            "Name is required.",
        ))
        .prompt()?;

    let config = AppConfig {
        folder,
        name,
        icon,
        filename,
        output_line_template: "export { default as Icon%name% } from './%icon%.svg';".to_string(),
        preset,
    };
    run_app(config).await
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
        Some(Commands::Prompt {}) | None => run_prompt_mode().await,
    }
}
