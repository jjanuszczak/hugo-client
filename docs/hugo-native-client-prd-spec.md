# Hugo Native Content Client

**Product requirements and technical specification**  
**Status:** MVP implementation in progress; Manifest v1 pilot path implemented  
**Date:** 10 October 2026

## 1. Product summary

Hugo Native Content Client is a local desktop application for working with content stored in a Hugo website repository. It watches the repository, asks Hugo to resolve and render site data, and presents supported content in native application components. It combines a fast local content workspace with a faithful browser-based fallback for pages and components that require normal web rendering.

The app is not intended to reproduce arbitrary Hugo themes by reverse-engineering their HTML. Hugo remains the authority for content resolution and site generation. A small, explicit content interface lets a theme or site declare which pages and blocks can be represented natively.

## 2. Problem and opportunity

Hugo provides fast static-site generation and a flexible content model, but many editing and review activities still happen through a text editor, terminal, and browser. A native client could make a Hugo repository feel like a structured content workspace: browse pages, review metadata, validate content, inspect assets, and preview supported content without repeatedly navigating raw files.

A native renderer alone has weak differentiation from `hugo server` plus a browser. The product earns its value through useful content workflows and a stable interface to Hugo-managed content. Native rendering is a presentation option within that client, not the product's only purpose.

## 3. Product principles

1. **Hugo owns site semantics.** Use Hugo to resolve front matter, content bundles, menus, taxonomies, languages, shortcodes, and configuration.
2. **Native rendering is explicit.** A page or block is native only when represented through a declared, versioned content contract.
3. **Graceful fallback preserves fidelity.** Unsupported or web-dependent output opens in an embedded system WebView or the user's browser.
4. **Local-first and repository-safe.** The app reads and runs the user's local Hugo project; it does not silently modify content or Git state.
5. **Fast through event-driven work.** Coalesce filesystem events, avoid unnecessary Hugo invocations, and update only affected views when possible.
6. **Separate content from UI.** The manifest and content model must not depend on a particular desktop UI toolkit.
7. **Progressive adoption.** Existing Hugo sites remain usable without restructuring; native support can be added incrementally.

## 4. Goals and non-goals

### Goals

- Open a local Hugo repository and detect its Hugo configuration and project structure.
- Produce a useful native content browser and preview for sites that opt into the native content contract.
- Support common editorial tasks around content discovery, review, metadata validation, and asset inspection.
- Watch local changes and refresh previews quickly while keeping the app responsive.
- Preserve the regular Hugo output as the fidelity reference and provide a WebView fallback.
- Keep the core content model reusable by future desktop, mobile, or command-line clients.

### Non-goals for the initial release

- Reimplement Hugo's Go template engine or compete with Hugo's site generator.
- Convert arbitrary theme HTML/CSS/JavaScript into a native widget tree.
- Guarantee pixel-identical native rendering of every theme, shortcode, or script.
- Replace a user's editor, Git client, deployment pipeline, or Hugo command line.
- Provide a collaborative hosted CMS, remote publishing, or Git write operations.
- Require all pages to adopt native blocks before the app can be useful.

## 5. Users and core jobs

### Primary user: Hugo site author/editor

- Find a page or post without remembering its path.
- Review the content, metadata, taxonomy, language, and publication state together.
- See how a draft or change affects the rendered page.
- Identify missing required fields, broken local asset references, or unsupported blocks.
- Browse content and assets while working offline.

### Secondary user: theme/site maintainer

- Add native-compatible page and block declarations to a site or theme.
- Confirm which parts render natively and which fall back to web output.
- Validate the generated content manifest against a published schema.

## 6. Product experience

### Main workspace

- **Repository chooser:** choose an existing local repository; show detected Hugo project root and configuration.
- **Content navigation:** browse sections, pages, drafts, taxonomies, languages, and menus as available in the manifest.
- **Content pane:** native page title, summary, metadata, and supported content blocks.
- **Preview mode:** switch between native view and faithful Hugo/WebView output for a page.
- **Validation panel:** show actionable warnings and errors, with a path and field/block reference.
- **Build status:** show watching/building/ready/error state and the last successful update.

### Native versus web display

The desktop window, OS menus, and folder dialogs use platform components. The shared workspace and native content blocks use web UI inside the system WebView, so those controls are not OS-native widgets. Here, “native preview” means manifest-backed rendering without rendering the Hugo website. The full website remains web content. Unsupported pages and blocks must be viewable through a system WebView or external browser, and the UI must label the active rendering mode.

The Peak SaaS reference maps Hugo data into the workspace as follows: Hugo menu entries and hierarchy populate the content navigation sidebar; page title, summary, and supported blocks populate the central page view; page metadata, validation results, assets, and fallback details populate the right context panel. App-level destinations remain in the left rail, and open pages/tasks appear as tabs. The v1 schema draft defines optional declarative `site.chrome.header` and `site.chrome.footer` fields for a title, logo, links, and text. Native mode displays site chrome only when those fields are present; arbitrary HTML and scripts are not allowed. WebView remains the fidelity reference and preserves the theme's full chrome. Never treat the Hugo footer as the app's bottom status bar, which reports repository and build state.

### Visual layout direction

Use `docs/peak-saas-layout.jpeg` as the visual reference for a desktop workspace organized into five persistent regions:

- **Left rail:** compact, icon-led access to top-level areas such as Home, Search, Activity/Builds, Content, and Settings. Keep the selected area visually clear and provide accessible names for every icon control.
- **Navigation sidebar:** context-sensitive navigation for the selected area. In Content, this is where repository/section filters and the page list live; it should support narrowing the list by section, draft state, language, and taxonomy without replacing the main workspace.
- **Tabbed main workspace:** the selected page or task opens here. Tabs support keeping several pages available during review. The main area hosts the native page view or WebView, with an explicit rendering-mode control and a clear page identity.
- **Context panel:** a right-side inspector for information and actions relevant to the selected page, such as metadata, validation findings, asset references, and build/fallback details. It must not become a second source of content truth; values come from the manifest and Hugo diagnostics.
- **Bottom status bar:** persistent repository and build state, including watching/building/ready/error and whether the displayed output is current or stale. Do not rely on color alone for state.

Treat this as a layout direction, not a requirement to copy the reference's SaaS styling or force every panel open at every window size. Use native platform controls and conventions for the chosen OS. The context panel may collapse into a drawer or inspector toggle at narrower widths; the left rail may compact further, while the page list and main content retain priority. Preserve the current page and selection when panels open or close. Define minimum supported window size and exact breakpoints during the UI implementation spike.

This layout gives the MVP a stable shell for the existing content browser, page detail, validation, and build-state requirements. It does not change the read-only MVP scope, Hugo manifest contract, or native-versus-web rendering boundary. The shell will use Tauri 2, subject to the platform spike and pilot validation.

### Initial platform targets

The first release targets **macOS and Linux, with Omarchy as the initial Linux environment**. Treat Omarchy as the first Linux distribution and desktop configuration to validate, while keeping the app installable on other Linux systems where dependencies permit. Use each operating system's expected window behavior, menus, dialogs, keyboard shortcuts, and WebView integration. The shared workspace regions and content behavior should remain consistent across platforms, with platform-appropriate control styling and window conventions.

For these targets, the app uses platform-appropriate windows, menus, folder dialogs, keyboard conventions, and system WebViews; the shared workspace controls are web-based. The technical spike should prove packaging, file watching, WebView availability, and core navigation on macOS and Omarchy Linux, and record platform gaps. Windows is outside the first-release target.

## 7. MVP scope

MVP should prove that the app helps with a real Hugo repository beyond simply opening a browser preview.

### Included

- Select and remember one or more local Hugo repositories.
- Detect Hugo executable/version, config, content root, and output location; explain missing prerequisites.
- Use Hugo's resolved page index and merged config to load existing sites without requiring source/theme edits. Read content under the selected repository root only.
- Start the pilot's Hugo Website preview on loopback, with generated output, cache, and processed resources directed to an app-owned temporary directory.
- Normalize the page list and menu/config data into the app content model. A versioned JSON manifest remains the extension point for richer declared blocks and native site chrome.
- Native page list and page detail for: title, description/summary, date, draft status, section, tags/categories, language, and canonical/local path when available.
- Native first-pass blocks: heading, paragraph, quote, and simple list item. Flag unsupported shortcodes, raw HTML, code, tables, and media; use the Website WebView for full Hugo rendering.
- Shortcode content follows the progressive native strategy in §10: recognize and map selected shortcodes to declarative blocks, preserve readable children when only the shortcode presentation is unsupported, and show an explicit Website fallback for unsupported output.
- Display Hugo main/footer menu labels, ordering, and parent relationships.
- Watch relevant files, debounce changes, rebuild, refresh the current page, and preserve selection/scroll position where feasible.
- Read-only content validation for required metadata, missing local assets, invalid manifest data, and unknown block types.

### Excluded from MVP

- Editing and writing content in the app.
- Git commit, branch, merge, or push workflows.
- Arbitrary shortcode execution in native views.
- Full WYSIWYG or full Markdown compatibility in the native renderer.
- Arbitrary shortcode/template execution or a promise that every theme shortcode will have a native equivalent.
- Cross-repository content aggregation, cloud sync, publishing, and plugins.

## 8. Functional requirements

### Repository and Hugo lifecycle

- **FR-1:** The user can choose a repository root and the app detects whether it is a Hugo project.
- **FR-2:** The app locates Hugo using an explicit configured path or the user's executable search path; it reports the version and build errors.
- **FR-3:** The app invokes Hugo with a controlled working directory and arguments, honoring project configuration and environment as configured by the user.
- **FR-4:** The app does not write to source content or invoke Git mutations without an explicit future user action.
- **FR-5:** Build output is isolated from the source tree where practical, with project-configured output behavior respected.

### Content model and native rendering

- **FR-6:** The app consumes a versioned manifest describing site metadata, pages, navigation, assets, supported blocks, and optional declarative site chrome.
- **FR-7:** The app renders known block types with native components and reports unknown types without crashing.
- **FR-8:** A page with no native representation remains discoverable and can be opened in WebView/browser output.
- **FR-9:** Links and assets resolve safely to local generated output or declared repository assets; external links are identified as external.
- **FR-10:** The user can switch between native and web rendering for a page.
- **FR-10a:** The native renderer can display recognized shortcodes through safe declarative adapters that map shortcode parameters and supported children to known native block types. The renderer never executes Hugo templates, arbitrary HTML, CSS, or JavaScript.
- **FR-10b:** When a shortcode cannot be mapped, preserve readable child content where possible and show an explicit placeholder with the shortcode name/reason and a direct Website-mode action. Do not silently omit the shortcode or imply that the native page is complete.
- **FR-10c:** Expose per-page or per-block coverage as native, text fallback, or Website required. A page containing a mixture reports mixed coverage.

### Watching and refresh

- **FR-11:** The app watches content, layouts, configuration, data, assets, and relevant theme files.
- **FR-12:** It ignores generated directories, `.git` internals, editor swap files, and irrelevant events by default, with configurable exclusions.
- **FR-13:** Bursts of file events are debounced and coalesced into a single build request.
- **FR-14:** If changes arrive during a build, the app schedules the latest state and avoids presenting stale output as current.
- **FR-15:** Successful updates preserve the selected page and, when possible, its scroll position.

### Validation and diagnostics

- **FR-16:** Errors identify the build stage, source path where known, and actionable next step.
- **FR-17:** Validation distinguishes errors that block a native page from warnings that only trigger fallback.
- **FR-18:** The app records no content remotely; diagnostic collection is opt-in if introduced later.

## 9. Non-functional requirements and success measures

- **Responsiveness:** app shell remains interactive while Hugo builds.
- **Refresh target:** on a representative small/medium site, file-change-to-preview refresh should have a p95 target of under 1.5 seconds when Hugo completes within that budget. Measure separately from Hugo build time; do not claim the app can make Hugo itself faster.
- **Resource use:** idle watcher should use negligible CPU; no busy polling as the primary mechanism.
- **Correctness:** manifest schema validation prevents malformed data from crashing the UI.
- **Fidelity:** web fallback displays Hugo-generated output, not a reconstruction.
- **Portability:** platform-specific file watching and WebView integration are behind interfaces.
- **Security:** execute only the configured Hugo binary; do not evaluate arbitrary manifest scripts; constrain local path resolution and external navigation.

Success should be judged by a pilot with a real site: time to locate/review content, rate of useful native coverage, preview refresh latency, build failure clarity, and whether the user returns to the client for repeat work. A pure preview-speed benchmark is insufficient evidence of product value.

## 10. Technical specification

### High-level components

1. **Desktop shell:** native window, menus, repository selection, navigation, status, and platform WebView host.
2. **App coordinator:** repository lifecycle, build queue, current selection, app settings, and UI events.
3. **Hugo adapter:** executable discovery, version detection, command invocation, environment, output capture, and cancellation/supersession.
4. **Change detector:** native filesystem watcher with filtering, debounce, and optional low-frequency reconciliation scan for missed events.
5. **Hugo compatibility reader:** uses Hugo's resolved CLI index/config and safely reads supported Markdown from the selected site without writing to it.
6. **Manifest producer/loader:** optional Hugo templates/output format (or site-provided generator) for richer v1 block/site-chrome declarations; validates schema version and paths before conversion to the same UI-independent site model.
7. **Content model:** typed site, page, asset, navigation, and block structures.
8. **Native renderer:** maps supported blocks to platform controls and styles.
9. **Web renderer:** system WebView or external browser pointed to locally generated Hugo output.

### Recommended implementation shape

Use Rust for the core and Tauri 2 for the MVP shell. Tauri provides a native desktop window and platform WebView, with one shared workspace implementation. The Peak SaaS inspired workspace controls are web-based, not OS-native widgets. Use native OS menus and folder dialogs, follow platform keyboard/window conventions, and verify accessibility. macOS uses WKWebView; Linux uses WebKitGTK. The Omarchy build and runtime still need validation.

Validate the Tauri shell and its web-based workspace tradeoff on macOS and Omarchy Linux. Keep the content model and Hugo adapter independent of the UI toolkit.

### Hugo integration contract

The richer native interface is opt-in and versioned. A Hugo site/theme can emit a JSON manifest alongside its normal output, using Hugo templates or a supported site-side generator. The manifest is declarative data only; no executable JavaScript or arbitrary component code is permitted. The MVP also produces this shape internally from Hugo's merged config and resolved page index, so existing sites can exercise the same renderer before they add a site-side producer.

The MVP also includes a read-only compatibility path for an existing Hugo site that has no manifest. Hugo resolves the page index and merged menu/config data; the client reads source Markdown only for its declared basic native subset and flags unsupported structures. Website mode uses Hugo's own local output. This lets the user evaluate a site before installing a module or changing its theme. The manifest remains the path for richer native blocks, assets, and explicitly declared site chrome.

Example (illustrative):

```json
{
  "schemaVersion": 1,
  "site": {
    "title": "Example Site",
    "baseURL": "https://example.test/",
    "defaultLanguage": "en"
  },
  "pages": [
    {
      "id": "content/posts/example.md",
      "path": "/posts/example/",
      "title": "Example",
      "summary": "A short introduction.",
      "date": "2026-10-10T00:00:00Z",
      "draft": false,
      "section": "posts",
      "taxonomies": {"tags": ["hugo"]},
      "rendering": {"mode": "native", "fallbackURL": "/posts/example/"},
      "blocks": [
        {"type": "heading", "level": 2, "text": "First section"},
        {"type": "paragraph", "inlines": [{"type": "text", "text": "Hello "}, {"type": "link", "text": "world", "href": "https://example.test/"}]},
        {"type": "image", "src": "/images/example.jpg", "alt": "Example"},
        {"type": "callout", "tone": "note", "children": [{"type": "paragraph", "inlines": [{"type": "text", "text": "A note."}]}]}
      ]
    }
  ],
  "navigation": [],
  "assets": []
}
```

The published v1 schema is [`../schemas/hugo-client-manifest-v1.schema.json`](../schemas/hugo-client-manifest-v1.schema.json). In v1, `id` is the normalized repository-relative source path; `sourcePath` carries the same source identity; `path` is the site-local URL path when it belongs to `baseURL` and otherwise the resolved URL; `canonicalURL` retains Hugo's resolved permalink. `date` is an RFC 3339 date-time when Hugo provides one. `navigation[].menu` distinguishes `main` and `footer`; `parent` preserves menu hierarchy. Native blocks currently include headings, paragraphs, quotes, and simple lists. Unsupported source structures mark rendering `mixed` and point to a Website fallback. `site.chrome` is declarative and optional; the internal pilot manifest currently reports why native chrome is omitted. Schema changes require a version increment or a backward-compatible optional field. Unknown fields should be ignored; unknown block types should trigger a page-level or block-level fallback according to declared policy. Runtime validation of produced manifests against the schema remains an implementation task.

### Proposed block types for v1

| Type | Data | Native behavior |
|---|---|---|
| `heading` | level, text, optional anchor | Native heading text and anchor navigation |
| `paragraph` | inline nodes | Native rich text; limited marks and links |
| `list` | ordered flag, nested items | Native list |
| `quote` | inline/block children, optional attribution | Native quote styling |
| `image` | source, alt, caption, dimensions | Native image view with accessible description |
| `callout` | tone, children | Native bordered or tinted content panel |
| `card` | title, summary, image, href | Native linked card |
| `code` | language, literal text | Monospaced selectable text; syntax highlighting deferred |

A `web` block or page-level `mode: web` can carry a URL to Hugo output. Avoid embedding unsanitized arbitrary HTML in native mode.

### Shortcode rendering strategy

Treat a shortcode as input syntax, not as a native component. Hugo remains responsible for resolving it and producing the authoritative Website output. The client uses a progressive, explicit policy:

1. **Recognized shortcode → declarative native block.** Inventory real shortcodes from the pilot and prioritize common, content-shaped patterns such as callouts, cards, buttons, and image galleries. An adapter translates known parameters and supported children into the existing native block vocabulary (for example, `callout`, `card`, `image`, `link`, or `list`). Site-specific adapters may declare mappings, but they provide structured data only; neither Hugo template code nor arbitrary scripts/styles execute in the app.
2. **Unknown presentation, readable children → text fallback.** Preserve readable Markdown/text children and links when the shortcode's visual treatment is unsupported. Mark the presentation as partial so authors know the page is not fully represented.
3. **No safe native/text representation → explicit Website placeholder.** Keep the shortcode's position in the content flow and show its name when known, a concise reason, and a direct action to open the same page in Website mode. Never silently drop the content.

Track coverage per block as `native`, `text-fallback`, or `website-required`; the page is `mixed` when its blocks use more than one path. In Website mode Hugo's rendered page remains the source of truth. Prefer the order **text-only fallback → common declarative blocks → site-declared mappings → interactive blocks only when pilot evidence warrants them**. Start by classifying pilot shortcodes into simple data, readable children, and theme-dependent/interactive groups. Unknown shortcodes follow the Website placeholder path by default.

#### Future editing and round-trip contract

The manifest must preserve the original Markdown document as inert source text, including front matter, shortcode delimiters, whitespace, quoting, unknown parameters, and nested content. It records UTF-8 byte ranges and ordered raw parameter lexemes as navigation/editing hints; a decoded parameter map is never the source of truth. A future edit should replace only an explicitly selected source span and leave all untouched bytes unchanged. Raw source is never interpreted as HTML or executed.

Future shortcode forms should be metadata-driven for known adapters: a site/app declaration can supply field labels, input types, defaults, constraints, and help text, while the adapter maps validated values to declarative native blocks. Unknown shortcodes remain available through source editing and Website preview; the client must not invent a form from guessed parameter names. Nested shortcode child content stays separately addressable by source range so later editing can expose a visual child editor without losing the original text representation.

### Build and refresh flow

1. On repository selection, verify project root and Hugo availability; load settings.
2. Run an initial Hugo build that produces both regular output and the manifest.
3. Validate the manifest and populate the native content model.
4. Start watchers after initial configuration is known.
5. On relevant file events, debounce and coalesce paths; classify likely affected inputs only for diagnostics/optimization.
6. Start one Hugo build. If another change occurs, mark a follow-up build; do not run unbounded concurrent builds.
7. On success, atomically switch to the completed manifest/output snapshot, update the selected page, and notify WebView to reload if active.
8. On failure, retain the last successful preview and display current build diagnostics; clearly mark it stale.

Hugo is the dependency resolver. Do not build a fragile parallel template/content dependency engine in v1. Full rebuilds may be acceptable for initial correctness; profile representative repositories before adding finer-grained optimization.

### Filesystem watching

- Use OS-backed recursive notifications where supported.
- Maintain configurable ignored paths: `.git`, Hugo destination/cache directories, app temp directories, common editor temporary files, and user-provided ignores.
- Watch source content, config, layouts, assets, data, i18n, archetypes, and theme/module inputs relevant to the selected site.
- Use debounce with a configurable default around 150–300 ms and a maximum coalescing window to avoid starvation during continuous writes.
- Add a conservative reconciliation mechanism only if platform watchers can lose events; it must not become high-frequency whole-tree polling.
- Prevent output directories from causing rebuild loops.

### Process management

- Invoke Hugo as a child process with argument arrays, never through a shell string.
- Set the repository root as working directory and pass explicit flags for destination/cache/manifest output where supported.
- Capture stdout/stderr; tag output with build ID and timestamp.
- Bound concurrent builds to one per repository. Terminate or supersede only when safe and supported; otherwise queue a single latest build.
- Do not rely on undocumented internal Hugo APIs. Version-check any flags used and surface incompatibilities.

### Path and URL handling

- Treat manifest paths as untrusted input.
- Resolve local resources only inside approved repository/output roots unless the user explicitly configures otherwise.
- Normalize URL paths and reject traversal outside roots.
- Permit external navigation through an explicit external-link action; never silently open arbitrary external pages inside a privileged app context.
- Separate source asset paths from public output URLs.

### UI-independent data model (conceptual)

```text
SiteModel
  schema_version
  metadata
  navigation[]
  pages[]
  assets[]

PageModel
  stable_id
  source_path?
  public_path
  metadata
  render_mode: native | web | mixed
  blocks[]

Block
  kind
  typed_payload
  source_reference?
  fallback_policy
```

The native UI consumes this model, never raw Hugo template internals. Keep parsing, validation, and URL resolution in the core layer.

## 11. Risks and mitigations

| Risk | Mitigation |
|---|---|
| Users expect all themes to become native automatically | Describe opt-in support clearly; WebView fallback; show coverage status |
| Native and web views diverge | Hugo remains source of truth; document the supported block subset; provide side-by-side/manual comparison during development |
| Theme changes break manifest generation | Versioned schema, fixtures for supported themes, clear diagnostics |
| Rebuilding is slower than expected | Profile real sites, keep UI responsive, rely on Hugo first, optimize only measured bottlenecks |
| Custom shortcodes cannot map to native controls | Mark page mixed/web or allow explicit block adapters in a later extension mechanism |
| Cross-platform UI weakens “native” promise | Validate the Tauri system-WebView shell, accessibility, and platform conventions on macOS and Omarchy Linux |
| Watch loops or missed events | Exclude generated output; build IDs; coalescing; reconciliation checks |
| Unsafe repository paths or links | Root-constrained path resolution and explicit external-link handling |
| Product is only a preview wrapper | Validate editorial/review workflow value before expanding renderer scope |

## 12. Open decisions

- Does the pilot require OS-native widgets throughout, or is a native window, OS menus/dialogs, and system WebView acceptable for the shared workspace UI?
- Which real Hugo site and theme will be the pilot?
- Is the first high-value workflow content review, validation, asset discovery, or another task?
- Should the initial app be read-only, or is in-app metadata/content editing essential to adoption?
- Should the manifest be generated by Hugo templates, an official module, or a companion CLI?
- What content coverage makes a pilot useful: percentage of pages, block types, or a defined set of editorial tasks?
- Is system WebView fallback sufficient, or must the first release support launching the system browser?

## 13. Delivery stages

### Stage 0: Validate the problem

Select a real repository and observe the user's current content workflow. Inventory site structures and identify a recurring task that the client can improve. Establish a baseline for time, steps, and current preview latency.

### Stage 1: Technical spike

Generate a manifest from one representative Hugo site; validate schema evolution and supported blocks; prove watcher-to-build-to-refresh behavior; compare native and WebView fallback. No broad theme compatibility promise.

### Stage 2: Read-only MVP

Ship repository selection, content browser, native rendering for core blocks, validation, build diagnostics, and WebView fallback. Measure return usage and task completion in the pilot.

### Stage 3: Workflow depth

Based on pilot evidence, add the most valuable review/authoring functions, theme adapters, richer Markdown, search, or asset workflows. Consider editing only after deciding how edits remain compatible with front matter, bundles, and Git workflows.

### Development chunks and dependencies

Build in the sequence below. Chunks 2 and 3 can proceed in parallel after Chunk 1 establishes the product and platform boundary. Chunk 4's schema design can start after Chunk 1, while producing and validating the pilot manifest depends on Chunk 3. The compatibility reader in Chunk 6 depends on Chunks 3 and 5; its manifest branch also depends on Chunk 4. Chunk 5 can use mock data while the Hugo pipeline is under development. Each chunk ends with a reviewable artifact and a clear handoff to dependent work.

| Chunk | Scope and reviewable result | Depends on |
|---|---|---|
| **1. Pilot and product contract** | Select the pilot Hugo repository and review its config, menus, theme header/footer, content bundles, assets, and common blocks. Confirm the read-only review job, define what “native” means for the shell versus page renderer, and record MVP page/block coverage. Track kickoff decisions and pilot status in `docs/implementation-kickoff.md`. | None |
| **2. Platform and shell spike** | Validate the selected Tauri 2 approach on macOS and Omarchy Linux. Prove app launch, native window and folder-dialog behavior, menu/keyboard conventions, file watching, and system WebView availability. Record the custom web-based workspace tradeoff, packaging requirements, minimum window size, responsive behavior, and a static Peak SaaS inspired shell. | 1 |
| **3. Hugo repository and build adapter** | Implement repository selection and Hugo/config/version detection; run Hugo with controlled arguments; capture build output and errors; establish generated output and build lifecycle APIs. | 1 |
| **4. Manifest v1 and pilot producer** | Publish the JSON Schema and generate a manifest from the pilot when its compatibility coverage is known. Define stable page IDs, navigation/menu mapping, page metadata, blocks, assets, URL/date/language rules, render mode, fallback policy, unknown-field behavior, and optional declarative `site.chrome` fields. Preserve shortcode provenance and per-block native/text/Website disposition so unsupported content can be represented at its original position. Populate header/footer fields only when the pilot can describe them without arbitrary HTML; WebView remains the full-fidelity path. **Progress:** schema draft, internal pilot producer, runtime schema validation, and app consumption are implemented. A reusable site-side producer remains open. | 1, 3 |
| **5. App shell and workspace state** | Implement left rail, content navigation sidebar, tabs, central pane, context panel, and app status bar. Add panel collapse behavior and selection preservation at narrower widths. Populate with mock data until the manifest loader is ready. | 2 |
| **6. Hugo content/config reader and manifest loader** | Load resolved pages/config with the compatibility adapter; prefer and validate manifest v1 when present. Map menus into the sidebar, populate page lists and tabs, and expose metadata and diagnostics to the context panel. Reject unsafe paths and report malformed or unsupported data without crashing. | 3, 5 (manifest branch also requires 4) |
| **7. Native content renderer** | Render the agreed pilot Markdown subset in the central pane, including accessible links. Display declared site branding/header and footer only if Chunk 4 included fields in the schema; otherwise label native rendering as omitting theme chrome. Keep unknown or unsupported output at its content position with an explicit reason and Website action. Add declared adapters for prioritized shortcode patterns; preserve readable shortcode children as text fallback. Do not execute shortcode templates or arbitrary web code. **Progress:** SC-3 placeholders and SC-4 adapters for `ref`/`relref`, `button`, `lead`, `quick-answer`, and `faq`/`faq-item` are implemented. Figure/media adapters remain dependent on safe Hugo-resolved assets. | 4, 6 |
| **8. WebView fallback and mode switching** | Display Hugo-generated output for web-only or unsupported pages, switch directly from an unsupported shortcode placeholder to the matching page in Website mode, and identify the active mode. Verify theme header/footer and page output remain Hugo-rendered in this path. | 2, 3, 5, 6, 7 |
| **9. Watch, rebuild, and refresh** | Watch relevant inputs, debounce/coalesce changes, serialize builds, atomically adopt successful output/manifest snapshots, retain the last good view on failure, and preserve selected page and scroll position where feasible. Update the bottom status bar with current/stale and build states. | 3, 4, 5, 6, 7, 8 |
| **10. MVP diagnostics and hardening** | Complete actionable error reporting, required metadata and missing-asset checks, path/link safety, accessibility pass, and platform-specific packaging checks. Verify acceptance criteria on macOS and Omarchy Linux with the pilot repository; document limitations and pilot measurements. | 2, 6, 7, 8, 9 |

**Dependency path:** `1 → 2 and 3`; `1 → schema design for 4`; `3 → pilot generation for 4`; `2 → 5`; `3 + 4 + 5 → 6`; `6 → 7 and 8`; `7 + 8 → 9`; `9 → 10`. Chunk 5 can start with mock data as soon as Chunk 2 is complete.

#### Shortcode work sequence within the chunks

| Work item | Reviewable result | Depends on |
|---|---|---|
| **SC-1. Pilot shortcode inventory** | Classify shortcode instances as simple data, readable children, or theme-dependent/interactive; record frequency and example pages. **Complete:** 832 opening invocations across 31 names in 133 content files, with candidate mappings and examples recorded in [`shortcode-inventory.md`](shortcode-inventory.md). | 1 |
| **SC-2. Manifest representation** | Preserve exact source for future edits; expose shortcode token ranges, syntax, ordered raw/decoded parameter hints and per-token native/text/Website disposition. Add representative fixtures and runtime schema validation. | SC-1, 4 (pilot producer exists) |
| **SC-3. Text fallback and placeholders** | **Complete:** source-ordered content flow keeps native text around shortcode positions, shows a Website-required placeholder with readable child text, and provides a direct switch to the same page in Website mode. Text rendering retains link labels; original Markdown and link targets remain lossless in `sourceDocument.rawSource`. | SC-2, 6, 7, 8 |
| **SC-4. Priority adapters** | **Complete (first priority set):** map Hugo `ref`/`relref` and Blowfish `button` to safe native links, flatten `lead`, map `quick-answer` to a native callout, and render nested `faq`/`faq-item` as disclosure-based question/answer items. Adapted tokens and rendered items retain source ranges for future editing. Malformed/unresolved forms remain Website-required. Rust unit tests, pilot schema/load checks, and the macOS app build pass. Figure remains deferred until Hugo-resolved asset metadata is available; interactive patterns stay Website-first. | SC-2, 7, 8 |

`SC-1 → SC-2 → SC-3 → SC-4` is the first shortcode delivery path. SC-2 uses the existing internal pilot producer; a reusable site-side producer remains a separate Chunk 4 follow-on. No adapter may execute a template or arbitrary script/style payload. Next on the main plan is Chunk 2, Omarchy/Linux platform and shell validation, which also unblocks Chunk 5.

## 14. Acceptance criteria for MVP

- A new user can select a supported Hugo repository and see detected project/Hugo status without manual config editing.
- A successful Hugo load populates a browsable list from the resolved page index; a site-provided manifest can add richer page/block declarations.
- Hugo menu entries populate the content navigation sidebar with their declared labels, hierarchy, and ordering.
- Supported blocks render with accessible native controls and valid links/assets.
- Recognized pilot shortcodes map to declarative native blocks; readable children of unsupported shortcodes remain available as text fallback.
- Unsupported shortcode output stays visible in the native flow as a clearly identified placeholder with a direct action to the same page in Website mode. Native/partial/Website coverage is clear, and no shortcode template or arbitrary web code runs in native mode.
- Unsupported native structures are reported and remain accessible through the locally rendered Website WebView.
- WebView output retains the Hugo theme's header and footer. Native mode renders site chrome only when explicitly represented in the manifest, and makes any omission clear.
- A content or layout change refreshes the correct preview after one coalesced build; generated output does not create a rebuild loop.
- Build failures retain the last good preview and expose actionable diagnostics.
- No content files, Git state, or remote services are modified as a side effect.
- A pilot user can complete the chosen content-review task more efficiently than the current workflow, with the improvement measured and documented.

## 15. Decision summary

Proceed as a **local-first Hugo content client with an opt-in native content contract**, not as a universal HTML-to-native theme converter. Keep Hugo responsible for resolution and site output, build native workflows around structured content, and retain a system WebView fallback for full web fidelity. Validate a concrete editorial/review job on a real site before investing in broad native theme coverage.
