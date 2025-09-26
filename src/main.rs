use clap::{Parser, Subcommand, ValueEnum};
use reqwest::Url;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[derive(ValueEnum, Clone, Debug, PartialEq)]
enum Preset {
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

/// A CLI tool to fetch icons and save them into your Vite, NextJS, or similar project.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Pathname of the folder where the icon will be saved and index.ts updated.
    #[arg(long, global = true)]
    folder: Option<PathBuf>,

    /// Optional preset to use instead of fetching an icon.
    #[arg(long, global = true)]
    preset: Option<Preset>,

    /// The alias for the SVG, used in the index.ts export (e.g., "Chevron").
    #[arg(long, global = true)]
    name: Option<String>,

    /// The name of the icon (e.g., "stash:chevron") or a full URL to the icon (e.g., "https://api.iconify.design/stash:chevron.svg") or an SVG.
    #[arg(long, global = true)]
    icon: Option<String>,

    /// Optional custom filename for the SVG file (without extension). Defaults to the icon name.
    #[arg(long, global = true)]
    filename: Option<String>,

    /// Custom template for the export line. Use %name% for the icon alias and %icon% for the filename stem.
    /// Variables: %icon%, %name%
    /// Normally for complex usecases where for example you might need url suffixes for imports i.e. `?react`.
    #[arg(
        long,
        global = true,
        default_value = "export { default as Icon%name% } from './%icon%.svg';"
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
            default_value = "export { default as Icon%name% } from './%icon%.svg';"
        )]
        output_line_template: String,
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

/// Enum representing the type of icon source
#[derive(Debug, PartialEq)]
enum IconSourceType {
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
fn _determine_icon_source_type(icon_source: Option<&String>) -> IconSourceType {
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
async fn _icon_source_to_svg(
    icon_source: &Option<String>,
    append_attribute: Option<&'static str>,
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

    Ok(content)
}

/// Util: Reused in all cases, for appending the filename of svg, i.e. add .tsx or .svg or .svelte.
/// Returns a file_stem and an ext
fn _make_svg_filename(
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
    let (svg_content, file_stem_str, ext) = match (&config.icon, &config.preset) {
        // Case 1: Icon is provided AND the preset is EmptySvg. This is the only mutual exclusivity.
        (Some(_), Some(Preset::EmptySvg)) => {
            anyhow::bail!(
                "The --icon argument cannot be used with the --preset emptysvg. Please provide only one or the other."
            );
        }

        // Case 2: Only a preset is provided.
        (None, Some(Preset::EmptySvg)) => {
            let content = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"></svg>"#.to_string();
            let (file_stem, ext) = _make_svg_filename(
                config.filename.as_ref(),
                ".svg",
                config.icon.as_ref(),
                &config.name,
            );
            Ok::<(String, String, &'static str), anyhow::Error>((content, file_stem, ext))
        }

        // Case 3: React
        (icon_source, Some(Preset::React)) => {
            let content = _icon_source_to_svg(icon_source, Some("{...props}")).await?;

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
            Ok::<(String, String, &'static str), anyhow::Error>((content, file_stem, ext))
        }

        // Case 4: Svelte
        (icon_source, Some(Preset::Svelte)) => {
            let content = _icon_source_to_svg(icon_source, Some("{...props}")).await?;

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
            Ok::<(String, String, &'static str), anyhow::Error>((content, file_stem, ext))
        }

        // Case 5: Solid
        (icon_source, Some(Preset::Solid)) => {
            let content = _icon_source_to_svg(icon_source, Some("{...props}")).await?;

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
            Ok::<(String, String, &'static str), anyhow::Error>((content, file_stem, ext))
        }

        // Case 6: Vue
        (icon_source, Some(Preset::Vue)) => {
            let content = _icon_source_to_svg(icon_source, Some("v-bind=\"$props\"")).await?;

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
            Ok::<(String, String, &'static str), anyhow::Error>((content, file_stem, ext))
        }

        // Case 7: Only an icon is provided.
        (Some(icon_source), None) => {
            let content = _icon_source_to_svg(&Some(icon_source.clone()), None).await?;

            let (file_stem, ext) = _make_svg_filename(
                config.filename.as_ref(),
                ".svg",
                config.icon.as_ref(),
                &config.name,
            );
            Ok((content, file_stem, ext))
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
        config
            .output_line_template
            .replace("%name%", icon_alias)
            .replace("%icon%", &file_stem_str)
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
                "emptysvg" => Some(Preset::EmptySvg),
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
        Some(Commands::Prompt {}) => run_prompt_mode(&args).await,
        None => run_prompt_mode(&args).await,
    }
}
