# `iconmate sync` — Plan

Status: **v1 shipped.** CLI (`iconmate sync [--apply] [--prune] [--rename old=new]`) and a read-only TUI dialog (`Shift+S`) now implement the reconciliation flow. Path-drift detection (§3.3) and inline TUI collision-rename (§4.3) are deferred to a follow-up.

## 1. Why it exists

Two recurring nuisances today, across every preset:

1. **Hand-dropped SVGs.** A developer (or designer, or coding agent) drops `foo.svg` into `assets/icons/` without running iconmate. Nothing references it from the barrel, so the icon is invisible to the rest of the app.
2. **Barrel drift.** Someone deletes an SVG manually, leaves the barrel entry pointing at a missing file. Or pulls a teammate's branch that only edited the filesystem.

Today the only fix is manual editing of `index.ts` / `icons.dart`. `sync` automates the reconciliation while staying out of the way.

## 2. Hard constraints (safety rails)

The user's primary fear: a command called `sync` that wakes up and rewrites their project. These rules exist to make that fear unfounded.

### 2.1 Never touches SVG assets

`sync` **only ever edits the barrel file** (`index.ts` / `icons.dart`). It never:

- Deletes SVG files
- Moves or renames SVG files
- Modifies SVG file contents

Worst case failure mode is a broken build (missing barrel entry), which `git checkout` of the barrel undoes instantly. The actual icon folder is untouched.

### 2.2 Dry-run by default

- `iconmate sync` — **read-only**. Prints the diff, exits. Writes nothing.
- `iconmate sync --apply` — actually writes.

This inverts the usual convention intentionally. `sync` should be annoying in the safe direction.

### 2.3 Double opt-in for destructive edits

Removing a barrel entry is the only operation that can break user code (call sites referencing the removed identifier stop compiling). So:

- `iconmate sync --apply` adds orphan files and fixes drifted paths, but **leaves orphan entries alone**.
- `iconmate sync --apply --prune` is required before sync will remove orphan entries.

Adding entries is always safe — worst case is an unused constant.

### 2.4 Atomic write + one-shot backup

- Write new barrel to a tempfile, `rename()` atomically. No half-written files on crash.
- Keep a `.iconmate.backup` copy of the previous barrel next to the real one for one session, so a bad sync is one `mv` away from undone.

### 2.5 No rename detection heuristics

If `heart.svg` was renamed to `red_heart.svg` on disk, `sync` sees one orphan file + one orphan entry. It will **not** try to match them by content hash, filename similarity, or any other heuristic. Clever-but-wrong-sometimes matching destroys trust. The user runs `iconmate rename` explicitly when they mean rename.

## 3. What it reconciles

Three operation categories, run against every preset (JS presets parse `index.ts`, Flutter parses `icons.dart`, otherwise identical logic):

### 3.1 Orphan files — SVG on disk, no barrel entry

- Infer an identifier from the filename via existing sanitization rules.
- Add an entry to the barrel.
- **Collision handling** (inferred identifier already exists in barrel):
  - **Non-interactive (`iconmate sync --apply`):** hard error, print the collision, exit non-zero. Offer a `--rename foo=fooAlt` flag to resolve, or suggest renaming the file on disk. No silent `foo2` auto-suffixing.
  - **TUI (`iconmate` interactive):** prompt inline with "would collide with existing `foo` — rename to: [___]", pre-filled with `fooAlt` as a suggestion. User accepts or edits.

### 3.2 Orphan entries — barrel entry, no SVG on disk

- `--apply` alone: listed but left alone (double opt-in protects call sites).
- `--apply --prune`: removed from the barrel.

### 3.3 Path drift — barrel entry points to wrong location

- File exists at a different relative path than the barrel claims (e.g. user moved `assets/icons/heart.svg` to `assets/icons/shapes/heart.svg`).
- `--apply`: fix the path in the barrel. Safe — identifier and call sites are unchanged.

## 4. UX

### 4.1 CLI, non-interactive

```
$ iconmate sync
Barrel: assets/icons/icons.dart

Would add (2):
  + heart        → assets/icons/heart.svg           (orphan file)
  + chevronRight → assets/icons/chevron_right.svg   (orphan file)

Would prune (1):
  - oldIcon      → assets/icons/old_icon.svg        (file missing)

Would fix (1):
  ~ settings     assets/icons/settings.svg → assets/icons/ui/settings.svg

Run with --apply to write additions and fixes.
Run with --apply --prune to also remove orphan entries.
```

### 4.2 Collision output

```
$ iconmate sync --apply
Error: orphan file assets/icons/foo.svg would collide with existing barrel entry `foo`.

Resolve by one of:
  • Rename the file:       mv assets/icons/foo.svg assets/icons/foo_alt.svg
  • Override identifier:   iconmate sync --apply --rename foo=fooAlt
```

Exit code non-zero. No partial writes when any collision is present — sync is all-or-nothing within a single run.

### 4.3 TUI

Surface sync as an action in the main TUI (alongside add/delete). Shows the same diff as the CLI, but each row has `a` (apply this item), `s` (skip), `r` (rename — for collisions). Pressing `A` applies all safe operations; prune requires a second confirmation.

## 5. Code touch points

Rough orientation, not a task list. Depends on the Flutter preset landing first (sync relies on preset-aware barrel parse/write).

- New: `src/sync.rs`
  - `SyncPlan { additions, removals, path_fixes, collisions }`
  - `compute_sync_plan(folder, preset, barrel_path) -> SyncPlan`
  - `apply_sync_plan(plan, options: { prune, renames }) -> Result<...>`
- `src/main.rs`
  - New subcommand `sync` with `--apply`, `--prune`, `--rename <id=newId>` flags.
- `src/views/`
  - New `sync_popup.rs` for the TUI flow.
- `src/utils.rs`
  - `get_existing_icons` stays preset-aware (shared with Flutter work).
  - Filesystem scan helper: list `*.svg` in folder, return relative paths.
- `README.md`
  - Document `iconmate sync` with a prominent note that it's dry-run by default and never touches SVG files.

## 6. Prerequisites

- Flutter preset shipped (parse-and-edit for `icons.dart` is reused here).
- Atomic write helper (tempfile + rename) extracted if not already present.
- Agreement on the `.iconmate.backup` convention — or decision to skip backups and rely on git. Open question for when we revisit.

## 7. Explicitly out of scope

- Rename detection.
- Bulk re-sanitization (changing identifier casing rules across a whole barrel).
- Multi-folder sync in one invocation.
- Automatic `pubspec.yaml` reconciliation for Flutter projects. The one-time pubspec hint from the Flutter preset covers this.
