# Iconify CLI + API Plan

Goal: ship first-class Iconify commands in `iconmate` with a reusable API client and AI-friendly output defaults.

## Scope

- Keep command group: `iconmate iconify ...`.
- Keep shared API module: `src/iconify.rs`.
- Keep existing Iconify SVG fetches routed through shared client.
- Keep API models and errors presentation-agnostic (no CLI formatting in API layer).
- Update endpoint contracts to match real Iconify responses.

## Reality Check from API Probes

- Base URL: `https://api.iconify.design`
- `GET /collections` returns a top-level object keyed by prefix (not wrapped in `collections`).
- `GET /collections?prefix=mdi` works as an exact-prefix filter.
- `GET /collections?prefix=m` returns empty (no fuzzy prefix matching).
- `GET /collection?prefix=mdi` does not return an `icons` field; it returns `uncategorized`, `categories`, `aliases`, and `hidden`.
- `GET /{prefix}:{icon}.svg` returns SVG payload.
- `GET /{prefix}.json?icons={icon}` returns icon JSON payload.

## CLI Surface

### Commands

- `iconmate iconify search <query>`
- `iconmate iconify collections`
- `iconmate iconify collection <prefix>`
- `iconmate iconify get <prefix:icon>`

### Flags

- `search`
  - `--limit <n>`
  - `--start <n>`
  - `--format <text|json>` (default: `text`)
  - `--include-collections` (JSON mode only)
- `collections`
  - `--format <text|json>` (default: `text`)
  - `--prefix <exact_prefix>` (optional exact match only)
- `collection`
  - `--format <text|json>` (default: `text`)
  - `--raw` (optional: return unmodified API shape in JSON mode)
- `get`
  - `--format <svg|json>` (default: `svg`)

## Architecture

- Keep `IconifyClient` with configurable base URL.
- Keep typed methods:
  - `collections(prefix: Option<&str>)`
  - `collection(prefix)`
  - `search(query, limit, start, include_collections)`
  - `svg(prefix_icon)`
  - `icon_json(prefix, icon)`
- Add collection normalization helper in API layer:
  - flatten `uncategorized` + category icon lists into one deduped icon list
  - optionally expose raw categories/aliases/hidden in raw mode

## Data Model Draft

- `IconifySearchResponse`
  - `icons: Vec<String>`
  - `total: u32`
  - `limit: u32`
  - `start: u32`
  - `collections: Option<HashMap<String, Value>>`
- `IconifyCollectionsResponse`
  - `HashMap<String, IconifyCollectionMeta>` (top-level map)
- `IconifyCollectionResponseRaw`
  - `prefix: String`
  - `total: Option<u32>`
  - `title: Option<String>`
  - `uncategorized: Option<Vec<String>>`
  - `categories: Option<HashMap<String, Vec<String>>>`
  - `aliases: Option<HashMap<String, String>>`
  - `hidden: Option<Vec<String>>`
- `IconifyCollectionResponseNormalized`
  - `prefix: String`
  - `icons: Vec<String>` (deduped)
  - `total: Option<u32>`

## Output Contract (AI-Friendly)

### `search`

- Text: one `prefix:icon` per line.
- JSON: `{ icons, total, limit, start }`.
- Include `collections` only when `--include-collections` is set.

### `collections`

- Text: `prefix<TAB>name<TAB>total`.
- JSON: `[ { prefix, name, total } ]`.
- Optional `--prefix` is exact only.

### `collection`

- Text: one `prefix:icon` per line from normalized deduped icon list.
- JSON default: `{ prefix, icons, total }` (normalized).
- JSON with `--raw`: full API object for advanced agents.

### `get`

- Default: raw SVG to stdout.
- JSON mode: call `GET /{prefix}.json?icons={icon}` and return API payload.

## Integration with Existing Icon Fetch Path

- Keep `IconSourceType::IconifyName` using `IconifyClient::svg()`.
- Keep one shared Iconify transport path for CLI and TUI.

## Error Handling

- Keep structured API errors (network, decode, http status, invalid icon name).
- Include HTTP status and endpoint in errors.
- CLI maps API errors to concise `anyhow` messages and non-zero exit codes.

## Testing Strategy

- Unit tests in `src/iconify.rs` for:
  - deserializing real `/collections` top-level map payload
  - deserializing real `/collection` payload with categories and aliases
  - normalization and dedupe behavior for collection icon lists
  - optional field handling and 404/HTTP error mapping
- CLI tests for text/JSON output for all iconify subcommands.
- Integration test confirming `IconSourceType::IconifyName` uses shared client path.

## Docs

- Update `README.md` with:
  - command usage and examples
  - exact behavior of `collections --prefix`
  - normalized vs raw output for `collection`
  - raw SVG piping examples for `get`

## Milestones

1. Align `src/iconify.rs` models with actual endpoint shapes.
2. Add collection normalization and optional raw mode.
3. Wire CLI formatting and keep existing shared fetch integration.
4. Add tests and refresh docs.

## Acceptance Criteria

- All four iconify commands work for happy path and common errors.
- `collections` no longer returns empty due model mismatch.
- `collection` returns meaningful icon lists via normalization.
- `get` defaults to raw SVG and supports JSON mode.
- API layer is reusable and CLI-independent.
- Tests and docs cover real endpoint behavior.
