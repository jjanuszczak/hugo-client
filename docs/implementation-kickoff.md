# MVP implementation kickoff

**Status:** SC-1 through SC-4 first adapter set complete. Omarchy/Linux build, WebView, native menu, and folder-picker shortcut checks pass; remaining cross-platform and responsive checks keep Chunk 2 open.
**Updated:** 11 October 2026

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
- Unit and pilot-site checks cover CSV parsing, basic Markdown, unsupported markup detection, Hugo page/menu loading, loopback preview startup, source-preserving shortcode flow, and omission of style/script payloads from native blocks. Latest run: 12 unit tests pass; the pilot integration check loads 139 pages, 13 main-menu items, and 2 footer-menu items.
- Chunk 4 MVP: the app produces Manifest v1 JSON from Hugo's merged config, resolved page index, menus, front matter, and supported Markdown blocks. The front end now consumes the manifest for page navigation, native rendering, menus, metadata, and Website fallback. The schema records menu location, canonical/local page paths, content kind, and the reason native site chrome is omitted.
- Shortcode plan added to the PRD and implementation chunks: inventory pilot patterns first; define safe provenance/disposition fields; implement text fallback and explicit placeholders; then add only high-value declarative adapters. No template, arbitrary HTML, CSS, or script execution in native mode.
- SC-1 inventory complete: 832 shortcode opening invocations across 31 names in 133 of 198 Markdown files. Findings, risk classes, pilot call sites, and adapter recommendations are in `docs/shortcode-inventory.md`.
- SC-4 first adapter set verified: the pilot manifest schema-validates and includes native `ref`/`relref` links, `button` links, flattened `lead` content, Quick Answer callouts, and nested FAQ items. Adapted tokens retain source ranges and native disposition; malformed wrappers remain Website-required.
- Manifest-backed pilot verification passed in Rust and in the rebuilt macOS app: 139 pages, 13 main-menu items, 2 footer-menu items, with nested menu relationships and page metadata present. The app continued to display the real Hugo Website mode.
- End-to-end desktop review is now confirmed: the native app loaded the pilot's 138 pages; the Menus view showed 13 main-menu and 2 footer-menu entries with nested hierarchy; Website mode displayed Hugo's actual home page, header, content, and footer.
- macOS shell, folder picker, preview-mode switch, and the Peak-inspired three-pane workspace were visually reviewed. Omarchy/Linux packaging and runtime validation remain open under Chunk 2.
- Omarchy validation (11 October 2026): confirmed Omarchy 4.0.4 / Arch on Hyprland Wayland (`x86_64`, kernel `7.2.5-3-omarchy`). Installed and confirmed Tauri's Arch prerequisites, including WebKitGTK 4.1 (2.52.6), GTK 3, and Rust/Cargo (1.99.0). `pkg-config` resolves WebKitGTK 4.1 and GTK 3. `npm run build` passes.
- Linux packaging: the existing 512×512 `icons/icon.png` is configured for bundles. The updated release executable and 4.48 MiB Debian bundle built successfully. The combined `npx tauri build --bundles deb,appimage` command failed at `linuxdeploy` while network access was restricted because `appimagetool` could not fetch its x86_64 runtime. With network access approved, standalone `npx tauri bundle --bundles appimage` produced the updated 99.67 MiB AppImage; `--appimage-version` confirms its runtime. Artifacts are under `src-tauri/target/release/bundle/` and are build outputs, not tracked files.
- Linux runtime: launched the updated release executable and rebuilt AppImage on Hyprland. The Hyprland client list reported `hugo-native-client` / `Hugo Native Client` at 621×750. The configured window size is 1480×960 with a 900×650 minimum. Fullscreen and floating layouts initially looked correct to the user; tiled layouts exposed vertical stretching side by side, then horizontal stretching when stacked. The first compact breakpoints did not activate because this WebView reports a scaled viewport. After calibrating them to 1520px width / 1300px height, a fresh screenshot of the 621×750 Hyprland tile showed the page using the available width with no blank center. A subsequent user report found the inspector could span the app window. Its compact presentation now uses a fixed-width, right-anchored overlay; the user confirmed this correction is better. Compact mode now provides pin controls that dock either sidebar into the grid and let the center pane resize; this new behavior awaits Omarchy visual confirmation. The compact mode removes the forced 900px page width, opens content navigation from the Content rail button, and hides the inspector until requested. The short-height variant collapses side panels and reduces vertical chrome. `npm run build` passes; user retesting of both tile orientations and pinned sidebars is still needed.
- Omarchy shell behavior: added native File and Edit menus. File → Open Hugo Repository… has the `Ctrl+O` accelerator and dispatches to the existing Tauri directory picker. The menu bar was visible in the Linux app. On 11 October, `Ctrl+O` opened a native directory chooser titled “Open Hugo repository” (Hyprland reported a second `hugo-native-client` dialog window). This verifies the shortcut-to-picker path on Omarchy; menu-item pointer activation and keyboard navigation inside the dialog were not separately exercised. macOS menu behavior has not been rechecked after adding these menus.
- Omarchy filesystem event capability: `inotifywait` observed a `CREATE` event for a file created inside a temporary directory. This verifies Linux host event delivery only. Per the updated Chunk 2 plan, full app-level watching, filtering, debounce, and rebuild coordination remain in Chunk 9. macOS host event delivery still needs a corresponding check.
- Rust verification: `cargo test --manifest-path src-tauri/Cargo.toml --offline` passed all 12 unit tests (0 failed). No pilot integration environment variable was set, so this does not verify loading the macOS-only documented pilot on Linux.
- Remaining Chunk 2 checks: verify the inspector stays a narrow right overlay in fullscreen and that pinned sidebars dock while the center pane resizes; retest side-by-side and stacked two-panel layouts on Omarchy; exercise menu-item activation and dialog keyboard navigation on Omarchy; recheck the new native menus/shortcut on macOS; and check filesystem event support on macOS. Hyprland tiles the app below its configured 900×650 minimum, so the compact presentation should be confirmed in both orientations. The combined Linux build command needs network access to download the AppImage runtime; separate Debian and AppImage bundle steps succeeded. `npm run app:build` remains macOS-specific because it requests the `app` bundle; use `npx tauri build --bundles deb,appimage` on Linux with network access available.
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

## Handoff for the next developer or agent

**SC-1 through SC-4 are complete for the first adapter set. Chunk 2 remains open:** Omarchy's native File/Edit menus and Ctrl+O folder-picker path are implemented and runtime-verified; responsive breakpoints are calibrated and one tiled screenshot looks correct, but the user should retest both tiling orientations. Also verify direct menu-item activation and dialog keyboard navigation, recheck the menu/shortcut on macOS, and verify macOS filesystem events. Linux host file-event delivery is confirmed with `inotifywait`; app-level watching remains in Chunk 9 as clarified in the PRD. Linux Debian and AppImage bundles now build in their separate steps; the combined build requires network access for the AppImage runtime download. Do not mark Chunk 2 complete until the remaining checks are recorded. The inventory is [`shortcode-inventory.md`](shortcode-inventory.md). SC-2 preserves source for future edits and validates Manifest v1 against the checked-in schema. SC-3 adds ordered text/shortcode flow, readable child text, and a direct Website action. SC-4 maps `ref`/`relref`, `button`, `lead`, `quick-answer`, and `faq`/`faq-item` into safe native content. The figure adapter waits on Hugo-resolved asset metadata; a reusable site-side manifest generator remains open under Chunk 4.

### SC-2 task

1. Read this kickoff, the PRD's “Shortcode rendering strategy” and development-chunk plan, `docs/shortcode-inventory.md`, and `schemas/hugo-client-manifest-v1.schema.json`.
2. Review `build_manifest_v1`, `manifest_blocks`, and `markdown_blocks` in `src-tauri/src/lib.rs` plus the manifest-to-view conversion in `frontend/app.js`.
3. Keep the exact source document as the round-trip authority for future editing. Retain source ranges, raw shortcode lexemes, ordered parameters, shortcode syntax and position. Treat decoded parameter values as hints; never discard unknown parameters or formatting during edits. Do not put rendered HTML or executable code in the manifest.
4. Extend the schema and add source-preservation fixtures/tests for readable-child, nested and interactive shortcode examples. Keep per-token disposition (`native`, `text-fallback`, or `website-required`); until adapters exist, mark all shortcode tokens Website-required. Preserve unknown shortcodes as Website-required placeholders.
5. Keep native adapter implementations and page UI polish out of SC-2; those belong to SC-3/SC-4 after the representation is reviewable.

### SC-2 exit criteria

- The schema describes exact source provenance/ranges, ordered parameter lexemes, and per-token disposition without treating source as executable/renderable HTML.
- Raw source survives serialization byte-for-byte; tokens identify ranges that future editors can replace while preserving untouched text and unknown attributes.
- Unknown/malformed tokens remain available in raw source and do not crash manifest loading.
- The internal producer validates manifests against the checked-in JSON Schema at load time; fixture tests include a rejected invalid disposition.
- `cargo test --manifest-path src-tauri/Cargo.toml` and `npm run app:build` pass; the pilot repository remains unchanged.

### Follow-on sequence and open dependencies

- **SC-3 (complete)** renders text fallback blocks and positioned placeholders with a direct Website-mode action. The app's ordered flow is built from source ranges and schema-validated; pilot loading confirms compatibility. This is a presentation fallback, not an editor. Current readable child rendering keeps text and link labels; exact Markdown/link targets remain in `sourceDocument.rawSource` for a later editor.
- **SC-4 (first priority set complete)** implements `ref`/`relref`, `button`, `lead`, `quick-answer`, and nested `faq`/`faq-item` using declarative data. Pilot loading and schema validation cover the resulting links/callout/FAQ flow. Remaining inventory adapters are deferred by risk and dependency: figure needs Hugo-resolved asset metadata; interactive/media patterns remain Website-first. SC-4 depends on SC-2, Chunk 7's renderer, and Chunk 8's Website path.
- The reusable site-side producer remains open under Chunk 4. Chunk 3 actionable Hugo build-error reporting and Chunk 2 Omarchy/Linux packaging/runtime validation are independent open work where dependencies permit.

### Working tree and verification

- Workspace: `/Volumes/Dock NVMe SSD/Projects/hugo-client`.
- Product scope, full chunk/dependency plan, shortcode policy, and acceptance criteria: `docs/hugo-native-client-prd-spec.md`.
- Completed SC-1 evidence and candidate mappings: `docs/shortcode-inventory.md`.
- Current implementation and pilot status: this file. README build/run instructions: `README.md`.
- Pilot: `/Users/johnjanuszczak/Projects/ceo-playbook`; inspect it without modifying its source or Git state.
- macOS app build: `npm run app:build`. Pilot integration test: `HUGO_CLIENT_PILOT_PATH=/Users/johnjanuszczak/Projects/ceo-playbook cargo test --manifest-path src-tauri/Cargo.toml` (the test starts a loopback server and may need local networking permission).
- Latest cross-platform checks (11 October 2026): macOS baseline remains 12 passing Rust tests, a 15.05 MiB `.app` build, and pilot load of 139 pages / 13 main-menu entries / 2 footer-menu entries. Omarchy package prerequisites are installed; `npm run build` and `cargo fmt --check` pass; all 12 Rust unit tests pass offline; the updated Linux release executable, 4.48 MiB Debian bundle, and 99.67 MiB AppImage build; `--appimage-version` passes; the rebuilt AppImage opens a Hyprland window; native File/Edit menus render; and Ctrl+O opens the directory picker. Remaining checks are listed above.

**Completed:** Chunk 1; SC-1 through SC-4 first priority set; MVP portions of Chunks 4, 6, 7, and 8, including manifest production/consumption, runtime schema validation, Website fallback, shortcode placeholders, and safe native adapters. **Do not treat Chunk 4 as fully closed:** a reusable site-side producer and producer fixtures remain open. Chunk 2 Omarchy/Linux platform validation is partially complete: Linux build, WebKitGTK runtime, native menus, Ctrl+O, directory picker, and Linux host file-event evidence are recorded above. The remaining cross-platform and responsive checks stay open. The CSS/script leakage fix is included. The pilot path can be changed if the user names another site.
