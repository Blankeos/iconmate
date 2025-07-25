## Icones CLI

> Pre-release. Might change the name later because I might not only just want to use icones.js.og.

Based on my [blog post](https://carlotaleon.net/blog/why-you-dont-need-an-icon-library) here.

Built mainly for js/ts framework (Solid, React, Vue, Svelte, etc.) devs to have a quick, organized, flexible, and "dependency-less" way to import icons for their projects. Made with Rust 🦀.

## How it works

- The idea is, you only visit https://icones.js.org.
- Copy the icon name i.e. `heroicons:heart`
- Run the command: `icones-cli --folder ./src/assets/icons --icon heroicons:heart --name Heart`.

![illustration](_docs/icones-cli-illustration.png)

## Getting Started

This CLI tool helps you fetch icons and integrate them into your project.

### Installation

Currently only from Cargo, will add more when I figure it out

1.  Clone the repository:
    ```bash
    git clone https://github.com/carlotaleon/icones-cli.git
    cd icones-cli
    ```
2.  Install globally with `cargo`:
    ```bash
    cargo install --path .
    ```

### Usage

```bash
icones-cli --folder ./src/components/Icons --icon heroicons:heart --name Heart
```

Recommended if you have package.json.

```jsonc
"scripts": {
  // Usage will be `npm run addicon --icon heroicons:heart --name Heart
  "addicon": "icons-cli --folder ./src/assets/icons/"
}
```

This command will:

1.  Create the `./src/components/Icons` folder if it doesn't exist.
2.  Fetch the `heroicons:heart` SVG from Iconify API and save it as `./src/components/Icons/heroicons:heart.svg`.
3.  Update or create `./src/components/Icons/index.ts` with an export line.

### Parameters

- `--folder <PATH>`: (Required) Path to the directory where the icon SVG will be saved and `index.ts` updated.
- `--icon <NAME_OR_URL>`: (Required) The name of the icon (e.g., `"heroicons:heart"`) or a full URL to the icon SVG (e.g., `"https://api.iconify.design/heroicons:heart.svg"`).
- `--name <ALIAS>`: (Required) The alias for the SVG, used in the `index.ts` export (e.g., `"HeartIcon"`).
- `--output-line-template <TEMPLATE>`: (Optional) Custom template for the export line. Use `%name%` for the icon alias and `%icon%` for the raw icon source.
  - **Default**: `export { default as Icon%name% } from './%icon%.svg';`
    - Example with default: `export { default as IconHeartIcon } from './heroicons:heart.svg';`

## Future Plans

- [ ] Paste an actual svg instead of an icon `name` (i.e. `heroicons:heart`). So it integrates better w/ not just icones, but also icon jar where you can copy icons from non-open-source icon libraries (i.e. Untitled UI, Anron Pro).
- [ ] An `--svg-output-template <TEMPLATE>` argument with %svg% as the variable. So that you can maybe customize the output before it's exported.
    - [ ] + a `--svg-output-ext-template <TEMPLATE>` i.e. `%iconname%.svg` or `%iconname%.tsx`
- [ ] Just a `--preset=svg,reactcomp,solidcomp,sveltecomp,vuecomp` - which basically overrides `--svg-output-file-template` and `--svg-output-ext-temlpate` and `--output-line-template`. So you won't need to specify it. Much much easier devx I think? + the default will just be `svg`.
- [ ] Prompt Mode maybe via `icones-cli prompt` - Basically runs the CLI in interactive mode so you won't need to pass arguments like a menace. Also assumes that you can use the same flags for default values.
- [ ] A Zed extension (I use Zed, idk if it can create an extra command in Zed?).
- [ ] A VSCode extension (Seems super doable).
