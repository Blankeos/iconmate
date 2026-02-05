export type IconmatePreset = "" | "react" | "svelte" | "solid" | "vue" | "emptysvg";

/**
 * Project-level Iconmate config loaded from `iconmate.config.json`.
 */
export interface IconmateLocalConfig {
  /**
   * Folder where icons are written.
   * Default: `src/assets/icons`.
   */
  folder?: string;

  /**
   * Output preset. Empty string means plain `.svg` mode.
   * Default: `""`.
   */
  preset?: IconmatePreset;

  /**
   * Template used for each export line in `index.ts`.
   * Supported variables: `%name%`, `%icon%`, `%ext%`.
   */
  output_line_template?: string;

  /**
   * Command used to open SVG files from the TUI.
   * Use `%filename%` as the SVG file path placeholder.
   */
  svg_view_cmd?: string;
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
