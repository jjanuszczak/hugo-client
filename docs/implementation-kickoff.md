# MVP implementation kickoff

**Status:** Chunks 1, 3, and 4 implementation complete; Chunk 3 runtime failure-path checks remain open. Chunk 4's reusable Hugo producer, source enrichment, fixture, and schema validation are complete. Chunk 2 Omarchy/Linux validation remains independent and open and is the next implementation handoff.
**Updated:** 11 October 2026

## Objective

Deliver a buildable desktop MVP that opens a real local Hugo site and helps its author review content, inspect metadata and validation results, and preview pages. The intended product direction extends to local or cloned repositories, upstream sync, lossless content editing, and explicit GitHub PR submission. The first targets are macOS and Linux, with Omarchy as the first Linux environment.

## Working product contract

- **Primary workflow:** read-only content review. The user opens a Hugo repository, finds a page, reviews its content and metadata, and checks the rendered result.
- **Repository safety:** inspect and build locally. Do not change source content or Git state as a side effect.
- **Hugo ownership:** Hugo resolves content and generates site output. The MVP reads Hugo's resolved page index/config for compatibility and uses a versioned manifest as the extension point for richer native semantics.
- **Native scope:** render only declared content and supported blocks. Do not run arbitrary shortcodes, scripts, or theme HTML inside native mode.
- **Fallback:** pages or blocks that need theme behavior remain available through WebView output. Native mode must identify itself and make omitted theme chrome clear.
- **Implementation direction:** Rust core with Tauri 2 for the first desktop shell. The app window and platform menus/dialogs are native; the shared Peak SaaS inspired workspace is web-based and runs in the system WebView, so its controls are not OS-native widgets. macOS uses WKWebView and Linux uses WebKitGTK. Verify packaging, accessibility, keyboard/window conventions, and the WebKitGTK 4.1 dependency on Omarchy before calling the platform supported.

## Future product direction

The current implementation remains a read-only reviewer. The PRD now plans the following work after the MVP:

1. Open existing local checkouts and clone remote Git repositories into an app-managed workspace; remember and reopen repositories.
2. Show branch, upstream, dirty state, and ahead/behind status. Fetch and pull only through explicit actions, preserving dirty or diverged work and explaining conflicts before updates.
3. Add editing only for a defined subset that can be saved without rewriting unknown Hugo syntax. Keep `sourceDocument.rawSource` authoritative, detect external file changes, and present a source diff before writes.
4. Add branch, commit, push, and GitHub PR workflows with explicit review, secure credential handling, and recoverable error states.

These are planned product capabilities, not claims about the current build. Chunk 2 Omarchy/Linux validation remains the immediate implementation handoff; repository lifecycle, editing, and PR work follow the read-only MVP as Chunks 11–13 in the PRD.

## Implementation progress

- Rust/Tauri desktop shell and the Peak inspired web workspace build. The macOS `.app` bundle was rebuilt successfully with the Manifest v1 producer and consumer (`npm run app:build`; 11.39 MiB).
- The native loader now calls Hugo's resolved `list all` and merged `config --format json`, reads Markdown only beneath the selected root, shows page metadata and menus, and starts a loopback Hugo preview into an app-owned temp destination.
- Basic headings, paragraphs, quotes, and simple list items render in the native pane. Shortcodes, raw HTML, code, tables, and media are currently flagged as unsupported; Website mode uses Hugo's rendered site inside an iframe. Planned shortcode handling: map recognized patterns to declarative native blocks, preserve readable children when possible, and show a positioned placeholder plus Website-mode action otherwise.
- Unit and pilot-site checks cover CSV parsing, basic Markdown, unsupported markup detection, Hugo page/menu loading, loopback preview startup, source-preserving shortcode flow, and omission of style/script payloads from native blocks. Latest run: 12 unit tests pass; the pilot integration check loads 139 pages, 13 main-menu items, and 2 footer-menu items.
- Chunk 4 MVP: the app produces Manifest v1 JSON from Hugo's merged config, resolved page index, menus, front matter, and supported Markdown blocks. The front end now consumes the manifest for page navigation, native rendering, menus, metadata, and Website fallback. The schema records menu location, canonical/local page paths, content kind, and the reason native site chrome is omitted.
- Chunk 4 site-side producer: `producer/` includes a Hugo 0.146+ JSON output format and a companion Node source-enrichment script. Together they export resolved site language, recursive main/footer navigation, page identity/URLs/metadata/dates/tags/categories, author-declared safe blocks, data-only header/footer chrome, local image assets, exact source text, shortcode tokens/parameters/ranges, readable child text, and Website-required fallback flow. The producer marks declared content mixed by default; `hugoClient.complete: true` is an author assertion and cannot be independently verified. Fixture validation passed on Hugo 0.165.0 and the app's Rust Manifest v1 validator.
- Shortcode plan added to the PRD and implementation chunks: inventory pilot patterns first; define safe provenance/disposition fields; implement text fallback and explicit placeholders; then add only high-value declarative adapters. No template, arbitrary HTML, CSS, or script execution in native mode.
- SC-1 inventory complete: 832 shortcode opening invocations across 31 names in 133 of 198 Markdown files. Findings, risk classes, pilot call sites, and adapter recommendations are in `docs/shortcode-inventory.md`.
- SC-4 first adapter set verified: the pilot manifest schema-validates and includes native `ref`/`relref` links, `button` links, flattened `lead` content, Quick Answer callouts, and nested FAQ items. Adapted tokens retain source ranges and native disposition; malformed wrappers remain Website-required.
- Manifest-backed pilot verification passed in Rust and in the rebuilt macOS app: 139 pages, 13 main-menu items, 2 footer-menu items, with nested menu relationships and page metadata present. The app continued to display the real Hugo Website mode.
- End-to-end desktop review is now confirmed: the native app loaded the pilot's 138 pages; the Menus view showed 13 main-menu and 2 footer-menu entries with nested hierarchy; Website mode displayed Hugo's actual home page, header, content, and footer.
- macOS shell, folder picker, preview-mode switch, and the Peak-inspired three-pane workspace were visually reviewed. Omarchy/Linux packaging and runtime validation remain open under Chunk 2.
- Pilot review found CSS emitted by a shortcode appearing as text in Native mode. The parser now omits `<style>` and `<script>` payloads while preserving the unsupported-content warning and Website fallback; the app was rebuilt and the pilot was reloaded to verify the fix.
- The Peak inspired browser preview still opens with Northstar demo content until a repository is selected in the desktop app.
- Hugo preview output, cache, and processed resources are directed to temporary app paths. The pilot repo's tracked files remain unchanged.
- Chunk 3 on `codex/chunk-3-hugo-adapter`: repository inspection now runs before content loading in the desktop flow. Hugo command failures identify the command, repository, exit status, and captured stderr/stdout. Preview startup captures Hugo diagnostics; a replacement preview is adopted only after it responds, so a failed load preserves the last good preview. Temporary output is cleaned up on startup failure, empty Hugo content indexes fail clearly, and the UI reports a failed load without marking it ready. Full diagnostics stay visible in a scrollable toast for ten seconds. Runtime failure-path verification remains open.
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

**SC-1 through SC-4 are complete for the first adapter set. Chunks 3 and 4 implementation is complete on this branch; next recommended: Chunk 2 Omarchy/Linux runtime validation.** The inventory is [`shortcode-inventory.md`](shortcode-inventory.md). SC-2 preserves source for future edits and validates Manifest v1 against the checked-in schema. SC-3 adds ordered text/shortcode flow, readable child text, and a direct Website action. SC-4 maps `ref`/`relref`, `button`, `lead`, `quick-answer`, and `faq`/`faq-item` into safe native content. The figure adapter waits on Hugo-resolved asset metadata; interactive/media patterns remain Website-first.

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
- Chunk 4 is complete. The producer source, install/validation instructions, expected enriched manifest, and minimal Hugo fixture are in [`../producer/README.md`](../producer/README.md). Validation run for completion: five Node enrichment tests pass; Hugo 0.165.0 builds the fixture; the enrichment script processes five content pages; the Rust validator accepts the result; nested navigation and paired shortcode source ranges are present. Hugo 0.146 is the declared compatibility floor but was not separately run.
- Chunk 2 Omarchy/Linux packaging/runtime validation remains open and blocks Chunk 5. Chunk 3 runtime failure-path verification on representative Hugo failures also remains outstanding.

### Working tree and verification

- Workspace: `/Volumes/Dock NVMe SSD/Projects/hugo-client`.
- Chunk 3 branch check: `cargo fmt --manifest-path src-tauri/Cargo.toml` and `git diff --check` passed after the implementation changes. Rust tests, app build, and runtime failure-path checks have not been run in this turn.
- Product scope, full chunk/dependency plan, shortcode policy, and acceptance criteria: `docs/hugo-native-client-prd-spec.md`.
- Completed SC-1 evidence and candidate mappings: `docs/shortcode-inventory.md`.
- Current implementation and pilot status: this file. README build/run instructions: `README.md`.
- Pilot: `/Users/johnjanuszczak/Projects/ceo-playbook`; inspect it without modifying its source or Git state.
- macOS app build: `npm run app:build`. Pilot integration test: `HUGO_CLIENT_PILOT_PATH=/Users/johnjanuszczak/Projects/ceo-playbook cargo test --manifest-path src-tauri/Cargo.toml` (the test starts a loopback server and may need local networking permission).
- Latest verified baseline (11 October 2026): 12 Rust tests pass; `npm run app:build` creates a 15.05 MiB macOS app bundle; the pilot check loads 139 pages, 13 main-menu entries, and 2 footer-menu entries. Omarchy/Linux has not been verified.

**Completed:** Chunk 1; Chunk 4; SC-1 through SC-4 first priority set; MVP portions of Chunks 6, 7, and 8, including manifest consumption, runtime schema validation, Website fallback, shortcode placeholders, and safe native adapters. Chunk 2 Omarchy/Linux runtime validation and Chunk 3 failure-path checks remain open. The CSS/script leakage fix is included. The pilot path can be changed if the user names another site.
