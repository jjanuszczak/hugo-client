#!/usr/bin/env node

import { readFile, realpath, rename, writeFile } from "node:fs/promises";
import { readFileSync, realpathSync } from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

const OPEN = Buffer.from("{{");

function fail(message) {
  throw new Error(message);
}

function parseArgs(args) {
  const values = new Map();
  for (let i = 0; i < args.length; i += 1) {
    const option = args[i];
    if (!new Set(["--root", "--manifest"]).has(option) || !args[i + 1]) {
      fail("Usage: node enrich-manifest.mjs --root SITE_ROOT --manifest MANIFEST_JSON");
    }
    if (values.has(option)) fail(`Option supplied more than once: ${option}`);
    values.set(option, args[++i]);
  }
  if (!values.has("--root") || !values.has("--manifest")) {
    fail("Usage: node enrich-manifest.mjs --root SITE_ROOT --manifest MANIFEST_JSON");
  }
  return { root: values.get("--root"), manifest: values.get("--manifest") };
}

function isWithin(root, candidate) {
  const relative = path.relative(root, candidate);
  return relative === "" || (!relative.startsWith(`..${path.sep}`) && relative !== ".." && !path.isAbsolute(relative));
}

function sourceBodyStart(source) {
  const firstNewline = source.indexOf(0x0a);
  if (firstNewline < 0) return 0;
  const first = source.subarray(0, firstNewline).toString("utf8").replace(/\r$/, "");
  const close = new Set(["---", "+++", ";;;"]).has(first) ? Buffer.from(first) : null;
  if (!close) return 0;
  const closeAt = source.indexOf(close, firstNewline + 1);
  if (closeAt < 0) return 0;
  const end = closeAt + close.length;
  return source[end] === 0x0a ? end + 1 : end;
}

function lineStarts(source) {
  const starts = [0];
  for (let i = 0; i < source.length; i += 1) {
    if (source[i] === 0x0a) starts.push(i + 1);
  }
  return starts;
}

function lineAt(starts, byteOffset) {
  let low = 0;
  let high = starts.length;
  while (low < high) {
    const middle = (low + high) >>> 1;
    if (starts[middle] <= byteOffset) low = middle + 1;
    else high = middle;
  }
  return Math.max(1, low);
}

function sourceRange(starts, startByte, endByte) {
  return {
    startByte,
    endByte,
    startLine: lineAt(starts, startByte),
    endLine: lineAt(starts, endByte),
  };
}

function shortcodeEnd(source, from, close) {
  let quote = 0;
  for (let i = from; i <= source.length - close.length; i += 1) {
    const byte = source[i];
    if (quote) {
      if (byte === quote) quote = 0;
      continue;
    }
    if (byte === 0x22 || byte === 0x27) {
      quote = byte;
      continue;
    }
    if (source.subarray(i, i + close.length).equals(close)) return i + close.length;
  }
  return -1;
}

function splitShortcodeWords(text) {
  const words = [];
  let word = "";
  let quote = "";
  for (const character of text) {
    if (quote) {
      word += character;
      if (character === quote) quote = "";
    } else if (character === "\"" || character === "'") {
      quote = character;
      word += character;
    } else if (/\s/u.test(character)) {
      if (word) words.push(word);
      word = "";
    } else {
      word += character;
    }
  }
  if (word) words.push(word);
  return words;
}

function parametersFrom(words) {
  return words.map((raw) => {
    const separator = raw.indexOf("=");
    const name = separator >= 0 ? raw.slice(0, separator) : null;
    const value = separator >= 0 ? raw.slice(separator + 1) : raw;
    return {
      kind: name === null ? "positional" : "named",
      name,
      value: value.replace(/^["']|["']$/g, ""),
      raw,
    };
  });
}

export function tokenizeShortcodes(source, bodyStart = sourceBodyStart(source)) {
  const lines = lineStarts(source);
  const tokens = [];
  let cursor = bodyStart;
  while (cursor < source.length) {
    const start = source.indexOf(OPEN, cursor);
    if (start < 0) break;
    const syntaxStart = source[start + 2];
    const syntax = syntaxStart === 0x3c ? "angle" : syntaxStart === 0x25 ? "percent" : null;
    if (!syntax) {
      cursor = start + OPEN.length;
      continue;
    }
    const close = Buffer.from(syntax === "angle" ? ">}}" : "%}}");
    const contentStart = start + 3;
    const end = shortcodeEnd(source, contentStart, close);
    if (end < 0) {
      const rawInner = source.subarray(contentStart).toString("utf8").trim();
      const words = splitShortcodeWords(rawInner);
      if (words.length) {
        const rawName = words[0];
        const closing = rawName.startsWith("/");
        tokens.push({
          type: "shortcode",
          name: rawName.replace(/^\/+/, ""),
          syntax,
          closing,
          selfClosing: rawInner.endsWith("/"),
          raw: source.subarray(start).toString("utf8"),
          parameters: parametersFrom(words.slice(1)),
          disposition: "website-required",
          sourceRange: sourceRange(lines, start, source.length),
        });
      }
      break;
    }
    const contentEnd = end - close.length;
    const rawInner = source.subarray(contentStart, contentEnd).toString("utf8").trim();
    const words = splitShortcodeWords(rawInner);
    if (words.length) {
      const rawName = words[0];
      const closing = rawName.startsWith("/");
      const name = rawName.replace(/^\/+/, "");
      tokens.push({
        type: "shortcode",
        name,
        syntax,
        closing,
        selfClosing: rawInner.endsWith("/"),
        raw: source.subarray(start, end).toString("utf8"),
        parameters: parametersFrom(words.slice(1)),
        disposition: "website-required",
        sourceRange: sourceRange(lines, start, end),
      });
    }
    cursor = end;
  }
  return tokens;
}

function hasClosingToken(tokens, currentIndex, name) {
  return tokens.slice(currentIndex + 1).some((token) => token.name === name && token.closing);
}

function shortcodeSpans(tokens) {
  const opens = [];
  const spans = [];
  for (let i = 0; i < tokens.length; i += 1) {
    const token = tokens[i];
    const { startByte: start, endByte: end } = token.sourceRange;
    if (token.closing) {
      const match = opens.findLastIndex((entry) => entry.name === token.name);
      if (match >= 0) {
        const [outer] = opens.splice(match, 1);
        if (match === 0) spans.push({ start: outer.start, end, name: outer.name });
      } else {
        spans.push({ start, end, name: token.name });
      }
    } else if (token.selfClosing || !hasClosingToken(tokens, i, token.name)) {
      spans.push({ start, end, name: token.name });
    } else {
      opens.push({ start, end, name: token.name });
    }
  }
  for (const open of opens) spans.push({ start: open.start, end: open.end, name: open.name });
  const ordered = spans.sort((a, b) => a.start - b.start || b.end - a.end);
  return ordered.filter((span, index) => !ordered.slice(0, index).some((outer) => outer.start <= span.start && outer.end >= span.end));
}

function readableChildren(source, span, tokens) {
  const nested = tokens.filter((token) =>
    token.sourceRange.startByte >= span.start && token.sourceRange.endByte <= span.end,
  );
  if (nested.length < 2 || nested[0].closing) return [];
  const start = nested[0].sourceRange.endByte;
  const end = nested[nested.length - 1].closing
    ? nested[nested.length - 1].sourceRange.startByte
    : span.end;
  if (end <= start) return [];
  const chunks = [];
  let cursor = start;
  for (const token of nested.slice(1, -1)) {
    if (token.sourceRange.startByte > cursor) {
      chunks.push(source.subarray(cursor, token.sourceRange.startByte));
    }
    cursor = token.sourceRange.endByte;
  }
  if (cursor < end) chunks.push(source.subarray(cursor, end));
  return markdownBlocks(Buffer.concat(chunks)).blocks;
}

function markdownBlocks(bytes) {
  const lines = bytes.toString("utf8").split(/\r?\n/u);
  const blocks = [];
  let paragraph = [];
  let listItems = [];
  let unsupported = false;
  let inCode = false;
  let rawTextTag = "";
  const flushParagraph = () => {
    if (paragraph.length) blocks.push({ type: "paragraph", inlines: [{ type: "text", text: paragraph.join(" ") }] });
    paragraph = [];
  };
  const flushList = () => {
    if (listItems.length) {
      blocks.push({
        type: "list",
        ordered: false,
        items: listItems.map((text) => [{ type: "paragraph", inlines: [{ type: "text", text }] }]),
      });
    }
    listItems = [];
  };
  for (const line of lines) {
    const trimmed = line.trim();
    const lower = trimmed.toLowerCase();
    if (rawTextTag) {
      if (lower.includes(`</${rawTextTag}>`)) rawTextTag = "";
      unsupported = true;
      continue;
    }
    if (/^(?:```|~~~)/u.test(trimmed)) {
      flushParagraph();
      flushList();
      inCode = !inCode;
      unsupported = true;
      continue;
    }
    if (inCode) continue;
    if (!trimmed) {
      flushParagraph();
      flushList();
      continue;
    }
    if (/^<\/?(?:style|script)(?:\s|>)/u.test(lower)) {
      flushParagraph();
      flushList();
      unsupported = true;
      const tag = lower.startsWith("<style") ? "style" : "script";
      if (!lower.includes(`</${tag}>`)) rawTextTag = tag;
      continue;
    }
    if (/^\|\s/u.test(trimmed) || /^<\/?[A-Za-z!]/u.test(trimmed) || /^!/u.test(trimmed)) {
      flushParagraph();
      flushList();
      unsupported = true;
      continue;
    }
    const heading = /^(#{1,6})\s+(.+)$/u.exec(trimmed);
    if (heading) {
      flushParagraph();
      flushList();
      blocks.push({ type: "heading", level: heading[1].length, text: heading[2].trim() });
      continue;
    }
    if (trimmed.startsWith("> ")) {
      flushParagraph();
      flushList();
      blocks.push({ type: "quote", text: trimmed.slice(2) });
      continue;
    }
    if (/^[-*+]\s/u.test(trimmed)) {
      flushParagraph();
      listItems.push(trimmed.slice(2));
      continue;
    }
    flushList();
    paragraph.push(trimmed);
  }
  flushParagraph();
  flushList();
  return { blocks, unsupported };
}

export function sourceDocumentAndFlow(source) {
  let rawSource;
  try {
    rawSource = new TextDecoder("utf-8", { fatal: true }).decode(source);
  } catch {
    fail("Hugo source files must contain valid UTF-8 before source ranges can be recorded.");
  }
  const bodyStart = sourceBodyStart(source);
  const tokens = tokenizeShortcodes(source, bodyStart);
  const lines = lineStarts(source);
  const spans = shortcodeSpans(tokens);
  const contentFlow = [];
  let cursor = bodyStart;
  let hasUnsupportedMarkdown = false;
  const appendText = (start, end) => {
    if (end <= start) return { unsupported: false };
    const textBytes = source.subarray(start, end);
    const parsed = markdownBlocks(textBytes);
    hasUnsupportedMarkdown ||= parsed.unsupported;
    if (parsed.blocks.length) {
      contentFlow.push({
        type: "text",
        blocks: parsed.blocks,
        sourceRange: sourceRange(lines, start, end),
      });
    }
    return parsed;
  };
  for (const span of spans) {
    const start = Math.max(cursor, Math.min(source.length, span.start));
    const end = Math.max(start, Math.min(source.length, span.end));
    appendText(cursor, start);
    contentFlow.push({
      type: "shortcode",
      name: span.name || "shortcode",
      disposition: "website-required",
      readableChildren: readableChildren(source, span, tokens),
      sourceRange: sourceRange(lines, start, end),
    });
    cursor = end;
  }
  appendText(cursor, source.length);
  return {
    sourceDocument: {
      format: "hugo-markdown",
      rawSource,
      bodyStartByte: bodyStart,
      shortcodeTokens: tokens,
      roundTripPolicy: "Preserve rawSource; edits replace explicit byte ranges only.",
    },
    contentFlow,
    hasUnsupportedMarkdown: spans.length > 0 || hasUnsupportedMarkdown,
  };
}

export function enrichManifest(manifest, _rootDirectory, readSource) {
  if (!manifest || manifest.schemaVersion !== 1 || !Array.isArray(manifest.pages)) {
    fail("Input is not a Hugo Client Manifest v1 document.");
  }
  for (const page of manifest.pages) {
    if (typeof page.sourcePath !== "string" || !page.sourcePath) continue;
    const source = readSource(page.sourcePath);
    const enriched = sourceDocumentAndFlow(source);
    page.sourceDocument = enriched.sourceDocument;
    page.contentFlow = enriched.contentFlow;
    if (enriched.hasUnsupportedMarkdown && page.rendering?.mode === "native") {
      page.rendering.mode = "mixed";
      page.rendering.reason = "Unsupported source content requires Website mode.";
    } else if (enriched.sourceDocument.shortcodeTokens.length && page.rendering?.mode !== "web") {
      page.rendering.mode = "mixed";
      page.rendering.reason = "Shortcodes remain Website-required unless a source-aware adapter represents them.";
    }
  }
  return manifest;
}

async function main() {
  const { root: rootArgument, manifest: manifestArgument } = parseArgs(process.argv.slice(2));
  const root = await realpath(rootArgument);
  const manifestPath = await realpath(manifestArgument);
  const document = JSON.parse(await readFile(manifestPath, "utf8"));
  const enriched = enrichManifest(document, root, (sourcePath) => {
    const normalized = sourcePath.replaceAll("\\", "/");
    if (normalized.startsWith("/") || normalized.split("/").some((part) => part === ".." || part === "")) {
      fail(`Unsafe source path in manifest: ${sourcePath}`);
    }
    const candidate = path.resolve(root, normalized);
    if (!isWithin(root, candidate)) fail(`Source path escapes the selected Hugo site: ${sourcePath}`);
    const canonical = realpathSync(candidate);
    if (!isWithin(root, canonical)) fail(`Source path resolves outside the selected Hugo site: ${sourcePath}`);
    const bytes = readFileSync(canonical);
    return bytes;
  });
  const temporary = `${manifestPath}.tmp-${process.pid}`;
  await writeFile(temporary, `${JSON.stringify(enriched, null, 2)}\n`, { flag: "wx" });
  await rename(temporary, manifestPath);
  process.stdout.write(`Enriched ${enriched.pages.filter((page) => page.sourceDocument).length} pages with source provenance.\n`);
}

if (import.meta.url === pathToFileURL(process.argv[1] || "").href) {
  main().catch((error) => {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  });
}
