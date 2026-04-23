export type IconmatePreset =
  | "normal"
  | "react"
  | "svelte"
  | "solid"
  | "vue"
  | "emptysvg"
  | "flutter";

/**
 * Project-level Iconmate config loaded from `iconmate.config.json`.
 */
export interface IconmateLocalConfig {
  /**
   * Folder where icons are written.
   * Default: `src/assets/icons` (or `assets/icons` when preset is `flutter`).
   */
  folder?: string;

  /**
   * Output preset. `normal` means plain `.svg` mode. `flutter` writes SVGs
   * and a Dart barrel file.
   * Default: `"normal"` (auto-switches to `"flutter"` when a Flutter project
   * is detected and no explicit preset is configured).
   */
  preset?: IconmatePreset;

  /**
   * Template used for each export line in `index.ts`.
   * Supported variables: `%name%`, `%icon%`, `%ext%`.
   * Ignored when preset is `"flutter"` (Dart barrel format is fixed).
   */
  output_line_template?: string;

  /**
   * Command used to open SVG files from the TUI.
   * Use `%filename%` as the SVG file path placeholder.
   */
  svg_view_cmd?: string;

  /**
   * Flutter preset only: project-root-relative path to the Dart barrel file.
   * Default: `"lib/icons.dart"`.
   */
  flutter_barrel_file?: string;

  /**
   * Flutter preset only: Dart class name emitted into the barrel.
   * Default: `"AppIcons"`.
   */
  flutter_barrel_class?: string;
}

/**
 * User-level Iconmate config loaded from the OS-specific config path.
 */
export interface IconmateGlobalConfig {
  /**
   * Command used to open SVG files from the TUI.
   * Use `%filename%` as the SVG file path placeholder.
   */
  svg_view_cmd?: string;
}
