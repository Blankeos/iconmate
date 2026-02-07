# Iconmate TODO Roadmap

## Done in this pass

- [x] Enter submits add popup (when focused on Name).
- [x] Paste shortcut support across add popup fields (Cmd/Ctrl+V behavior).
- [x] Default name + filename inference from Iconify names and known URLs.
- [x] Parse Iconify URL patterns and icones.js.org URL patterns.
- [x] Add popup visual cleanup (reduced borders/lines, removed debug block).
- [x] Delete popup visual cleanup (simpler `y` / `n` flow, less instruction noise).

## Existing completed

- [x] Help popup: status instead of defaults.
- [x] Homepage: Up/Down during search while keeping focus.
- [x] `iconmate.jsonc` generation.

## Next high-priority slice

- [x] Rename flow UX
  - [x] Add a lightweight rename action that only renames the file path export target.
  - [x] Keep icon component alias unchanged and show a warning for alias rename in-editor.
  - [x] Add explicit messaging to recommend IDE rename refactor for symbol-level rename.
  - [x] Acceptance: rename updates file + `index.ts` export path safely.

- [x] Global system prefs (`~/.config/iconmate.jsonc` or `~/iconmate.jsonc`)
  - [x] Confirm final search paths and precedence for macOS/Linux/Windows.
  - [x] Implement robust loader with unknown-key warnings (non-fatal).
  - [x] Add schema validation and friendly error output with file path context.
  - [x] Acceptance: startup logs/source reflect applied global values.

- [x] `o` key to open selected icon
  - [x] Add keybind in main list and shared opener support for Iconify search list integration.
  - [x] Resolve viewer command from local -> global -> OS default.
  - [x] Add `%filename%` token substitution and command escaping behavior.
  - [x] Acceptance: selected icon opens in configured app/editor.

- [x] `svg_viewer_cmd` defaults and fallback behavior
  - [x] macOS: Quick Look/open behavior.
  - [x] Linux/Windows: browser/system default fallback.
  - [x] Fallback to web preview when local command fails.
  - [x] Acceptance: command works cross-platform with graceful fallback.

## Add-flow product direction

- [x] ~During add: allow icon discovery using Iconify API inside TUI~
  - Prototype inline query input + paginated result list.
  - Validate whether this beats copy/paste from icones.js.org.
  - Decide final UX based on keystroke count and speed.

- [x] Dedicated icon search interface (TUI)
  - [x] Launch from Home (`i`) instead of starting inside Add popup.
  - [x] Build collection picker + icon view with `Tab` switching and Up/Down navigation.
  - [x] Use `/collections` for collections and local client-side collection filtering.
  - [x] Run debounced icon `/search` only while the Icons tab is active.
  - [x] `Enter` on collection clears search input and opens collection-scoped icon list.
  - [x] `Enter` on icon opens Add popup with icon/name/filename prefilled (no auto-submit).
  - [x] `Ctrl+o` in Icons downloads to temp and opens preview with loading/status feedback.

- [x] Search as standalone CLI for AI/tooling
  - Add machine-friendly command (JSON first).
  - Keep deterministic outputs and stable field names.
  - Acceptance: scriptable icon search with no interactive prompts.

## Notes

- Keep TUI minimal-first: less chrome, more keyboard speed.
- Prefer defaults inferred from icon source whenever possible.
