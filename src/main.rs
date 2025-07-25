use clap::Parser;
use reqwest::Url;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// A CLI tool to fetch icons from icones.js.org and save them into your Vite, NextJS, or similar project..
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    /// Pathname of the folder where the icon will be saved and index.ts updated.
    #[arg(long)]
    folder: PathBuf,

    /// The name of the icon (e.g., "stash:chevron") or a full URL to the icon (e.g., "https://icones.js.org/collection/all?icon=stash:chevron").
    #[arg(long)]
    icon: String,

    /// The alias for the SVG, used in the index.ts export (e.g., "Chevron").
    #[arg(long)]
    name: String,

    /// Custom template for the export line. Use %name% for the icon alias and %icon% for the raw icon source.
    #[arg(
        long,
        default_value = "export { default as Icon%name% } from './%icon%.svg';"
    )]
    output_line_template: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = CliArgs::parse();

    let folder_path = &args.folder;
    let icon_source = &args.icon;
    let icon_alias = &args.name;

    // Ensure the folder exists
    fs::create_dir_all(folder_path)?;

    // Construct the full URL for the icon
    let icon_url = if icon_source.starts_with("http://") || icon_source.starts_with("https://") {
        Url::parse(icon_source)?
    } else {
        // Assume it's a shorthand like "stash:chevron"
        Url::parse(&format!("https://api.iconify.design/{}.svg", icon_source))?
    };

    println!("Fetching icon from: {}", icon_url);

    // Fetch the SVG content
    let client = reqwest::Client::new();
    let response = client.get(icon_url).send().await?.error_for_status()?;
    let svg_content = response.text().await?;

    // Determine the SVG file path
    let svg_file_name = format!("{}.svg", icon_source);
    let svg_file_path = folder_path.join(&svg_file_name);

    // Save the SVG content to the file
    fs::write(&svg_file_path, svg_content)?;
    println!("Successfully saved icon to: {}", svg_file_path.display());

    // Update or create index.ts
    let index_ts_path = folder_path.join("index.ts");
    let export_line = format!(
        "{}\n",
        args.output_line_template
            .replace("%name%", icon_alias)
            .replace("%icon%", icon_source)
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
