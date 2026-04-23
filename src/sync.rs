// `iconmate sync` — reconcile barrel file with icons on disk.
//
// Scope (v1): orphan files (SVGs on disk with no barrel entry) and orphan
// entries (barrel entries whose SVG is missing). Path drift detection is
// deferred (see `_docs/iconmate-sync-plan.md`).
//
// Safety: `compute_sync_plan` is pure / read-only. `apply_sync_plan` only ever
// writes to the barrel file (`index.ts` or the Dart barrel). SVG assets are
// never modified.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::flutter;
use crate::utils::{IconEntry, parse_export_line_ts};

#[derive(Debug, Clone, PartialEq)]
pub struct Addition {
    /// Folder-relative SVG filename, e.g. `heart.svg`.
    pub file_path: String,
    /// Final barrel identifier after template rendering (JS: e.g. `IconHeart`;
    /// Flutter: e.g. `heart`). Unique within the plan.
    pub identifier: String,
    /// For JS presets only: the full export line that would be appended.
    pub rendered_line: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Removal {
    pub identifier: String,
    /// As it appears in the barrel (JS: relative like `./heart.svg`; Flutter:
    /// folder-relative file path).
    pub file_path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Collision {
    pub file_path: String,
    pub inferred_identifier: String,
    pub conflicting_identifier: String,
}

#[derive(Debug, Clone)]
pub struct SyncPlan {
    pub preset: String,
    pub barrel_location: String,
    pub additions: Vec<Addition>,
    pub removals: Vec<Removal>,
    pub collisions: Vec<Collision>,
}

impl SyncPlan {
    pub fn is_clean(&self) -> bool {
        self.additions.is_empty() && self.removals.is_empty() && self.collisions.is_empty()
    }
}

pub struct SyncContext<'a> {
    pub folder: &'a Path,
    pub preset: &'a str,
    /// JS preset: template like `export { default as Icon%name% } from './%icon%%ext%';`.
    pub output_line_template: &'a str,
    /// Flutter preset: project-root-relative path to the barrel file.
    pub flutter_barrel_file: Option<&'a Path>,
    pub flutter_barrel_class: Option<&'a str>,
    /// User-provided identifier overrides. Keyed by the inferred identifier,
    /// value is the replacement to use instead.
    pub renames: &'a HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ApplyOptions {
    pub prune: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ApplySummary {
    pub added: usize,
    pub removed: usize,
}

/// Scan `folder` for flat `*.svg` files. Returns folder-relative names. We
/// intentionally do not recurse: iconmate writes to a flat folder.
pub fn find_svg_files(folder: &Path) -> anyhow::Result<Vec<String>> {
    if !folder.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(folder)
        .with_context(|| format!("Failed to read icons folder {}", folder.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.to_ascii_lowercase().ends_with(".svg") {
            out.push(name);
        }
    }
    out.sort();
    Ok(out)
}

/// Convert a filename stem (e.g. `chevron-right`) to PascalCase (`ChevronRight`).
fn pascal_case(input: &str) -> String {
    input
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut s = String::new();
                    s.extend(first.to_uppercase());
                    s.push_str(&chars.as_str().to_ascii_lowercase());
                    s
                }
                None => String::new(),
            }
        })
        .collect()
}

fn stem_of(filename: &str) -> (&str, &str) {
    match filename.rfind('.') {
        Some(idx) => (&filename[..idx], &filename[idx..]),
        None => (filename, ""),
    }
}

/// Render a JS export line for a given filename + alias using the template.
/// Returns (rendered_line, parsed_identifier).
fn render_js_addition(
    template: &str,
    filename: &str,
    alias: &str,
) -> Option<(String, String)> {
    let (stem, ext) = stem_of(filename);
    let rendered = template
        .replace("%name%", alias)
        .replace("%icon%", stem)
        .replace("%ext%", ext);
    let entry = parse_export_line_ts(rendered.trim_end_matches(';'))
        .or_else(|| parse_export_line_ts(&rendered))?;
    Some((rendered, entry.name))
}

/// Compare barrel paths case-insensitively and ignoring leading `./`.
fn normalize_js_path(value: &str) -> String {
    value
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_string()
}

pub fn compute_sync_plan(ctx: &SyncContext) -> anyhow::Result<SyncPlan> {
    if ctx.preset == "flutter" {
        compute_flutter_sync_plan(ctx)
    } else {
        compute_js_sync_plan(ctx)
    }
}

fn compute_js_sync_plan(ctx: &SyncContext) -> anyhow::Result<SyncPlan> {
    let barrel_path = ctx.folder.join("index.ts");
    let mut entries: Vec<IconEntry> = if barrel_path.exists() {
        let contents = fs::read_to_string(&barrel_path)?;
        let mut out = Vec::new();
        for line in contents.lines() {
            for stmt in line.split(';') {
                let stmt = stmt.trim();
                if stmt.is_empty() {
                    continue;
                }
                if let Some(entry) = parse_export_line_ts(stmt) {
                    out.push(entry);
                }
            }
        }
        out
    } else {
        Vec::new()
    };

    let svgs_on_disk = find_svg_files(ctx.folder)?;
    let disk_set: HashSet<&str> = svgs_on_disk.iter().map(|s| s.as_str()).collect();

    // Existing barrel entries, indexed by the final path component. iconmate
    // writes flat folders so basename matching is sufficient.
    let basename_of = |value: &str| -> String {
        value
            .replace('\\', "/")
            .rsplit('/')
            .next()
            .unwrap_or("")
            .to_string()
    };
    let mut barrel_paths: HashMap<String, usize> = HashMap::new();
    for (i, e) in entries.iter().enumerate() {
        barrel_paths.insert(basename_of(&e.file_path), i);
    }

    let existing_identifiers: HashSet<String> =
        entries.iter().map(|e| e.name.clone()).collect();

    let mut additions = Vec::new();
    let mut collisions = Vec::new();

    for filename in &svgs_on_disk {
        if barrel_paths.contains_key(filename) {
            continue;
        }
        let (stem, _) = stem_of(filename);
        let inferred_alias = pascal_case(stem);
        if inferred_alias.is_empty() {
            continue;
        }
        let Some((rendered, full_name)) =
            render_js_addition(ctx.output_line_template, filename, &inferred_alias)
        else {
            continue;
        };

        let final_name = ctx.renames.get(&full_name).cloned().unwrap_or(full_name.clone());
        if final_name != full_name {
            // Rerender with the overridden alias. We need to figure out what
            // `%name%` value would produce `final_name`. Simplest approach:
            // substitute the alias token directly — the user is overriding the
            // whole parsed name, not the raw %name% input. We reconstruct the
            // line by replacing the identifier in the rendered output.
            let rerendered = rendered.replacen(&full_name, &final_name, 1);
            if existing_identifiers.contains(&final_name) {
                collisions.push(Collision {
                    file_path: format!("./{filename}"),
                    inferred_identifier: full_name,
                    conflicting_identifier: final_name,
                });
                continue;
            }
            additions.push(Addition {
                file_path: format!("./{filename}"),
                identifier: final_name,
                rendered_line: Some(rerendered),
            });
            continue;
        }

        if existing_identifiers.contains(&full_name) {
            collisions.push(Collision {
                file_path: format!("./{filename}"),
                inferred_identifier: full_name.clone(),
                conflicting_identifier: full_name,
            });
            continue;
        }

        additions.push(Addition {
            file_path: format!("./{filename}"),
            identifier: full_name,
            rendered_line: Some(rendered),
        });
    }

    let mut removals = Vec::new();
    entries.sort_by(|a, b| a.file_path.cmp(&b.file_path));
    for entry in &entries {
        let normalized = basename_of(&entry.file_path);
        if !disk_set.contains(normalized.as_str()) {
            removals.push(Removal {
                identifier: entry.name.clone(),
                file_path: entry.file_path.clone(),
            });
        }
    }

    Ok(SyncPlan {
        preset: ctx.preset.to_string(),
        barrel_location: barrel_path.display().to_string(),
        additions,
        removals,
        collisions,
    })
}

fn compute_flutter_sync_plan(ctx: &SyncContext) -> anyhow::Result<SyncPlan> {
    let barrel_path: PathBuf = ctx
        .flutter_barrel_file
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(flutter::DEFAULT_FLUTTER_BARREL_FILE));

    let entries = flutter::read_barrel_entries(&barrel_path)?;

    let svgs_on_disk = find_svg_files(ctx.folder)?;
    let disk_set: HashSet<&str> = svgs_on_disk.iter().map(|s| s.as_str()).collect();

    // iconmate writes flat folders, so match barrel entries to disk files by
    // the final path component. This avoids brittle prefix comparison between
    // absolute/relative folder strings.
    let basename_of = |asset_path: &str| -> String {
        asset_path
            .replace('\\', "/")
            .rsplit('/')
            .next()
            .unwrap_or("")
            .to_string()
    };

    let barrel_basenames: HashSet<String> =
        entries.iter().map(|e| basename_of(&e.asset_path)).collect();

    let existing_ids: HashSet<String> =
        entries.iter().map(|e| e.identifier.clone()).collect();

    let mut additions = Vec::new();
    let mut collisions = Vec::new();

    for filename in &svgs_on_disk {
        if barrel_basenames.contains(filename) {
            continue;
        }
        let (stem, _) = stem_of(filename);
        let inferred = match flutter::sanitize_identifier(stem) {
            Ok(id) => id,
            Err(_) => continue,
        };
        let final_id = ctx
            .renames
            .get(&inferred)
            .cloned()
            .unwrap_or_else(|| inferred.clone());

        if existing_ids.contains(&final_id) {
            collisions.push(Collision {
                file_path: filename.clone(),
                inferred_identifier: inferred,
                conflicting_identifier: final_id,
            });
            continue;
        }

        additions.push(Addition {
            file_path: filename.clone(),
            identifier: final_id,
            rendered_line: None,
        });
    }

    let mut removals = Vec::new();
    for entry in &entries {
        let basename = basename_of(&entry.asset_path);
        if !disk_set.contains(basename.as_str()) {
            removals.push(Removal {
                identifier: entry.identifier.clone(),
                file_path: basename,
            });
        }
    }

    Ok(SyncPlan {
        preset: ctx.preset.to_string(),
        barrel_location: barrel_path.display().to_string(),
        additions,
        removals,
        collisions,
    })
}

pub fn apply_sync_plan(
    plan: &SyncPlan,
    ctx: &SyncContext,
    options: ApplyOptions,
) -> anyhow::Result<ApplySummary> {
    if !plan.collisions.is_empty() {
        anyhow::bail!(
            "Refusing to apply: {} collision(s) must be resolved via --rename or by renaming the SVG on disk.",
            plan.collisions.len()
        );
    }

    if plan.preset == "flutter" {
        apply_flutter(plan, ctx, options)
    } else {
        apply_js(plan, ctx, options)
    }
}

fn apply_js(
    plan: &SyncPlan,
    ctx: &SyncContext,
    options: ApplyOptions,
) -> anyhow::Result<ApplySummary> {
    let barrel_path = ctx.folder.join("index.ts");

    let mut contents = if barrel_path.exists() {
        fs::read_to_string(&barrel_path)?
    } else {
        String::new()
    };

    let mut summary = ApplySummary::default();

    if options.prune && !plan.removals.is_empty() {
        let to_remove: Vec<IconEntry> = plan
            .removals
            .iter()
            .map(|r| IconEntry {
                name: r.identifier.clone(),
                file_path: r.file_path.clone(),
            })
            .collect();
        contents = remove_js_exports(&contents, &to_remove);
        summary.removed = plan.removals.len();
    }

    if !plan.additions.is_empty() {
        if !contents.is_empty() && !contents.ends_with('\n') {
            contents.push('\n');
        }
        for addition in &plan.additions {
            if let Some(line) = &addition.rendered_line {
                contents.push_str(line.trim_end());
                contents.push('\n');
            }
        }
        summary.added = plan.additions.len();
    }

    if summary.added > 0 || summary.removed > 0 {
        if let Some(parent) = barrel_path.parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(&barrel_path, contents)?;
    }

    Ok(summary)
}

fn remove_js_exports(contents: &str, to_remove: &[IconEntry]) -> String {
    let remove_set: HashSet<(String, String)> = to_remove
        .iter()
        .map(|e| (e.name.clone(), normalize_js_path(&e.file_path)))
        .collect();

    let mut kept: Vec<String> = Vec::new();
    for line in contents.lines() {
        let mut parsed_any = false;
        for stmt in line.split(';') {
            let stmt = stmt.trim();
            if stmt.is_empty() {
                continue;
            }
            let Some(entry) = parse_export_line_ts(stmt) else {
                continue;
            };
            parsed_any = true;
            let key = (entry.name, normalize_js_path(&entry.file_path));
            if remove_set.contains(&key) {
                continue;
            }
            kept.push(format!("{stmt};"));
        }
        if !parsed_any {
            kept.push(line.to_string());
        }
    }
    let mut out = kept.join("\n");
    if contents.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn apply_flutter(
    plan: &SyncPlan,
    ctx: &SyncContext,
    options: ApplyOptions,
) -> anyhow::Result<ApplySummary> {
    let barrel_path: PathBuf = ctx
        .flutter_barrel_file
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(flutter::DEFAULT_FLUTTER_BARREL_FILE));
    let class = ctx
        .flutter_barrel_class
        .unwrap_or(flutter::DEFAULT_FLUTTER_BARREL_CLASS);

    let mut entries = flutter::read_barrel_entries(&barrel_path)?;
    let folder_str = ctx.folder.to_string_lossy().replace('\\', "/");

    let mut summary = ApplySummary::default();

    if options.prune && !plan.removals.is_empty() {
        let remove_ids: HashSet<&str> =
            plan.removals.iter().map(|r| r.identifier.as_str()).collect();
        entries.retain(|e| !remove_ids.contains(e.identifier.as_str()));
        summary.removed = plan.removals.len();
    }

    for addition in &plan.additions {
        let asset_path = flutter::asset_path_for(&folder_str, &addition.file_path);
        entries.push(flutter::DartBarrelEntry {
            identifier: addition.identifier.clone(),
            asset_path,
        });
        summary.added += 1;
    }

    if summary.added > 0 || summary.removed > 0 {
        flutter::write_barrel(&barrel_path, class, &entries)?;
    }

    Ok(summary)
}

/// Render the plan as text. When `use_color`, emits ANSI escapes for
/// additions (green), removals (red), and collisions (yellow). Stable,
/// LLM-friendly format either way.
pub fn render_plan_text(plan: &SyncPlan, use_color: bool) -> String {
    let green = if use_color { "\x1b[32m" } else { "" };
    let red = if use_color { "\x1b[31m" } else { "" };
    let yellow = if use_color { "\x1b[33m" } else { "" };
    let bold = if use_color { "\x1b[1m" } else { "" };
    let reset = if use_color { "\x1b[0m" } else { "" };

    let mut out = String::new();
    out.push_str(&format!("Barrel: {}\n", plan.barrel_location));
    out.push_str(&format!("Preset: {}\n\n", plan.preset));

    if plan.is_clean() {
        out.push_str(&format!("{green}{bold}Clean — no drift.{reset}\n"));
        return out;
    }

    if !plan.additions.is_empty() {
        out.push_str(&format!(
            "{green}{bold}Would add ({}):{reset}\n",
            plan.additions.len()
        ));
        for a in &plan.additions {
            out.push_str(&format!(
                "  {green}+ {:<24} → {}{reset}  (orphan file)\n",
                a.identifier, a.file_path
            ));
        }
        out.push('\n');
    }

    if !plan.removals.is_empty() {
        out.push_str(&format!(
            "{red}{bold}Would prune ({}):{reset}\n",
            plan.removals.len()
        ));
        for r in &plan.removals {
            out.push_str(&format!(
                "  {red}- {:<24} → {}{reset}  (file missing)\n",
                r.identifier, r.file_path
            ));
        }
        out.push('\n');
    }

    if !plan.collisions.is_empty() {
        out.push_str(&format!(
            "{yellow}{bold}Collisions ({}):{reset}\n",
            plan.collisions.len()
        ));
        for c in &plan.collisions {
            out.push_str(&format!(
                "  {yellow}! {} would collide with existing `{}`{reset} (from {})\n",
                c.inferred_identifier, c.conflicting_identifier, c.file_path
            ));
        }
        out.push('\n');
    }

    if !plan.additions.is_empty() || !plan.removals.is_empty() {
        out.push_str("Run with --apply to write additions.\n");
        if !plan.removals.is_empty() {
            out.push_str("Run with --apply --prune to also remove orphan entries.\n");
        }
    }
    if !plan.collisions.is_empty() {
        out.push_str("Resolve collisions with --rename <inferred>=<newName>, or rename the SVG on disk.\n");
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn write_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    const DEFAULT_TEMPLATE: &str =
        "export { default as Icon%name% } from './%icon%%ext%';";

    #[test]
    fn js_clean_when_in_sync() {
        let tmp = TempDir::new().unwrap();
        let folder = tmp.path();
        write_file(&folder.join("heart.svg"), "<svg></svg>");
        write_file(
            &folder.join("index.ts"),
            "export { default as IconHeart } from './heart.svg';\n",
        );

        let renames = HashMap::new();
        let ctx = SyncContext {
            folder,
            preset: "react",
            output_line_template: DEFAULT_TEMPLATE,
            flutter_barrel_file: None,
            flutter_barrel_class: None,
            renames: &renames,
        };
        let plan = compute_sync_plan(&ctx).unwrap();
        assert!(plan.is_clean(), "{}", render_plan_text(&plan, false));
    }

    #[test]
    fn js_detects_orphan_file_as_addition() {
        let tmp = TempDir::new().unwrap();
        let folder = tmp.path();
        write_file(&folder.join("heart.svg"), "<svg></svg>");
        write_file(&folder.join("star.svg"), "<svg></svg>");
        write_file(
            &folder.join("index.ts"),
            "export { default as IconHeart } from './heart.svg';\n",
        );

        let renames = HashMap::new();
        let ctx = SyncContext {
            folder,
            preset: "react",
            output_line_template: DEFAULT_TEMPLATE,
            flutter_barrel_file: None,
            flutter_barrel_class: None,
            renames: &renames,
        };
        let plan = compute_sync_plan(&ctx).unwrap();
        assert_eq!(plan.additions.len(), 1);
        assert_eq!(plan.additions[0].identifier, "IconStar");
        assert_eq!(plan.additions[0].file_path, "./star.svg");
        assert!(plan.removals.is_empty());
    }

    #[test]
    fn js_detects_orphan_entry_as_removal() {
        let tmp = TempDir::new().unwrap();
        let folder = tmp.path();
        // No heart.svg on disk.
        write_file(
            &folder.join("index.ts"),
            "export { default as IconHeart } from './heart.svg';\n",
        );

        let renames = HashMap::new();
        let ctx = SyncContext {
            folder,
            preset: "react",
            output_line_template: DEFAULT_TEMPLATE,
            flutter_barrel_file: None,
            flutter_barrel_class: None,
            renames: &renames,
        };
        let plan = compute_sync_plan(&ctx).unwrap();
        assert_eq!(plan.removals.len(), 1);
        assert_eq!(plan.removals[0].identifier, "IconHeart");
    }

    #[test]
    fn js_detects_collision() {
        let tmp = TempDir::new().unwrap();
        let folder = tmp.path();
        // Barrel already exports `IconHeart` pointing at a non-existent file.
        // A new `heart.svg` on disk infers the same alias → collision.
        write_file(&folder.join("heart.svg"), "<svg></svg>");
        write_file(
            &folder.join("index.ts"),
            "export { default as IconHeart } from './heart-old.svg';\n",
        );

        let renames = HashMap::new();
        let ctx = SyncContext {
            folder,
            preset: "react",
            output_line_template: DEFAULT_TEMPLATE,
            flutter_barrel_file: None,
            flutter_barrel_class: None,
            renames: &renames,
        };
        let plan = compute_sync_plan(&ctx).unwrap();
        assert_eq!(plan.collisions.len(), 1, "plan: {}", render_plan_text(&plan, false));
        assert_eq!(plan.collisions[0].inferred_identifier, "IconHeart");
    }

    #[test]
    fn js_rename_resolves_collision() {
        let tmp = TempDir::new().unwrap();
        let folder = tmp.path();
        write_file(&folder.join("heart.svg"), "<svg></svg>");
        write_file(
            &folder.join("index.ts"),
            "export { default as IconHeart } from './heart-old.svg';\n",
        );

        let mut renames = HashMap::new();
        renames.insert("IconHeart".to_string(), "IconHeart2".to_string());
        let ctx = SyncContext {
            folder,
            preset: "react",
            output_line_template: DEFAULT_TEMPLATE,
            flutter_barrel_file: None,
            flutter_barrel_class: None,
            renames: &renames,
        };
        let plan = compute_sync_plan(&ctx).unwrap();
        assert!(plan.collisions.is_empty(), "plan: {}", render_plan_text(&plan, false));
        assert_eq!(plan.additions.len(), 1);
        assert_eq!(plan.additions[0].identifier, "IconHeart2");
        assert!(
            plan
                .additions[0]
                .rendered_line
                .as_ref()
                .unwrap()
                .contains("IconHeart2")
        );
    }

    #[test]
    fn js_apply_adds_orphan_file_to_barrel() {
        let tmp = TempDir::new().unwrap();
        let folder = tmp.path();
        write_file(&folder.join("heart.svg"), "<svg></svg>");
        write_file(&folder.join("star.svg"), "<svg></svg>");
        write_file(
            &folder.join("index.ts"),
            "export { default as IconHeart } from './heart.svg';\n",
        );

        let renames = HashMap::new();
        let ctx = SyncContext {
            folder,
            preset: "react",
            output_line_template: DEFAULT_TEMPLATE,
            flutter_barrel_file: None,
            flutter_barrel_class: None,
            renames: &renames,
        };
        let plan = compute_sync_plan(&ctx).unwrap();
        let summary = apply_sync_plan(&plan, &ctx, ApplyOptions::default()).unwrap();
        assert_eq!(summary.added, 1);
        assert_eq!(summary.removed, 0);

        let updated = fs::read_to_string(folder.join("index.ts")).unwrap();
        assert!(updated.contains("IconStar"));
        assert!(updated.contains("./star.svg"));
    }

    #[test]
    fn js_apply_requires_prune_to_remove() {
        let tmp = TempDir::new().unwrap();
        let folder = tmp.path();
        write_file(
            &folder.join("index.ts"),
            "export { default as IconHeart } from './heart.svg';\n",
        );

        let renames = HashMap::new();
        let ctx = SyncContext {
            folder,
            preset: "react",
            output_line_template: DEFAULT_TEMPLATE,
            flutter_barrel_file: None,
            flutter_barrel_class: None,
            renames: &renames,
        };
        let plan = compute_sync_plan(&ctx).unwrap();

        // Without --prune: orphan entry remains.
        let summary = apply_sync_plan(&plan, &ctx, ApplyOptions::default()).unwrap();
        assert_eq!(summary.removed, 0);
        let still = fs::read_to_string(folder.join("index.ts")).unwrap();
        assert!(still.contains("IconHeart"));

        // With --prune: removed.
        let summary = apply_sync_plan(&plan, &ctx, ApplyOptions { prune: true }).unwrap();
        assert_eq!(summary.removed, 1);
        let pruned = fs::read_to_string(folder.join("index.ts")).unwrap();
        assert!(!pruned.contains("IconHeart"));
    }

    #[test]
    fn flutter_detects_orphan_file_and_entry() {
        let tmp = TempDir::new().unwrap();
        let folder = tmp.path().join("assets/icons");
        let barrel = tmp.path().join("lib/icons.dart");
        write_file(&folder.join("heart.svg"), "<svg></svg>");
        write_file(&folder.join("star.svg"), "<svg></svg>");
        // Barrel references only heart + a missing `ghost.svg`.
        write_file(
            &barrel,
            "// GENERATED by iconmate — do not edit by hand.\n\
             class AppIcons {\n\
             \tAppIcons._();\n\n\
             \tstatic const String heart = 'assets/icons/heart.svg';\n\
             \tstatic const String ghost = 'assets/icons/ghost.svg';\n\
             }\n",
        );

        let renames = HashMap::new();
        let ctx = SyncContext {
            folder: &folder,
            preset: "flutter",
            output_line_template: "",
            flutter_barrel_file: Some(&barrel),
            flutter_barrel_class: Some("AppIcons"),
            renames: &renames,
        };
        let plan = compute_sync_plan(&ctx).unwrap();
        assert_eq!(plan.additions.len(), 1);
        assert_eq!(plan.additions[0].identifier, "star");
        assert_eq!(plan.removals.len(), 1);
        assert_eq!(plan.removals[0].identifier, "ghost");
    }

    #[test]
    fn flutter_apply_writes_barrel() {
        let tmp = TempDir::new().unwrap();
        let folder = tmp.path().join("assets/icons");
        let barrel = tmp.path().join("lib/icons.dart");
        write_file(&folder.join("heart.svg"), "<svg></svg>");
        write_file(&folder.join("star.svg"), "<svg></svg>");
        write_file(
            &barrel,
            "// GENERATED by iconmate — do not edit by hand.\n\
             class AppIcons {\n\
             \tAppIcons._();\n\n\
             \tstatic const String heart = 'assets/icons/heart.svg';\n\
             }\n",
        );

        let renames = HashMap::new();
        let ctx = SyncContext {
            folder: &folder,
            preset: "flutter",
            output_line_template: "",
            flutter_barrel_file: Some(&barrel),
            flutter_barrel_class: Some("AppIcons"),
            renames: &renames,
        };
        let plan = compute_sync_plan(&ctx).unwrap();
        let summary = apply_sync_plan(&plan, &ctx, ApplyOptions::default()).unwrap();
        assert_eq!(summary.added, 1);

        let updated = fs::read_to_string(&barrel).unwrap();
        assert!(updated.contains("static const String star"));
        assert!(updated.contains("assets/icons/star.svg"));
    }

    #[test]
    fn render_plan_text_clean() {
        let plan = SyncPlan {
            preset: "react".into(),
            barrel_location: "foo/index.ts".into(),
            additions: vec![],
            removals: vec![],
            collisions: vec![],
        };
        let text = render_plan_text(&plan, false);
        assert!(text.contains("Clean"));
    }
}
