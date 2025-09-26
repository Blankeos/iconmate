# iconmate 💚

Your new favorite way to manage icons for your Vite, NextJS projects without icon libraries!

Based on my blog post on [Why you might not need an icon library](https://carlotaleon.net/blog/why-you-dont-need-an-icon-library). Built with 🦀 Rust, ⚡️ designed for speed, 🦅 made for developers who hate icon library bloat.

Stop installing bloated icon libraries. All you need is [icones.js.org](https://icones.js.org) or your designer's Figma icon pack and paste them into your project with surgical precision.

**What Makes It Special ✨**

- **Zero Dependencies** 📦: Just a CLI, No icon libraries to bundle
- **Framework Native** 🧩: Works with React, Vue, Svelte, Solid - generates components automatically
- **Interactive Mode** 🎮: Just run `iconmate` and let it guide you
- **URL Support** 🌐: Fetch from any SVG URL, not just iconify
- **Raw SVG** 📋: Copy-paste SVG code directly
- **Empty SVG** 🏗️: Create placeholder icons for rapid prototyping

## Quick Start

```bash
# Install
npm install -g iconmate

# Run inside your project 🚀
iconmate

> 📁 Folder (src/assets/icons/) # Enter
> ✨ Preset # Choose react
> 🚀 Icon # heroicons:heart
> 💎 Name # Heart
```

✨ That's it. The interactive CLI guides you through adding icon to your project!

```tsx
// 👇 Then, you can just easily use any icon on your project like this!

import { IconHeart } from "@/assets/icons";

function MyApp() {
  return <IconHeart />;
}
```

You can also add sensible defaults by passing flags:

```bash
iconmate --folder src/components/Icons/ --folder src/components/icons
iconmate --folder src/components/Icons/ --icon heroicons:heart --name Heart
```

## Installation

### NPM 🦖

```bash
npm install -g iconmate
# or
pnpm add -g iconmate
# or
bun add -g iconmate
```

For one-off usage:

```bash
npx iconmate
# or
pnpm dlx iconmate
# or
bunx iconmate
```

> [!NOTE]
> **Note for Bun users:** Bun doesn't run `postinstall` scripts [by default](https://bun.com/guides/install/trusted) which is needed to install the iconmate binary. Add `"trustedDependencies": ["iconmate"]` to your `package.json` to do it! But you're only limited to running it with a package.json.
>
> Recommended: Just use pnpx for quick one-off usage. If in a project, either install globally or configure trustDependencies.

### Install from Cargo 🦀

```bash
cargo install iconmate
```

Or clone and install from source:

```bash
git clone https://github.com/blankeos/iconmate.git
cd iconmate
cargo install --path .
```

## Framework Presets

Determines the output filetype and the contents inside that file type.

| Preset     | File Type | Framework         |
| ---------- | --------- | ----------------- |
| `normal`   | `.svg`    | Vanilla HTML/CSS  |
| `react`    | `.tsx`    | React Components  |
| `svelte`   | `.svelte` | Svelte Components |
| `solid`    | `.tsx`    | Solid Components  |
| `vue`      | `.vue`    | Vue Components    |
| `emptysvg` | `.svg`    | Placeholder       |

> [!IMPORTANT]
> If you want to use `.svg` file types, make sure to setup [svgr](https://github.com/gregberge/svgr). I covered how to do this in:
>
> - [SolidJS (Vite)](https://carlotaleon.net/blog/why-you-dont-need-an-icon-library#use-svg-only-with-solidjs)
> - [React (Vite)](https://carlotaleon.net/blog/why-you-dont-need-an-icon-library#use-svg-only-with-react-vite)
> - [React (NextJS)](https://carlotaleon.net/blog/why-you-dont-need-an-icon-library#use-svg-only-with-react-nextjs)
> - Vue - contribution welcome!
> - Svelte - couldn't find an svgr integration :(

## Command Line

### Interactive Mode (Recommended)

```bash
iconmate

# Description of each prompt:
> 📁 Folder (src/assets/icons/) # where your icons will be saved.

> ✨ Preset # i.e. How will it be saved? An `.svg` or `.tsx` file in react, solid, etc.

> 🚀 Icon # Source of your icon. i.e. 'heroicons:heart' from https://icones.js.org, full URL, or any SVG. Can be empty except for 'emptysvg' preset.

> 🌄 Filename # The filename without the extension. i.e. heroicons:heart. Will only be prompted if you used an SVG, or an URL on icon.

> 💎 Name # The "Heart" in <IconHeart />
```

### Add Specific Icon

```bash
iconmate add --folder src/assets/icons --icon heroicons:heart --name Heart
```

### With URL

```bash
iconmate add --folder src/assets/icons --icon https://api.iconify.design/mdi:heart.svg --name Heart
```

### Raw SVG Content

```bash
iconmate add --folder src/assets/icons --icon '<svg>...</svg>' --name Heart
```

### Custom Export Template

```bash
iconmate add --folder src/assets/icons --icon heroicons:heart --name Heart --output-line-template "export { ReactComponent as Icon%name% } from './%icon%.svg?react';"
```

### Delete icons

```bash
iconmate delete --folder src/assets/icons
```

### Package.json Scripts

Best practice: Add sensible defaults to your script runner.

```jsonc
"scripts": {
  // Usage: npm run iconmate (usually this is the only command you need)!
  "iconmate": "iconmate --folder src/assets/icons/"

  // Usage: npm run iconmate-react
  "iconmate-react": "iconmate --folder ./src/assets/icons/ --preset react",

  // Usage: npm run iconmate-empty
  "iconmate-empty": "iconmate --folder ./src/assets/icons/ --preset emptysvg",
}
```

## Supported Platforms

- macOS (Intel & Apple Silicon) 🍎
- Linux (x64 & ARM64) 🐧
- Windows (x64) 🪟

## How It Works

1. **Find your icon**: Visit https://icones.js.org.
2. **Copy the name**: Like `heroicons:heart`.
3. **Run iconmate**: `iconmate`

![illustration](https://raw.githubusercontent.com/Blankeos/iconmate/refs/heads/main/_docs/icones-cli-illustration.png)

## Why this structure?

1. **Copy-paste workflow**: Find icon on icones.js.org → copy name → paste into iconmate
2. **Organized by default**: Everything goes into `index.ts` exports automatically and just typing `<Icon` will autosuggest from your current collection. Just regular TS behavior.
3. **TypeScript ready**: Generated code is fully typed. Pass custom width, height, fills, you name it.
4. **Customizable** 🎨: Want to add a default Tailwind class on every icon? custom props? Just add it to the file!
5. **Git-friendly**: Plain SVG files, no binary assets
6. **Lightning fast**: Native Rust binary, no Node.js startup time

## Contributing

Contributions are welcome—pull requests for bug fixes, new framework presets, or improvements are appreciated.

🐱 Repo: [github.com/Blankeos/iconmate](https://github.com/Blankeos/iconmate) - Star it if you love it ⭐

## What's Completed from the Roadmap

- ✅ Interactive prompt mode
- ✅ Framework presets (React, Vue, Svelte, Solid)
- ✅ URL and raw SVG support
- ✅ Custom export templates
- ✅ Zero-config installation

### Original Future Plans

- [x] An empty command. Creates an .svg, adds it to the index.ts with a name you can specify.
- [x] Paste an actual svg instead of an icon `name`.
- [x] Just a `--preset=svg,react,solid,svelte,vue` - which basically overrides templates. Default is `svg`.
- [x] Prompt Mode via `iconmate` - Interactive mode so you won't need to pass arguments.
- [x] Delete an icon using `iconmate delete`
- [ ] Other frameworks i.e. --preset=flutter or Go/Rust GUI apps? (Not sure how they work yet though).
- [ ] ~Zed or VSCode Extension~ (seems unnecessary now, it's just a CLI)

---

Made with Rust 🦀 | Based on [my blog post][my_blog]

[my_blog]: https://carlotaleon.net/blog/why-you-dont-need-an-icon-library
[my_blog#svgr]: https://carlotaleon.net/blog/why-you-dont-need-an-icon-library#bonus-just-save-them-as-svg-files
