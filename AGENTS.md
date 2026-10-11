# Project instructions for agents and developers

## Start here

This repository builds **Hugo Native Content Client**, a local-first desktop app for reviewing a Hugo site. It targets macOS and Linux, with Omarchy as the first Linux environment. The current app uses a Rust core and Tauri 2; the shared Peak-inspired workspace is HTML/CSS/JavaScript inside the platform WebView, not an OS-native widget toolkit.

Before changing code:

1. Read [`docs/hugo-native-client-prd-spec.md`](docs/hugo-native-client-prd-spec.md) for product requirements, rendering boundaries, and the full ten-chunk development plan.
2. Read [`docs/implementation-kickoff.md`](docs/implementation-kickoff.md) for current progress, pilot details, verified commands, and the active handoff.
3. Read [`docs/shortcode-inventory.md`](docs/shortcode-inventory.md) before changing shortcode behavior.
4. Check `git status --short`. This workspace can contain in-progress user work. Preserve it; do not reset, clean, or revert unrelated changes.

The docs above are the project source of truth for product decisions. Update the kickoff when implementation status, verification, or the next handoff changes. Update the PRD when product scope or the planned sequence changes.

## Product contract and safety rules

- The app is a **read-only content reviewer** in the current MVP. Do not write to a selected Hugo site's content, config, layouts, assets, or Git state as a side effect.
- Hugo owns site semantics. Invoke Hugo to resolve config, pages, menus, and rendered Website output. Do not recreate Hugo's template engine in the app.
- The Website view is the fidelity reference. The Native view renders only explicitly supported, declarative content and must identify unsupported content rather than implying it rendered completely.
- Never execute Hugo shortcode templates, generated HTML, scripts, or CSS in the native content renderer. Keep interactive/theme-dependent shortcodes on the Website path unless a reviewed safe adapter is added.
- Keep all file reads within the selected repository root. Use controlled process argument arrays, not shell-interpolated Hugo commands.
- Build output, Hugo cache, and processed resources should use app-owned temporary paths where the existing implementation supports it. Do not alter this isolation casually.
- Keep original Markdown as inert source text. `sourceDocument.rawSource` is the authority for future edits; shortcode tokens, decoded parameters, and rendered flow are derived hints. Preserve byte ranges, raw lexemes, quoting, order, unknown parameters, whitespace, and nested source. Do not implement editing by serializing the decoded manifest back over source.
- Add a native shortcode adapter only when it can return safe structured data. Preserve unknown, malformed, or unresolved tokens as Website-required placeholders at their source position.
- Do not claim Linux/Omarchy support based on a macOS build. Record which host and checks actually ran.

## Architecture map

- `src-tauri/src/lib.rs`: Hugo detection/invocation, project loading, preview server, manifest producer, source-document/token capture, shortcode-to-content-flow conversion, schema validation, and Rust tests.
- `src-tauri/src/main.rs`: Tauri application entry point.
- `src-tauri/tauri.conf.json`: Tauri window, security policy, frontend build, and bundle settings.
- `frontend/app.js`: Peak-inspired workspace, manifest-to-view mapping, page navigation, Native/Website switching, and rendering of content-flow items.
- `frontend/styles.css`: workspace and content styling.
- `frontend/index.html`: browser shell entry point.
- `schemas/hugo-client-manifest-v1.schema.json`: contract validated by the Rust producer at runtime. Keep producer, schema, and consumer changes aligned.
- `docs/hugo-native-client-prd-spec.md`: product requirements, data and rendering contract, risks, acceptance criteria, and chunk dependencies.
- `docs/implementation-kickoff.md`: current execution log and task handoff.
- `docs/shortcode-inventory.md`: pilot shortcode evidence and adapter recommendations.
- `docs/peak-saas-layout.jpeg`: visual layout reference. Follow its workspace regions while retaining platform window, menu, dialog, and keyboard conventions.

For shortcode work, trace the path end to end: `source_document` captures lossless source and token ranges; `source_content_flow` creates the ordered safe rendering model; `apply_adapter_dispositions` marks only represented tokens; the JSON Schema validates the manifest; and `frontend/app.js` renders the declared flow. Keep source positions and Website fallback behavior intact across all layers.

## Current implementation and status

The verified MVP path opens a real Hugo repository, calls Hugo for merged config and resolved pages, shows page metadata and main/footer menus, produces and validates Manifest v1, renders a conservative Markdown subset, and starts an isolated local Hugo preview for Website mode. The app has a Peak-inspired left rail, content navigation, tabs, central pane, context panel, and status area.

The first shortcode set is implemented:

- Hugo `ref` and `relref` become links when resolvable.
- Blowfish `button` becomes a safe link when its target resolves.
- `lead` children flatten into native text blocks.
- `quick-answer` becomes a callout.
- Nested `faq`/`faq-item` become question and answer data displayed as disclosures.
- Unknown, malformed, unresolved, and interactive/theme-dependent shortcodes remain Website-required. `figure` is deferred until Hugo-resolved asset metadata is available.

The exact next planned work is **Chunk 2: platform and shell validation on Omarchy/Linux**. The macOS app has built; Omarchy packaging and runtime have not been verified. Chunk 2 depends on Chunk 1 and blocks Chunk 5. See the dependency table and plan in the PRD before changing the sequence. Chunk 4 is only partly closed: the internal pilot producer exists, but a reusable site-side producer and its fixtures remain open.

The current pilot is `/Users/johnjanuszczak/Projects/ceo-playbook`. Treat it as read-only. Its Hugo output, menus, and shortcode cases are useful integration evidence, but paths in that external repository must not be edited by this project. Latest recorded pilot load: 139 pages, 13 main-menu items, and 2 footer-menu items. Earlier desktop review recorded 138 page rows, so distinguish backend pilot-test output from a visual UI observation when reporting counts.

## Build and verification commands

From the repository root:

```sh
npm install
npm run dev                 # browser-only shell with demo data
npm run build               # static browser shell
npm run app:dev             # desktop app
npm run app:build           # macOS .app bundle on macOS
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml --offline
```

The opt-in pilot integration test starts a loopback preview server and may need permission to bind a local port:

```sh
HUGO_CLIENT_PILOT_PATH=/Users/johnjanuszczak/Projects/ceo-playbook \
  cargo test --manifest-path src-tauri/Cargo.toml --offline \
  loads_a_real_hugo_site_when_pilot_path_is_set -- --nocapture
```

The pilot path is machine-specific; if it is missing, use another Hugo test repository only with the user's direction and keep it read-only. `git diff --check` is a useful whitespace check. Do not report a check as passing unless it actually ran. Respect the task's instruction about whether to run tests/builds; do not add tests unrelated to the change.

On Linux, Tauri requires WebKitGTK 4.1 and platform development libraries. For Omarchy, check the current Tauri v2 Linux prerequisites and the installed Arch packages rather than assuming a macOS dependency list applies unchanged. Build Linux bundles on Linux, then record package type, launch result, folder-dialog behavior, menus/keyboard behavior, WebView operation, and any missing dependencies in the kickoff. Do not mark Chunk 2 complete until a real Omarchy runtime review has happened.

## Working and handoff practices

- Make the smallest change that satisfies the active chunk and its acceptance criteria. Keep product scope aligned with the PRD.
- Prefer changes that leave the repository safe to inspect and easy to recover. Never discard pre-existing user edits.
- When changing a manifest field or content-flow type, update the Rust producer, schema, frontend consumer, and focused fixtures/tests together as needed.
- Keep accessibility in view: keyboard operation, accessible names for icon controls, visible selection/focus, and state that is not conveyed by color alone.
- Keep Native and Website modes clearly labeled. A Website action from a placeholder should open the same page in Website mode.
- At chunk completion, update `docs/implementation-kickoff.md` with scope completed, actual verification, open limitations, and the next dependency-aware handoff. Do not mark a whole numbered chunk complete if only one subtask is done.
- Do not create commits, publish releases, or modify the pilot site unless the user explicitly asks.
