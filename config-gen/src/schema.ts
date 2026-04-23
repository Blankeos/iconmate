import * as z from "zod";

export const DEFAULT_FOLDER = "src/assets/icons";
export const DEFAULT_PRESET = "normal";
export const DEFAULT_OUTPUT_LINE_TEMPLATE =
  "export { default as Icon%name% } from './%icon%%ext%';";

export const PRESET_VALUES = [
  "normal",
  "react",
  "svelte",
  "solid",
  "vue",
  "emptysvg",
  "flutter"
] as const;

export const PresetSchema = z.enum(PRESET_VALUES).meta({
  title: "Preset",
  description:
    "Icon output preset. 'normal' means plain SVG mode. 'flutter' writes SVGs + a Dart barrel (lib/icons.dart by default). Others are framework presets or an emptysvg placeholder.",
  default: DEFAULT_PRESET,
  examples: ["normal", "react", "solid", "emptysvg", "flutter"]
});

export const FlutterBarrelFileSchema = z.string().min(1).meta({
  title: "Flutter Barrel File",
  description:
    "Project-root-relative path to the generated Dart barrel when preset='flutter'. Default: `lib/icons.dart`.",
  examples: ["lib/icons.dart", "lib/gen/icons.dart"]
});

export const FlutterBarrelClassSchema = z.string().min(1).meta({
  title: "Flutter Barrel Class",
  description:
    "Dart class name emitted into the barrel when preset='flutter'. Default: `AppIcons`.",
  examples: ["AppIcons", "Assets"]
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
    svg_view_cmd: SvgViewCommandSchema.optional(),
    flutter_barrel_file: FlutterBarrelFileSchema.optional(),
    flutter_barrel_class: FlutterBarrelClassSchema.optional()
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
