# MVP implementation kickoff

**Status:** Chunk 1 complete. Chunk 4 MVP producer is implemented and loaded by the native app against the pilot.
**Updated:** 10 October 2026

## Objective

Deliver a buildable desktop MVP that opens a real local Hugo site and helps its author review content, inspect metadata and validation results, and preview pages. The first targets are macOS and Linux, with Omarchy as the first Linux environment.

## Working product contract

- **Primary workflow:** read-only content review. The user opens a Hugo repository, finds a page, reviews its content and metadata, and checks the rendered result.
- **Repository safety:** inspect and build locally. Do not change source content or Git state as a side effect.
- **Hugo ownership:** Hugo resolves content and generates site output. The MVP reads Hugo's resolved page index/config for compatibility and uses a versioned manifest as the extension point for richer native semantics.
- **Native scope:** render only declared content and supported blocks. Do not run arbitrary shortcodes, scripts, or theme HTML inside native mode.
- **Fallback:** pages or blocks that need theme behavior remain available through WebView output. Native mode must identify itself and make omitted theme chrome clear.
- **Implementation direction:** Rust core with Tauri 2 for the first desktop shell. The app window and platform menus/dialogs are native; the shared Peak SaaS inspired workspace is web-based and runs in the system WebView, so its controls are not OS-native widgets. macOS uses WKWebView and Linux uses WebKitGTK. Verify packaging, accessibility, keyboard/window conventions, and the WebKitGTK 4.1 dependency on Omarchy before calling the platform supported.

## Implementation progress

- Rust/Tauri desktop shell and the Peak inspired web workspace build. The macOS `.app` bundle was rebuilt successfully with the Manifest v1 producer and consumer (`npm run app:build`; 11.39 MiB).
- The native loader now calls Hugo's resolved `list all` and merged `config --format json`, reads Markdown only beneath the selected root, shows page metadata and menus, and starts a loopback Hugo preview into an app-owned temp destination.
- Basic headings, paragraphs, quotes, and simple list items render in the native pane. Shortcodes, raw HTML, code, tables, and media are currently flagged as unsupported; Website mode uses Hugo's rendered site inside an iframe. Planned shortcode handling: map recognized patterns to declarative native blocks, preserve readable children when possible, and show a positioned placeholder plus Website-mode action otherwise.
- Unit and pilot-site checks cover CSV parsing, basic Markdown, unsupported markup detection, Hugo page/menu loading, loopback preview startup, and omission of style/script payloads from native blocks. Latest run: 6 passed against the pilot; Hugo resolved 138 app-visible pages, 13 main-menu items, and 2 footer-menu items.
- Chunk 4 MVP: the app produces Manifest v1 JSON from Hugo's merged config, resolved page index, menus, front matter, and supported Markdown blocks. The front end now consumes the manifest for page navigation, native rendering, menus, metadata, and Website fallback. The schema records menu location, canonical/local page paths, content kind, and the reason native site chrome is omitted.
- Shortcode plan added to the PRD and implementation chunks: inventory pilot patterns first; define safe provenance/disposition fields; implement text fallback and explicit placeholders; then add only high-value declarative adapters. No template, arbitrary HTML, CSS, or script execution in native mode.
- Manifest-backed pilot verification passed in Rust and in the rebuilt macOS app: 138 pages, 13 main-menu items, 2 footer-menu items, with nested menu relationships and page metadata present. The app continued to display the real Hugo Website mode.
- End-to-end desktop review is now confirmed: the native app loaded the pilot's 138 pages; the Menus view showed 13 main-menu and 2 footer-menu entries with nested hierarchy; Website mode displayed Hugo's actual home page, header, content, and footer.
- macOS shell, folder picker, preview-mode switch, and the Peak-inspired three-pane workspace were visually reviewed. Omarchy/Linux packaging and runtime validation remain open under Chunk 2.
- Pilot review found CSS emitted by a shortcode appearing as text in Native mode. The parser now omits `<style>` and `<script>` payloads while preserving the unsupported-content warning and Website fallback; the app was rebuilt and the pilot was reloaded to verify the fix.
- The Peak inspired browser preview still opens with Northstar demo content until a repository is selected in the desktop app.
- Hugo preview output, cache, and processed resources are directed to temporary app paths. The pilot repo's tracked files remain unchanged.
- The generated desktop icon is stored under `src-tauri/icons/`.

## Workspace mapping

- The left rail contains app areas such as Home, Search, Content, and Settings.
- Hugo menu entries populate the content navigation sidebar, preserving labels, hierarchy, and order.
- Open pages and review tasks appear as tabs.
- Page title, summary, and supported blocks appear in the central pane.
- Metadata, validation, assets, and fallback details appear in the right context panel.
- The v1 schema allows a declared Hugo site header above page content and footer after the body, using only a title, logo, links, and text. This pilot does not provide declarative native chrome, so the manifest records why it is omitted; WebView preserves the full theme chrome. Arbitrary HTML and scripts stay out of native mode.
- The bottom status bar reports repository/build state and preview freshness. It is separate from the Hugo page footer.

## Pilot site

**Working pilot:** `/Users/johnjanuszczak/Projects/ceo-playbook` (read-only; switch if the user names another site).

- Hugo Extended 0.165.0; Blowfish theme 3.6.0.
- 198 Markdown files. Hugo's development build includes drafts and resolves 491 output pages; the production build resolves 468. The app adapter currently exposes 138 page/home/section rows from Hugo's resolved index.
- YAML front matter, bundle-style article/advisory content, summaries/descriptions, dates, drafts, tags/categories, images, tables, raw HTML, and many custom Hugo shortcodes.
- `config/_default/menus.en.toml` defines 13 main-menu items, including nested Services → Advisory/Speaking and Insights → Articles/Media/Signals/Engine Room, plus two footer-menu items (Research and Tags).
- `config/_default/params.toml` sets a floating header, copyright, and theme footer behavior. The Website preview preserves the actual Blowfish header, footer, shortcodes, and theme rendering.
- The repo's `.agents/AGENTS.md` says preserve content conventions and run Hugo after content changes. This MVP does not edit the site.

This is the best current pilot because it has real nested menus, a substantial content library, and enough shortcode/theme content to exercise both native rendering and WebView fallback. The README identifies the site as public; keep the app's behavior local and read-only.

**Assumed pain point to validate:** content metadata, menus, and full theme output currently require separate Hugo/site steps to review together. The baseline task above tests whether the app brings those checks into one workspace.

## Baseline review task

Open the pilot, review the home page and one article, inspect title/date/draft/tags/source path, switch to Menus to verify nesting and ordering, then compare Native and Website modes. The Website mode must preserve the full Hugo page, including theme chrome and shortcode output. Unsupported native content must be called out rather than silently presented as complete.

## Chunk 1 exit criteria

- A real Hugo repository is selected and can be read locally.
- The pilot's current content-review workflow and recurring pain point are documented.
- The pilot's config, menus, theme chrome, content shapes, and asset patterns are inventoried.
- MVP page/block coverage and the native/WebView boundary are confirmed against that site.
- A baseline task is defined so the MVP can be evaluated against the current workflow.

## Immediate dependency

The Tauri/Rust shell and generic architecture work can start from the working contract above. Pilot-specific manifest design and end-to-end implementation depend on the repository path and inventory.

**Chunk 1 is complete. Chunk 4's MVP schema and producer path are implemented and verified against the pilot.** Remaining work to close Chunk 4: validate produced manifests against the published JSON Schema and add a reusable site-side producer/fixture path. The next shortcode work item is SC-1: classify pilot shortcode instances as simple data, readable children, or theme-dependent/interactive before choosing adapter priorities. Other dependencies are Chunk 3's actionable Hugo build-error reporting and Chunk 2's Omarchy/Linux runtime validation; the compatibility reader in Chunk 6 is already implemented, and its manifest branch now consumes the v1 shape. The CSS/script leakage fix is included. The pilot path is available; the user can redirect to a different Hugo site at any point.
