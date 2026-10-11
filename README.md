# Hugo Native Client

Local-first desktop workspace for reviewing content in a Hugo repository.

## Current build

The browser preview is a static, interactive shell with illustrative Northstar content. Real repository loading runs in the Tauri desktop app.

```sh
npm run dev
```

Open the printed local URL. To build the static browser preview:

```sh
npm run build
npm run preview
```

## Desktop app

The desktop shell uses Rust and Tauri 2. Install Node.js, Rust/Cargo, and Tauri's platform prerequisites. Then run:

```sh
npm install
npm run app:dev
```

Build the macOS `.app` bundle with `npm run app:build`. Linux packages can be built on Omarchy with `npx tauri build --bundles deb,appimage` after installing the platform prerequisites below.

On macOS, Tauri uses the system WKWebView. Linux builds require WebKitGTK 4.1 and the distribution's development packages. For Arch-based Omarchy systems, install `webkit2gtk-4.1`, `base-devel`, `curl`, `wget`, `file`, `openssl`, `appmenu-gtk-module`, `libappindicator-gtk3`, `librsvg`, and `xdotool` before building. See [Tauri's platform prerequisites](https://v2.tauri.app/start/prerequisites/) for current package names.

When you open a repository, the Rust adapter calls `hugo config` and `hugo list all` to load resolved page metadata and menus, then reads supported Markdown blocks from files inside that repository. It starts Hugo's local preview on loopback for full theme output. The preview output, cache, and processed resources go into an app-owned temporary directory; the site source is not edited.

## Try the pilot site

On macOS, run `npm run app:dev`, choose **Open Hugo repository**, and select `/Users/johnjanuszczak/Projects/ceo-playbook`. Open an article with shortcodes or tables to compare **Native** with **Website** mode, then open **Menus** to inspect the nested Hugo navigation. Native mode shows the supported Markdown subset and flags content it cannot represent; Website mode uses Hugo's rendered output and keeps the theme chrome.

The Rust integration test can exercise the same site without opening the desktop UI:

```sh
HUGO_CLIENT_PILOT_PATH=/Users/johnjanuszczak/Projects/ceo-playbook \
  cargo test --manifest-path src-tauri/Cargo.toml
```

The macOS app bundle and pilot integration test have been verified. Omarchy/Linux packaging and runtime have not yet been validated on a Linux host.

The native renderer supports headings, paragraphs, quotes, simple list items, and the first declarative shortcode set: `ref`/`relref`, `button`, `lead`, `quick-answer`, and `faq`/`faq-item`. Unknown shortcodes, raw HTML, code, tables, and media remain Website-required; switch to Website mode to see Hugo's full theme rendering. Automatic file watching and richer manifest-backed blocks are still in development; the Refresh button reloads the selected site on demand.

## Design and implementation notes

- Product requirements and the 10-chunk plan: [`docs/hugo-native-client-prd-spec.md`](docs/hugo-native-client-prd-spec.md)
- Current kickoff contract and pilot status: [`docs/implementation-kickoff.md`](docs/implementation-kickoff.md)
- Manifest schema draft: [`schemas/hugo-client-manifest-v1.schema.json`](schemas/hugo-client-manifest-v1.schema.json)
- Visual direction: [`docs/peak-saas-layout.jpeg`](docs/peak-saas-layout.jpeg)

To resume implementation, start with the **Handoff for the next developer or agent** section in `docs/implementation-kickoff.md`. SC-1 through SC-4's first adapter set are complete. The next planned work is Chunk 2: Omarchy/Linux platform and runtime validation. SC-1 findings are in `docs/shortcode-inventory.md`.

The working pilot is `/Users/johnjanuszczak/Projects/ceo-playbook`, a local Hugo site with nested menus, substantial content, and theme-specific shortcodes. The Rust integration test loads its resolved page index, menus, and local Hugo preview without modifying the site. The browser-only preview remains labeled demo data and must not be treated as repository output.
