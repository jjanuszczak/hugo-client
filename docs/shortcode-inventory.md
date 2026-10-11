# Pilot shortcode inventory

**Status:** SC-1 complete; read-only audit
**Pilot:** `/Users/johnjanuszczak/Projects/ceo-playbook`
**Audited:** 10 October 2026

## Scope and method

Scanned the pilot's 198 Markdown files under `content/` for Hugo shortcode delimiters and grouped opening invocations by name. Closing delimiters are excluded from invocation counts. The scan found **832 opening invocations across 31 shortcode names in 133 content files**. Counts include Hugo's `ref` and `relref` link shortcodes because they use the same source syntax.

Inspected definitions in the pilot's `layouts/shortcodes/` and the checked-in Blowfish theme's `themes/blowfish/layouts/shortcodes/` where those definitions were present. Representative call sites were checked for parameters, children, and nesting. Inventory paths below are relative to the pilot repository. Definitions supplied only by an external Hugo module, if any, are marked unresolved rather than inferred.

## Simple data and readable-content candidates

These produce understandable text or structured values that could map to the native block vocabulary. “Candidate” means suitable for later adapter design, not already supported by the app.

| Shortcode | Calls / content files | Classification and observed shape | Recommended native treatment | Representative call site |
|---|---:|---|---|---|
| `ref` | 87 / 32 | **Simple data.** Hugo-resolved content reference, commonly used inline inside Markdown links. Built into Hugo; no local template. | Resolve to a regular inline link using Hugo's final URL and label. Do not preserve the shortcode syntax in the native model. | `content/advisory/corporate-innovation/index.md:18` |
| `relref` | 82 / 47 | **Simple data.** Hugo-resolved relative content reference, commonly used inline inside Markdown links. Built into Hugo; no local template. | Same as `ref`; retain whether the link is internal if useful for navigation. | `content/_index.md:26` |
| `quick-answer` | 44 / 44 | **Readable children.** One paired wrapper on each of 44 pages; `.Inner` is Markdown. Theme adds a fixed “Quick Answer” label and visual emphasis. | Keep child Markdown as a paragraph/callout body; optionally map to a standard `callout` block with a fixed title. No custom styles required. | `content/articles/10-million-inbox/index.md:52` |
| `faq` + `faq-item` | 44 + 130 / 44 | **Simple data + readable children.** A paired `faq` wrapper contains 130 paired question items across 44 pages. Each item has a `question` parameter and Markdown answer; the site wrapper also emits JSON-LD. | Normalize to an FAQ/list of question and answer blocks (or headings plus child blocks). Ignore the generated JSON-LD in native mode; Website output remains authoritative. | `content/articles/10-million-inbox/index.md:133-143` |
| `figure` | 124 / 53 | **Simple data.** Blowfish image shortcode with `src`, `alt`, optional Markdown `caption`, link, and presentation options. It may resolve page-bundle/global assets and transform images. | Map to an `image` block only after Hugo asset resolution supplies a safe preview URL; preserve alt/caption/link. Ignore theme classes and zoom behavior. | `content/articles/moats-vibe-coding/index.md:18` |
| `button` | 18 / 12 | **Simple data + readable label.** Blowfish emits a link styled as a button; `href` or `pageRef`, optional target/rel, and inner label. | Map to a normal accessible link/CTA block with resolved destination and label. | `content/_index.md:151` |
| `lead` | 11 / 11 | **Readable children.** Blowfish wraps `.Inner` Markdown in lead styling. | Flatten to the contained native paragraph/blocks; no special block is required. | `content/advisory/_index.md:12` |
| `feature-grid` + `feature` | 1 + 4 / 1 | **Simple data + readable children.** One grid contains four feature cards with icon, title, URL, label, heading level, and Markdown body. Blowfish renders icons and cards. | Map to a `card` collection; retain title, summary/body, href, and CTA label. Treat icon as optional allow-listed metadata or omit it. | `content/_index.md:112-125` |
| `stats` + `stat` | 1 + 3 / 1 | **Simple data + readable children.** One grid contains three metric values, labels, and brief descriptions. | Map to a metric/card collection using value, label, and child text; ignore grid styling. | `content/_index.md:145-149` |
| `alert` | 4 / 4 | **Readable children.** Blowfish renders `.Inner` Markdown and accepts icon/color parameters; one observed family is small. | Map the children to a `callout`; use a limited known tone mapping and ignore custom colors/icons in the first adapter. | `content/articles/air-space/index.md:22` |
| `accordion` + `accordionItem` | 3 + 6 / 3 | **Readable children.** Blowfish pairs a wrapper and items; each item has a title and Markdown content, with open/separated/icon options. Theme adds CSS and JavaScript behavior. | First pass: preserve item titles as headings and children as content, or use the app's own safe disclosure control. Do not import the theme's script/style behavior. | `content/lab/_index.md:13-17` |
| `notification-bar` | 1 / 1 | **Simple data.** Site shortcode has message, type, link, link text, and optional arbitrary background color; it emits inline styling and a script that changes page padding. | If prioritized, map message/link to a standard callout and a known tone; ignore `bgcolor` and all generated styling/script. Low occurrence. | `content/research/stablecoins/index.md:24` |
| `readwise` | 4 / 2 | **Simple data.** Site shortcode provides quote text, author, title, optional URL and image; template adds CSS and image treatment. | Could map to a quote with attribution and optional safe link. Treat remote image as optional; low occurrence. | `content/signals/signals-week-03-2026/index.md:62` |
| `social_links` | 1 / 1 | **Simple data via partial.** Site shortcode delegates to `author-links.html`; actual links come from site author/config data. | Reuse already parsed safe external-link data if available; otherwise use Website fallback. Low occurrence. | `content/contact.md:33` |

## Theme-dependent, interactive, or externally rendered

These rely on remote widgets, JavaScript, CSS, generated diagrams/charts, an iframe, or media playback. Keep them on the Website path by default. A later text fallback may expose a meaningful title or safe external link, but that is not equivalent to rendering the embedded experience.

| Shortcode | Calls / content files | Why it needs Website mode by default | Useful text/link fallback | Representative call site |
|---|---:|---|---|---|
| `x` | 132 / 47 | Site override creates an X blockquote and loads X's widgets script once per page. | “View @user's post on X” link; no native post content is present in the source. | `content/signals/signals-week-02-2026/index.md:14` |
| `x-article` | 56 / 29 | Site template emits inline CSS, remote image, and an external X article card. | Title and external “Read on X” link; omit remote image unless asset policy later permits it. | `content/signals/signals-week-14-2026/index.md:28` |
| `youtubeLite` | 27 / 26 | Blowfish custom element lazily embeds YouTube and relies on theme/browser behavior. | Video label plus a link to the YouTube video; optional thumbnail is a later decision. | `content/videos/fintech-to-energy-abundance/index.md:42` |
| `spotify` | 24 / 3 | Site template emits a Spotify iframe player. | Link to the Spotify item if a canonical URL can be formed safely. | `content/articles/top-podcasts-2025/index.md:41` |
| `site-stats` | 6 / 1 | Site template loads Chart.js and computes several sections from site data. | Section heading/description if available; charts remain Website-only. | `content/site-stats.md:18` |
| `mermaid` | 6 / 5 | Blowfish passes inner diagram syntax through as HTML for client-side Mermaid rendering. | Preserve diagram source as fenced text only if it can be extracted without interpreting it; otherwise placeholder with Website action. | `content/articles/building-an-audience/index.md:84` |
| `giscus` | 2 / 2 | Site template loads an external discussion script and embeds interactive comments. | “Open discussion” link only if a canonical discussion URL can be derived. | `content/research/gbri/index.md:454` |
| `katex` | 2 / 1 | Blowfish shortcode itself is empty; the page relies on theme math handling/assets. The actual source/render contract needs confirmation before considering a native math block. | Preserve surrounding prose and use Website mode for formulas. | `content/research/gbri/index.md:109` |
| `margo` | 2 / 2 | Site template embeds a local presentation in an iframe and adds JavaScript controls, CSS, and a dialog. | Presentation title/caption plus a link to open it. | `content/about.md:109` |
| `google-contact-form` | 1 / 1 | Site template emits a form, hidden iframe, inline JavaScript, and a configured remote submission action. | “Open contact page in Website mode”; do not recreate or submit forms natively. | `content/contact.md:21` |
| `video` | 1 / 1 | Theme template resolves local/remote media, poster, ratio, and playback options into a video element. | Video title/source link if available; otherwise Website mode. | `content/portfolio/voltai/index.md:25` |
| `github` | 2 / 2 | **Theme-dependent and network-backed.** Blowfish fetches repository metadata from the GitHub API during Hugo rendering and builds a repository card with remote thumbnail support. | Use Website mode by default; a later static link card could use the `repo` parameter without fetching metadata. | `content/lab/crm-llm/index.md:19` |

## Inventory-only / resolve before adapter design

| Shortcode | Calls / content files | Finding | Next action |
|---|---:|---|---|
| `include` | 3 / 1 | Site override resolves a page-bundle resource by name and inserts its `.Content`. The included content can vary and may contain more Markdown or HTML. | Inspect the three included resources and define a recursive safe-content policy before supporting it; Website fallback otherwise. | `content/articles/pushkin/index.md:84` |

## Recommended adapter order

1. **High value and low execution risk:** Hugo `ref`/`relref` to inline links; `button` to accessible links; `lead` and `quick-answer` to regular child text/callout blocks. These retain readable content and do not need theme scripts.
2. **Structured content:** `faq`/`faq-item` to question/answer blocks; `feature-grid`/`feature` and `stats`/`stat` to card/metric collections. Ignore generated JSON-LD, icons, and decorative styling.
3. **Asset-aware blocks:** `figure` to the existing image block after the adapter receives Hugo-resolved asset paths and alt/caption/link metadata. This is the highest-volume visual shortcode family (124 calls), but asset resolution must be correct first.
4. **Small text-first additions:** Blowfish `alert`, `accordion`/`accordionItem`, site `notification-bar`, and `readwise`. Preserve content and known fields; do not reproduce arbitrary theme colors, CSS, or scripts.
5. **Website-first:** keep X, X articles, YouTube, Spotify, charts, Giscus, Margo, forms, video, Mermaid, and unresolved GitHub cards on Website mode, with a safe text/link placeholder where source data supports one. Reconsider only with a user need and a declarative data contract.

The first pass should prioritize source-preserving semantics over visual similarity. Every recognized call should either produce a declared native block, preserve its readable children with a partial-coverage marker, or leave a positioned Website fallback. Never execute shortcode templates or generated HTML/CSS/JavaScript in the native renderer.

## Follow-up questions for SC-2

- How will Hugo-resolved `ref`/`relref` destinations be provided to the app so links are correct across language prefixes and aliases?
- Will nested shortcode output be normalized by a site-side manifest producer, a Hugo-generated intermediate representation, or a deliberately limited Markdown parser?
- What asset metadata must Hugo provide for `figure` (resolved path, alt, caption, dimensions, link, and bundle identity)?
- Should a partially mapped shortcode block preserve just text, or may it also preserve safe links and nested supported blocks?
