# Hugo Client Manifest v1 producer

This reusable Hugo output format targets Hugo 0.146 or later and writes `public/hugo-client-manifest.json` from Hugo's resolved site model. It exports the site title and language, recursive main and footer menus, and resolved page identity, URL, title, summary, kind, section, language, draft state, date, and tags/categories.

The Hugo layout exports metadata and author-declared blocks; the Node enrichment step reads original Markdown and parses only a conservative plain text subset for ordered content flow. Hugo remains responsible for its normal Website output. The client never executes shortcode templates or accepts rendered `.Content` as native content, and the producer does not emit arbitrary HTML, CSS, or JavaScript into the manifest.

Sites may optionally add `data/hugo_client_manifest.json` to declare image assets and plain site chrome. The producer accepts only local absolute image paths for assets, plain text fields, and safe link schemes (`/`, HTTP(S), `mailto:`, and `tel:`). It ignores unknown fields and invalid entries. These declarations do not contain HTML or scripts.

## Declaring blocks

A page may declare `hugoClient.blocks` in its front matter. The producer currently accepts `heading`, `paragraph`, `quote`, and `list` blocks and ignores other block types. These are data fields, not rendered Hugo output. Declarations default to `rendering.mode: "mixed"`, which keeps the Website path available for content the declaration does not represent.

Set `hugoClient.complete: true` only when the declared blocks represent the entire page content. The producer then marks the page `native`; this is an explicit assertion by the site author. The producer cannot verify that assertion. The source-enrichment script reads original Markdown as inert bytes, records exact source, shortcode lexemes, ordered raw parameter hints, byte and line ranges, and Website-required dispositions. It downgrades a native claim to mixed when a source shortcode or unsupported Markdown is present.

Example:

```yaml
---
title: About
hugoClient:
  blocks:
    - type: heading
      level: 2
      text: Our work
    - type: paragraph
      inlines:
        - type: text
          text: A short, explicitly declared description.
---
```

## Install

Copy the `config/` and `layouts/` directories from this folder into the Hugo site root, merging them with existing directories. Merge `hugoClientManifest` into any existing `outputFormats` configuration and append it to the site's existing `outputs.home` list so other output formats remain enabled. Copy `scripts/enrich-manifest.mjs` into the site's scripts folder as well. The included `outputs.toml` is a minimal example; it is not a safe replacement for an existing outputs list. The recursive menu partial includes nested menu entries while retaining their parent identifiers.

Build the site with Hugo:

```sh
hugo
```

The JSON file will be written to `public/hugo-client-manifest.json`. Enrich it with exact source provenance and validate it against the checked-in schema:

```sh
node scripts/enrich-manifest.mjs --root . --manifest public/hugo-client-manifest.json
cargo run --manifest-path /path/to/hugo-client/src-tauri/Cargo.toml \
  --bin validate_manifest -- public/hugo-client-manifest.json
```

The minimal fixture is self-contained. Copy it to a temporary location, then build it there so Hugo's generated resources, cache, and output stay outside the repository:

```sh
fixture=$(mktemp -d /tmp/hugo-client-fixture.XXXXXX)
cp -R producer/fixtures/minimal-site/. "$fixture/"
mkdir -p "$fixture/layouts/_partials/hugo-client"
cp producer/layouts/_partials/hugo-client/menu-items.html \
  "$fixture/layouts/_partials/hugo-client/"
hugo --source "$fixture" \
  --destination "$fixture/public" \
  --cacheDir "$fixture/cache"
node producer/scripts/enrich-manifest.mjs \
  --root "$fixture" \
  --manifest "$fixture/public/hugo-client-manifest.json"
cargo run --manifest-path src-tauri/Cargo.toml --bin validate_manifest -- \
  "$fixture/public/hugo-client-manifest.json"
```

## Fixture

`fixtures/minimal-site/` is a small site demonstrating nested main navigation, footer navigation, page metadata, date handling, mixed/native author-declared blocks, a paired Unicode shortcode with quoted delimiters, safe site chrome, and a local image asset. `fixtures/expected-manifest.json` records the enriched Manifest v1 output, including source ranges and fallback flow. Absolute canonical URLs in generated output follow the fixture's `baseURL`.
