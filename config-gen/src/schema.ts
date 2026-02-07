import * as z from "zod";

export const DEFAULT_FOLDER = "src/assets/icons";
export const DEFAULT_PRESET = "normal";
export const DEFAULT_OUTPUT_LINE_TEMPLATE =
  "export { default as Icon%name% } from './%icon%%ext%';";

export const PRESET_VALUES = ["normal", "react", "svelte", "solid", "vue", "emptysvg"] as const;

export const PresetSchema = z.enum(PRESET_VALUES).meta({
  title: "Preset",
  description:
    "Icon output preset. 'normal' means plain SVG mode. Other values are framework or emptysvg presets.",
  default: DEFAULT_PRESET,
  examples: ["normal", "react", "solid", "emptysvg"]
});

export const SvgViewCommandSchema = z.string().min(1).meta({
  title: "SVG View Command",
  description:
    "Command used to open an SVG. Supports `%filename%` placeholder, for example: `zed %filename%`.",
  examples: ["zed %filename%", "code %filename%", "open %filename%"]
});

export const OutputLineTemplateSchema = z.string().min(1).meta({
  title: "Output Line Template",
  description:
    "Template for each export line. Supported placeholders: `%name%`, `%icon%`, `%ext%`.",
  default: DEFAULT_OUTPUT_LINE_TEMPLATE,
  examples: [DEFAULT_OUTPUT_LINE_TEMPLATE]
});

export const LocalConfigSchema = z
  .object({
    folder: z.string().min(1).optional().meta({
      description: "Folder where icons are written.",
      default: DEFAULT_FOLDER,
      examples: [DEFAULT_FOLDER, "src/components/icons"]
    }),
    preset: PresetSchema.optional(),
    output_line_template: OutputLineTemplateSchema.optional(),
    svg_view_cmd: SvgViewCommandSchema.optional()
  })
  .meta({
    id: "IconmateLocalConfig",
    title: "Iconmate Local Config",
    description: "Schema for project-level iconmate config (`iconmate.config.json`)."
  });

export const GlobalConfigSchema = z
  .object({
    svg_view_cmd: SvgViewCommandSchema.optional()
  })
  .meta({
    id: "IconmateGlobalConfig",
    title: "Iconmate Global Config",
    description: "Schema for user-level iconmate config at the OS-specific config directory."
  });

export type LocalConfig = z.input<typeof LocalConfigSchema>;
export type GlobalConfig = z.input<typeof GlobalConfigSchema>;
