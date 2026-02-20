# LocalGPT Website (Docusaurus) - GEMINI.md

This directory contains the source code for the [LocalGPT](https://localgpt.app) documentation website and blog, built with **Docusaurus 3**.

## Project Overview

- **Core Technology:** Docusaurus 3 (React + TypeScript).
- **Purpose:** Central documentation hub for LocalGPT, a local-first, privacy-focused AI assistant built in Rust.
- **Key Features:**
  - Documentation for CLI, API, Memory System, and Sandbox.
  - Technical blog for project updates.
  - Dark-mode only interface with a focus on high-readability technical content.
  - Integrated search and sidebar navigation.

## Building and Running

The project uses `npm` for dependency management and task execution.

```bash
npm install            # Install dependencies
npm run start          # Start local development server (localhost:3000)
npm run build          # Generate production build (verifies links)
npm run serve          # Preview production build locally
npm run typecheck      # Run TypeScript type checking
```

**Note:** Always run `npm run build` before finalizing changes to ensure there are no broken links (`onBrokenLinks: 'throw'` is enabled in the config).

## Project Structure

- `docs/`: Markdown files for technical documentation. Structure is defined in `sidebars.ts`.
- `blog/`: Markdown files for blog posts. Historically significant records; do not modify past posts retroactively.
- `src/pages/`: Main landing page (`index.tsx`) and custom pages.
- `src/components/`: Custom React components (e.g., `HomepageFeatures`).
- `src/css/`: Global styles and Infima overrides (`custom.css`).
- `static/`: Static assets including logos, favicons, and update manifests.
- `docusaurus.config.ts`: Central configuration for the site, navbar, footer, and plugins.

## Development Conventions

### Styling & Theme
- **Primary Color:** `#25c2a0` (Teal Green).
- **Theme:** Dark mode only. The theme switcher is disabled.
- **Code Highlighting:** Prism is configured for `bash`, `toml`, `rust`, and `json`.

### Content Guidelines
- **LocalGPT Gen:** When describing the "Gen" feature, emphasize "immersive explorable worlds" (geometry, materials, lighting, camera). Avoid describing it as simple "3D scene generation."
- **Internal Linking:** Doc pages referencing LocalGPT CLI commands should link to the corresponding internal docs (e.g., `[`gen`](/docs/gen)`).
- **Technical Accuracy:** When updating documentation for CLI features, cross-reference the Rust implementation in the sibling directory `../localgpt/` to ensure the docs reflect the current state of the engine.

### Workflow
- **Commits:** Use Conventional Commits (`feat:`, `fix:`, `docs:`, `chore:`).
- **Sidebar:** If adding a new doc, ensure it is added to the appropriate section in `sidebars.ts`. Doc IDs must match their filenames (without `.md`).
- **Testing:** Perform manual verification of the site using `npm run start` and a final link-check via `npm run build`.
