# Folder + Config System Plan

Goal: add a clean configuration system for both project-level defaults and user-level defaults, plus a generated JSON schema and typed TS definitions for editor tooling.

## Scope

- Add a local config file for per-project behavior.
- Add a global config file for user-wide behavior.
- Generate root JSON schema files from a `config-gen` package using Zod v4.
- Wire `svg_view_cmd` into TUI open behavior (`o`) for both local icons and Iconify search results.

## Config Surfaces

### Local Config (project-level)

- Filename: `iconmate.config.json` (in project root).
- Intended usage: checked into repo, shared with the team.
- Keys:
  - `folder` (local only)
  - `preset` (local only)
  - `output_line_template` (local only)
  - `svg_view_cmd` (local + global)

### Global Config (user-level)

- Intended usage: personal machine defaults.
- Initial key support:
  - `svg_view_cmd`
- Suggested paths (using OS conventions):
  - macOS: `~/Library/Application Support/iconmate/config.json`
  - Linux: `~/.config/iconmate/config.json`
  - Windows: `%APPDATA%\\iconmate\\config.json`

## Config Keys and Defaults

- `folder`
  - Scope: local only
  - Type: `string`
  - Default: `src/assets/icons`
- `preset`
  - Scope: local only
  - Type: `string`
  - Default: empty (`""`) which means plain SVG mode
- `svg_view_cmd`
  - Scope: local + global
  - Type: `string`
  - Default behavior:
    - macOS: open with Quick Look
    - Linux: open with system browser
    - Windows: open with default browser
  - Custom value examples:
    - `zed %filename%`
    - `code %filename%`
  - Token support:
    - `%filename%` = full path to the SVG file to open
- `output_line_template`
  - Scope: local only
  - Type: `string`
  - Default: `export { default as Icon%name% } from './%icon%%ext%';`
  - Variables:
    - `%name%`
    - `%icon%`
    - `%ext%`

## Precedence Rules

- For `folder`, `preset`, and `output_line_template`: CLI flag > local config > built-in default.
- For `svg_view_cmd`: CLI flag (if added later) > local config > global config > OS default.
- Unknown keys should be ignored with a warning (non-fatal).

## TUI Open Behavior (`o`)

- Local icon list:
  - pressing `o` opens the existing file via resolved `svg_view_cmd`.
- Iconify search results:
  - pressing `o` downloads the SVG to a cache file, then opens it via resolved `svg_view_cmd`.

## Cache Plan for Iconify Preview

- Use OS data directory with an `iconmate` namespace.
- Suggested cache root:
  - macOS: `~/Library/Application Support/iconmate/cache/iconify`
  - Linux: `~/.local/share/iconmate/cache/iconify`
  - Windows: `%LOCALAPPDATA%\\iconmate\\cache\\iconify`
- Cache filename strategy: `<prefix>__<icon>.svg` (sanitized for filesystem safety).

## `config-gen` Package Plan

- Create folder: `config-gen/`.
- Add `config-gen/package.json`.
- Use Zod v4 as source of truth for config schemas and descriptions.
- Define schemas in TypeScript and generate:
  - local/global config TS types
  - `iconmatelocal.schema.json` at repo root
  - `iconmateglobal.schema.json` at repo root

Suggested layout:

- `config-gen/src/schema.ts` (zod definitions + docs metadata)
- `config-gen/src/generate.ts` (writes root schema files)
- `iconmatelocal.schema.json` (generated, committed)
- `iconmateglobal.schema.json` (generated, committed)

## Documentation Requirements

- README should include:
  - local config filename and key meanings
  - global config purpose and location conventions
  - `svg_view_cmd` token usage (`%filename%`)
  - defaults for macOS/Linux/Windows behavior
  - note that `o` opens local and Iconify preview SVGs

## Milestones

1. Add `config-gen` scaffolding with Zod v4.
2. Define local/global schemas and TS types.
3. Generate and commit root schema files.
4. Load local/global config at app startup and apply precedence.
5. Implement `o` open flow for local and Iconify search entries.
6. Add docs and tests.

## Acceptance Criteria

- Local config supports `folder`, `preset`, `output_line_template`, and `svg_view_cmd`.
- Global config supports `svg_view_cmd`.
- Root schema files are generated from Zod schemas.
- `o` works for local icons and Iconify search results.
- Custom viewer command supports `%filename%` substitution.
- README clearly documents behavior and defaults.
