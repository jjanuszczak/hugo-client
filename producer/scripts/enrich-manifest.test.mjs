import test from "node:test";
import assert from "node:assert/strict";
import { enrichManifest, sourceDocumentAndFlow, tokenizeShortcodes } from "./enrich-manifest.mjs";

test("shortcode byte ranges preserve UTF-8 source and quoted delimiters", () => {
  const source = Buffer.from('---\ntitle: "世界"\n---\ncafé — {{< card title="a > b" odd=\'x y\' >}}inner{{< /card >}}\n');
  const tokens = tokenizeShortcodes(source);
  assert.equal(tokens.length, 2);
  const start = source.indexOf(Buffer.from("{{< card"));
  const end = source.indexOf(Buffer.from(">}}"), source.indexOf(Buffer.from('odd=\'x y\''))) + 3;
  assert.equal(tokens[0].sourceRange.startByte, start);
  assert.equal(tokens[0].sourceRange.endByte, end);
  assert.equal(source.subarray(start, end).toString("utf8"), tokens[0].raw);
  assert.deepEqual(tokens[0].parameters, [
    { kind: "named", name: "title", value: "a > b", raw: 'title="a > b"' },
    { kind: "named", name: "odd", value: "x y", raw: "odd='x y'" },
  ]);
  assert.equal(tokens[0].sourceRange.startLine, 4);
});

test("source flow keeps paired shortcode placeholders at the correct position", () => {
  const source = Buffer.from("---\ntitle: fixture\n---\nBefore.\n{{% outer %}}child {{% inner /%}} tail{{% /outer %}} After.\n");
  const result = sourceDocumentAndFlow(source);
  assert.deepEqual(result.contentFlow.map((item) => item.type), ["text", "shortcode", "text"]);
  assert.equal(result.contentFlow[1].name, "outer");
  assert.equal(result.contentFlow[1].disposition, "website-required");
  assert.deepEqual(result.contentFlow[1].readableChildren[0], {
    type: "paragraph",
    inlines: [{ type: "text", text: "child  tail" }],
  });
  assert.equal(result.sourceDocument.shortcodeTokens.length, 3);
  assert.ok(result.sourceDocument.shortcodeTokens.every((token) => token.disposition === "website-required"));
  const range = result.contentFlow[1].sourceRange;
  assert.equal(source.subarray(range.startByte, range.endByte).toString("utf8"), "{{% outer %}}child {{% inner /%}} tail{{% /outer %}}");
});

test("manifest enrichment attaches lossless source data and downgrades unsafe native claims", () => {
  const source = Buffer.from("---\ntitle: fixture\n---\n{{< custom >}}body{{< /custom >}}\n");
  const manifest = {
    schemaVersion: 1,
    site: { title: "Fixture" },
    pages: [{ id: "content/page.md", sourcePath: "content/page.md", title: "Fixture", path: "/page/", rendering: { mode: "native" } }],
  };
  const result = enrichManifest(manifest, "/site", (sourcePath) => {
    assert.equal(sourcePath, "content/page.md");
    return source;
  });
  assert.equal(result.pages[0].sourceDocument.rawSource, source.toString("utf8"));
  assert.equal(result.pages[0].sourceDocument.bodyStartByte, source.indexOf(Buffer.from("{{< custom")));
  assert.equal(result.pages[0].rendering.mode, "mixed");
  assert.equal(result.pages[0].contentFlow[0].type, "shortcode");
});

test("malformed unclosed shortcode stays in raw source and becomes Website-required", () => {
  const source = Buffer.from("---\ntitle: fixture\n---\nBefore.\n{{< broken title=\"unfinished\nmore text");
  const result = sourceDocumentAndFlow(source);
  assert.equal(result.sourceDocument.shortcodeTokens.length, 1);
  assert.equal(result.sourceDocument.shortcodeTokens[0].raw, '{{< broken title="unfinished\nmore text');
  assert.equal(result.sourceDocument.shortcodeTokens[0].disposition, "website-required");
  assert.deepEqual(result.contentFlow.map((item) => item.type), ["text", "shortcode"]);
  assert.equal(result.contentFlow[1].sourceRange.endByte, source.length);
});

test("safe complete block declaration remains native without unsupported source", () => {
  const manifest = {
    schemaVersion: 1,
    site: { title: "Fixture" },
    pages: [{ id: "content/page.md", sourcePath: "content/page.md", title: "Fixture", path: "/page/", rendering: { mode: "native" } }],
  };
  enrichManifest(manifest, "/site", () => Buffer.from("---\ntitle: fixture\n---\n## Safe heading\n"));
  assert.equal(manifest.pages[0].rendering.mode, "native");
  assert.equal(manifest.pages[0].contentFlow[0].blocks[0].type, "heading");
});
