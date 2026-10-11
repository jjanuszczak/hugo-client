use serde::Serialize;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{Manager, State};

#[derive(Clone, Serialize)]
struct ProjectSummary {
    name: String,
    path: String,
    is_hugo: bool,
    config_path: Option<String>,
    content_path: Option<String>,
    hugo_version: Option<String>,
    content_count: usize,
    message: String,
}

#[derive(Serialize)]
struct MenuItem {
    id: String,
    label: String,
    url: String,
    parent: Option<String>,
    weight: i64,
    external: bool,
}

#[derive(Serialize)]
struct ContentPage {
    id: String,
    title: String,
    summary: String,
    path: String,
    source_path: String,
    kind: String,
    section: String,
    date: String,
    draft: bool,
    tags: Vec<String>,
    categories: Vec<String>,
    language: String,
    body: Vec<(String, String)>,
    source_document: Value,
    content_flow: Vec<Value>,
    has_unsupported_blocks: bool,
}

#[derive(Serialize)]
struct ProjectSnapshot {
    summary: ProjectSummary,
    manifest: Value,
    preview_url: String,
}

#[derive(Default)]
struct PreviewServer {
    child: Mutex<Option<Child>>,
    directory: Mutex<Option<PathBuf>>,
}

impl PreviewServer {
    fn stop(&self) {
        let _ = self.replace(None, None);
    }

    fn replace(
        &self,
        mut next_child: Option<Child>,
        next_directory: Option<PathBuf>,
    ) -> Result<(), String> {
        let mut child_slot = match self.child.lock() {
            Ok(slot) => slot,
            Err(_) => {
                if let Some(child) = &mut next_child {
                    let _ = child.kill();
                    let _ = child.wait();
                }
                if let Some(directory) = next_directory {
                    let _ = std::fs::remove_dir_all(directory);
                }
                return Err("The preview server state is unavailable.".to_string());
            }
        };
        let mut directory_slot = match self.directory.lock() {
            Ok(slot) => slot,
            Err(_) => {
                if let Some(child) = &mut next_child {
                    let _ = child.kill();
                    let _ = child.wait();
                }
                if let Some(directory) = next_directory {
                    let _ = std::fs::remove_dir_all(directory);
                }
                return Err("The preview directory state is unavailable.".to_string());
            }
        };
        let previous_child = std::mem::replace(&mut *child_slot, next_child);
        let previous_directory = std::mem::replace(&mut *directory_slot, next_directory);
        drop(directory_slot);
        drop(child_slot);

        if let Some(mut child) = previous_child {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(directory) = previous_directory {
            let _ = std::fs::remove_dir_all(directory);
        }
        Ok(())
    }
}

#[tauri::command]
fn inspect_project(path: String) -> Result<ProjectSummary, String> {
    let root = canonical_project_root(&path)?;
    Ok(summarize_project(&root))
}

#[tauri::command]
fn load_project(
    path: String,
    preview: State<'_, PreviewServer>,
) -> Result<ProjectSnapshot, String> {
    let root = canonical_project_root(&path)?;
    load_project_root(&root, &preview)
}

fn load_project_root(root: &Path, preview: &PreviewServer) -> Result<ProjectSnapshot, String> {
    let summary = summarize_project(&root);
    if !summary.is_hugo {
        return Err(summary.message);
    }

    let hugo = find_hugo_binary()
        .ok_or_else(|| "Hugo was not found. Install Hugo and reopen the repository.".to_string())?;
    let config_output = run_hugo(
        &hugo,
        &root,
        &["config", "--format", "json", "--noBuildLock"],
    )?;
    let config: Value = serde_json::from_slice(&config_output.stdout)
        .map_err(|error| format!("Hugo returned unreadable configuration: {error}"))?;
    let list_output = run_hugo(&hugo, &root, &["list", "all", "--noBuildLock"])?;
    let records = parse_csv(&String::from_utf8_lossy(&list_output.stdout));
    let (headers, rows) = records.split_first().ok_or_else(|| {
        "Hugo returned an empty content index. Check the site's content directory.".to_string()
    })?;
    let columns: std::collections::HashMap<&str, usize> = headers
        .iter()
        .enumerate()
        .map(|(index, header)| (header.as_str(), index))
        .collect();

    let mut pages = Vec::new();
    for row in rows {
        let field = |name: &str| -> &str {
            columns
                .get(name)
                .and_then(|index| row.get(*index))
                .map(String::as_str)
                .unwrap_or("")
        };
        let kind = field("kind");
        if !matches!(kind, "page" | "home" | "section") {
            continue;
        }

        let source_path = field("path").to_string();
        let source = safe_read_content(&root, &source_path)?;
        let (body, has_unsupported_blocks) = markdown_blocks(&source);
        let source_document = source_document(&source);
        let content_flow = Vec::new();
        let title = field("title").trim().to_string();
        let summary_text = frontmatter_scalar(&source, "summary")
            .or_else(|| frontmatter_scalar(&source, "description"))
            .unwrap_or_else(|| first_paragraph(&body));
        let tags = frontmatter_list(&source, "tags");
        let categories = frontmatter_list(&source, "categories");
        let language = frontmatter_scalar(&source, "lang")
            .or_else(|| {
                config
                    .get("defaultcontentlanguage")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .unwrap_or_else(|| "en".to_string());
        pages.push(ContentPage {
            id: source_path.clone(),
            title: if title.is_empty() {
                title_from_path(&source_path)
            } else {
                title
            },
            summary: summary_text,
            path: field("permalink").to_string(),
            source_path,
            kind: kind.to_string(),
            section: field("section").to_string(),
            date: field("date").to_string(),
            draft: field("draft").eq_ignore_ascii_case("true"),
            tags,
            categories,
            language,
            body,
            source_document,
            content_flow,
            has_unsupported_blocks,
        });
    }

    let base_url = config
        .get("baseurl")
        .or_else(|| config.get("baseURL"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let reference_paths = build_reference_paths(&pages, base_url);
    for page in &mut pages {
        let raw_source = page.source_document["rawSource"]
            .as_str()
            .unwrap_or_default();
        page.content_flow = source_content_flow(
            raw_source,
            &page.source_document,
            &page.source_path,
            &reference_paths,
        );
        apply_adapter_dispositions(&mut page.source_document, &page.content_flow);
        page.has_unsupported_blocks |= page
            .content_flow
            .iter()
            .any(|item| item["type"] == "shortcode");
    }

    pages.sort_by(|left, right| {
        let left_home = left.kind == "home";
        let right_home = right.kind == "home";
        right_home
            .cmp(&left_home)
            .then_with(|| left.section.cmp(&right.section))
            .then_with(|| left.title.to_lowercase().cmp(&right.title.to_lowercase()))
    });

    let site_title = config
        .get("title")
        .and_then(Value::as_str)
        .map(strip_html_tags)
        .filter(|title| !title.trim().is_empty())
        .unwrap_or_else(|| summary.name.clone());

    let main_menu = parse_menu(&config, "main");
    let footer_menu = parse_menu(&config, "footer");
    let manifest = build_manifest_v1(&config, &site_title, &main_menu, &footer_menu, &pages);
    validate_manifest_v1(&manifest)?;
    if pages.is_empty() {
        return Err(format!(
            "Hugo resolved no content pages in {}. Check the content directory and Hugo build configuration.",
            root.display()
        ));
    }
    let preview_url = start_hugo_preview(root, &hugo, preview)?;
    Ok(ProjectSnapshot {
        summary,
        manifest,
        preview_url,
    })
}

pub fn validate_manifest_v1(manifest: &Value) -> Result<(), String> {
    let schema: Value = serde_json::from_str(include_str!(
        "../../schemas/hugo-client-manifest-v1.schema.json"
    ))
    .map_err(|error| format!("Manifest v1 schema is invalid: {error}"))?;
    let validator = jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .build(&schema)
        .map_err(|error| format!("Cannot compile Manifest v1 schema: {error}"))?;
    if let Err(error) = validator.validate(manifest) {
        return Err(format!(
            "Generated Manifest v1 failed schema validation: {error}"
        ));
    }
    Ok(())
}

fn build_manifest_v1(
    config: &Value,
    site_title: &str,
    main_menu: &[MenuItem],
    footer_menu: &[MenuItem],
    pages: &[ContentPage],
) -> Value {
    let base_url = config
        .get("baseurl")
        .or_else(|| config.get("baseURL"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let default_language = config
        .get("defaultcontentlanguage")
        .or_else(|| config.get("defaultContentLanguage"))
        .and_then(Value::as_str)
        .unwrap_or("en");
    let languages = config
        .get("languages")
        .and_then(Value::as_object)
        .map(|values| values.keys().cloned().collect::<Vec<_>>())
        .filter(|values| !values.is_empty())
        .unwrap_or_else(|| vec![default_language.to_string()]);

    let mut navigation = Vec::new();
    for (location, items) in [("main", main_menu), ("footer", footer_menu)] {
        navigation.extend(items.iter().map(|item| {
            json!({
                "id": item.id,
                "label": item.label,
                "url": item.url,
                "parent": item.parent,
                "weight": item.weight,
                "external": item.external,
                "menu": location
            })
        }));
    }

    let manifest_pages = pages
        .iter()
        .map(|page| {
            let path = local_site_path(&page.path, base_url);
            let blocks = manifest_blocks(&page.body);
            let mut taxonomies = serde_json::Map::new();
            if !page.tags.is_empty() {
                taxonomies.insert("tags".to_string(), json!(page.tags));
            }
            let categories = &page.categories;
            if !categories.is_empty() {
                taxonomies.insert("categories".to_string(), json!(categories));
            }
            let mut manifest_page = json!({
                "id": page.source_path,
                "sourcePath": page.source_path,
                "path": path,
                "canonicalURL": page.path,
                "title": page.title,
                "summary": page.summary,
                "draft": page.draft,
                "kind": page.kind,
                "section": page.section,
                "language": page.language,
                "taxonomies": taxonomies,
                "rendering": {
                    "mode": if page.has_unsupported_blocks { "mixed" } else { "native" },
                    "fallbackURL": path,
                    "reason": if page.has_unsupported_blocks {
                        "Unsupported Hugo or raw HTML blocks require Website mode."
                    } else {
                        "All detected blocks use the supported native subset."
                    }
                },
                "blocks": blocks,
                "sourceDocument": page.source_document,
                "contentFlow": page.content_flow
            });
            if !page.date.is_empty() {
                manifest_page["date"] = json!(page.date);
            }
            manifest_page
        })
        .collect::<Vec<_>>();

    let mut site = json!({
        "title": site_title,
        "defaultLanguage": default_language,
        "languages": languages,
        "chromeOmittedReason": "The pilot theme does not declare native site chrome; Website mode preserves the full theme."
    });
    if !base_url.is_empty() {
        site["baseURL"] = json!(base_url);
    }
    json!({
        "schemaVersion": 1,
        "site": site,
        "navigation": navigation,
        "pages": manifest_pages,
        "assets": []
    })
}

fn manifest_blocks(body: &[(String, String)]) -> Vec<Value> {
    let mut blocks: Vec<Value> = Vec::new();
    for (kind, text) in body {
        if let Some(level) = kind
            .strip_prefix('h')
            .and_then(|value| value.parse::<u8>().ok())
        {
            blocks.push(json!({"type": "heading", "level": level, "text": text}));
        } else if kind == "quote" {
            blocks.push(json!({"type": "quote", "text": text}));
        } else if let Some(item) = text.strip_prefix("• ") {
            let item = json!([{
                "type": "paragraph",
                "inlines": [{"type": "text", "text": item}]
            }]);
            if let Some(last) = blocks
                .last_mut()
                .filter(|block| block.get("type").and_then(Value::as_str) == Some("list"))
            {
                if let Some(items) = last.get_mut("items").and_then(Value::as_array_mut) {
                    items.push(item);
                    continue;
                }
            }
            blocks.push(json!({"type": "list", "ordered": false, "items": [item]}));
        } else {
            blocks.push(json!({
                "type": "paragraph",
                "inlines": [{"type": "text", "text": text}]
            }));
        }
    }
    blocks
}

/// Lossless editing envelope. `rawSource` is inert Markdown, never HTML; byte ranges
/// address the original UTF-8 file. Token records are hints, not a replacement AST.
fn source_document(source: &str) -> Value {
    let body_start = source_body_start(source);
    let body = &source[body_start..];
    let mut tokens = Vec::new();
    let mut cursor = 0;
    while let Some(relative) = body[cursor..].find("{{") {
        let start = cursor + relative;
        let Some((open, close, syntax)) = (match body.get(start..start + 3) {
            Some("{{<") => Some(("{{<", ">}}", "angle")),
            Some("{{%") => Some(("{{%", "%}}", "percent")),
            _ => None,
        }) else {
            cursor = start + 2;
            continue;
        };
        let content_start = start + open.len();
        let Some(end_rel) = body[content_start..].find(close) else {
            cursor = content_start;
            continue;
        };
        let end = content_start + end_rel + close.len();
        let raw = &body[start..end];
        let inner = body[content_start..content_start + end_rel].trim();
        let words = shortcode_words(inner);
        if let Some(name) = words.first() {
            let closing = name.starts_with('/');
            let shortcode_name = name.trim_start_matches('/');
            let self_closing = inner.ends_with('/');
            let params = words.iter().skip(1).map(|word| {
                let (key, value) = word.split_once('=').map(|(k,v)| (Some(k),v)).unwrap_or((None,word.as_str()));
                json!({"kind": if key.is_some() {"named"} else {"positional"}, "name": key, "value": value.trim_matches(['"', '\'']), "raw": word})
            }).collect::<Vec<_>>();
            let abs_start = body_start + start;
            let abs_end = body_start + end;
            tokens.push(json!({"type":"shortcode", "name":shortcode_name, "syntax":syntax,
                "closing":closing, "selfClosing":self_closing, "raw":raw, "parameters":params,
                "disposition":"website-required", "sourceRange":source_range(source, abs_start, abs_end)}));
        }
        cursor = end;
    }
    json!({"format":"hugo-markdown", "rawSource":source, "bodyStartByte":body_start,
        "shortcodeTokens":tokens, "roundTripPolicy":"Preserve rawSource; edits replace explicit byte ranges only."})
}

/// Ordered safe render flow for SC-3. Plain Markdown chunks reuse the conservative
/// native subset parser; shortcode spans become inert placeholders with child text.
fn build_reference_paths(
    pages: &[ContentPage],
    base_url: &str,
) -> std::collections::HashMap<String, String> {
    let mut paths = std::collections::HashMap::new();
    for page in pages {
        let site_path = local_site_path(&page.path, base_url);
        let mut keys = vec![page.source_path.as_str()];
        let without_content = page
            .source_path
            .strip_prefix("content/")
            .unwrap_or(&page.source_path);
        keys.push(without_content);
        for key in keys {
            let normalized = normalize_ref_key(key);
            if !normalized.is_empty() {
                paths.insert(normalized.clone(), site_path.clone());
            }
        }
        let normalized_url = normalize_ref_key(&site_path);
        paths.insert(normalized_url, site_path.clone());
    }
    paths
}

fn normalize_ref_key(value: &str) -> String {
    let mut key = value.trim().trim_matches(['"', '\'', '/']).to_string();
    if key.starts_with("content/") {
        key = key[8..].to_string();
    }
    if key.ends_with(".md") {
        key.truncate(key.len() - 3);
    }
    if key.ends_with("/index") {
        key.truncate(key.len() - 6);
    }
    key.trim_matches('/').to_string()
}

fn source_content_flow(
    source: &str,
    document: &Value,
    source_path: &str,
    reference_paths: &std::collections::HashMap<String, String>,
) -> Vec<Value> {
    let body_start = document["bodyStartByte"].as_u64().unwrap_or(0) as usize;
    let tokens = document["shortcodeTokens"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut open: Vec<(String, usize, usize)> = Vec::new();
    let mut spans: Vec<(usize, usize, String)> = Vec::new();
    for token in &tokens {
        let range = &token["sourceRange"];
        let start = range["startByte"].as_u64().unwrap_or(0) as usize;
        let end = range["endByte"].as_u64().unwrap_or(0) as usize;
        let name = token["name"].as_str().unwrap_or("shortcode").to_string();
        if token["closing"] == true {
            if let Some(index) = open
                .iter()
                .rposition(|(candidate, _, _)| candidate == &name)
            {
                let (outer_name, outer_start, _) = open.remove(index);
                if index == 0 {
                    spans.push((outer_start, end, outer_name));
                }
            } else {
                spans.push((start, end, name));
            }
        } else if token["selfClosing"] == true || !has_closing_shortcode(&tokens, token, &name) {
            spans.push((start, end, name));
        } else {
            open.push((name, start, end));
        }
    }
    for (name, start, end) in open {
        spans.push((start, end, name));
    }
    spans.sort_by_key(|span| span.0);

    let mut flow = Vec::new();
    let mut cursor = body_start;
    for (start, end, name) in spans {
        let start = start.max(cursor).min(source.len());
        let end = end.max(start).min(source.len());
        let token = tokens.iter().find(|token| {
            token["sourceRange"]["startByte"].as_u64() == Some(start as u64)
                && token["closing"] != true
        });
        let paired = token.is_some_and(|token| has_closing_shortcode(&tokens, token, &name));
        let raw_before = &source[cursor..start];
        let mut next_cursor = end;
        let mut text_before = raw_before;
        let mut inline_link = None;
        if matches!(name.as_str(), "ref" | "relref") {
            let target = token
                .and_then(shortcode_parameter_value)
                .unwrap_or_default();
            let resolved = resolve_reference(&target, source_path, reference_paths);
            if let Some(href) = resolved {
                if let Some((prefix_len, label, after_len)) =
                    markdown_link_context(raw_before, &source[end..])
                {
                    text_before = &raw_before[..prefix_len];
                    next_cursor = end + after_len;
                    inline_link = Some((label, href));
                } else {
                    inline_link = Some((
                        target.rsplit('/').next().unwrap_or(&target).to_string(),
                        href,
                    ));
                }
            }
        }
        append_text_flow(&mut flow, text_before);
        if let Some((label, href)) = inline_link {
            flow.push(
                json!({"type":"link", "label":label, "href":href, "inline":true,
                "sourceRange":source_range(source, start, end)}),
            );
            cursor = next_cursor;
            continue;
        }
        let child_source = if end > start {
            let mut child = String::new();
            let mut child_cursor = start;
            for token in &tokens {
                let token_start = token["sourceRange"]["startByte"].as_u64().unwrap_or(0) as usize;
                let token_end = token["sourceRange"]["endByte"].as_u64().unwrap_or(0) as usize;
                if token_start >= start && token_end <= end {
                    if token_start >= child_cursor {
                        child.push_str(&source[child_cursor..token_start]);
                    }
                    child_cursor = token_end;
                }
            }
            if child_cursor <= end {
                child.push_str(&source[child_cursor..end]);
            }
            child
        } else {
            String::new()
        };
        let (child_blocks, _) = markdown_blocks(&child_source);
        let child_blocks = manifest_blocks(&child_blocks);
        let range = source_range(source, start, end);
        match name.as_str() {
            "quick-answer" if paired => flow.push(json!({"type":"callout", "title":"Quick Answer", "tone":"note", "children":child_blocks, "sourceRange":range})),
            "lead" if paired => {
                if !child_blocks.is_empty() { flow.push(json!({"type":"text", "blocks":child_blocks, "sourceRange":range})); }
            }
            "faq" if paired => flow.push(json!({"type":"faq", "items":faq_items(&tokens, source, start, end), "sourceRange":range})),
            "button" if paired => {
                let target = token.and_then(|value| named_shortcode_parameter(value, &["href", "pageRef", "pageref"])).unwrap_or_default();
                if let Some(href) = resolve_reference(&target, source_path, reference_paths) {
                    let label = child_blocks.iter().map(block_plain_text).filter(|text| !text.is_empty()).collect::<Vec<_>>().join(" ");
                    flow.push(json!({"type":"link", "label":if label.is_empty() { target } else { label }, "href":href, "sourceRange":range}));
                } else {
                    flow.push(json!({"type":"shortcode", "name":name, "disposition":"website-required", "readableChildren":child_blocks, "sourceRange":range}));
                }
            }
            _ => flow.push(json!({"type":"shortcode", "name":name, "disposition":"website-required",
                "readableChildren":child_blocks, "sourceRange":range})),
        }
        cursor = end;
    }
    append_text_flow(&mut flow, &source[cursor..]);
    flow
}

fn shortcode_parameter_value(token: &Value) -> Option<String> {
    token["parameters"]
        .as_array()?
        .iter()
        .find_map(|parameter| parameter["value"].as_str().map(str::to_string))
}

fn named_shortcode_parameter(token: &Value, names: &[&str]) -> Option<String> {
    token["parameters"]
        .as_array()?
        .iter()
        .find_map(|parameter| {
            let name = parameter["name"].as_str()?;
            names
                .contains(&name)
                .then(|| parameter["value"].as_str().unwrap_or_default().to_string())
        })
}

fn block_plain_text(block: &Value) -> String {
    if let Some(text) = block["text"].as_str() {
        return text.to_string();
    }
    if let Some(inlines) = block["inlines"].as_array() {
        return inlines
            .iter()
            .filter_map(|inline| inline["text"].as_str())
            .collect::<Vec<_>>()
            .join("");
    }
    String::new()
}

fn resolve_reference(
    target: &str,
    source_path: &str,
    paths: &std::collections::HashMap<String, String>,
) -> Option<String> {
    if let Some(url) = safe_reference_url(target) {
        return Some(url);
    }
    let suffix_start = target.find(['#', '?']).unwrap_or(target.len());
    let (path_part, suffix) = target.split_at(suffix_start);
    let key = normalize_ref_key(path_part);
    if let Some(path) = paths.get(&key) {
        return Some(format!("{path}{suffix}"));
    }
    if !path_part.starts_with('/') {
        let parent = Path::new(source_path)
            .parent()
            .unwrap_or_else(|| Path::new("content"));
        let mut parts = parent
            .strip_prefix("content")
            .unwrap_or(parent)
            .components()
            .filter_map(|part| part.as_os_str().to_str().map(str::to_string))
            .collect::<Vec<_>>();
        for part in Path::new(path_part).components() {
            match part {
                std::path::Component::ParentDir => {
                    parts.pop();
                }
                std::path::Component::Normal(value) => {
                    parts.push(value.to_string_lossy().to_string())
                }
                _ => {}
            }
        }
        if let Some(path) = paths.get(&normalize_ref_key(&parts.join("/"))) {
            return Some(format!("{path}{suffix}"));
        }
    } else if path_part.starts_with('/') && !path_part.starts_with("//") {
        return Some(format!("{path_part}{suffix}"));
    }
    None
}

fn apply_adapter_dispositions(document: &mut Value, flow: &[Value]) {
    let mut adapted_ranges = Vec::new();
    for item in flow {
        let range = &item["sourceRange"];
        if let (Some(start), Some(end)) = (range["startByte"].as_u64(), range["endByte"].as_u64()) {
            if matches!(
                item["type"].as_str(),
                Some("link" | "callout" | "faq" | "text")
            ) {
                adapted_ranges.push((start, end));
            }
        }
        if item["type"] == "faq" {
            if let Some(items) = item["items"].as_array() {
                for faq_item in items {
                    let range = &faq_item["sourceRange"];
                    if let (Some(start), Some(end)) =
                        (range["startByte"].as_u64(), range["endByte"].as_u64())
                    {
                        adapted_ranges.push((start, end));
                    }
                }
            }
        }
    }
    if let Some(tokens) = document["shortcodeTokens"].as_array_mut() {
        for token in tokens {
            let name = token["name"].as_str().unwrap_or_default();
            let range = &token["sourceRange"];
            let start = range["startByte"].as_u64().unwrap_or(0);
            let end = range["endByte"].as_u64().unwrap_or(0);
            let adapted = adapted_ranges.iter().any(|(flow_start, flow_end)| {
                if matches!(name, "ref" | "relref" | "button") {
                    *flow_start == start && *flow_end == end
                } else {
                    *flow_start <= start && *flow_end >= end
                }
            });
            if adapted {
                token["disposition"] = json!("native");
            }
        }
    }
}

fn safe_reference_url(value: &str) -> Option<String> {
    (value.starts_with("https://") || value.starts_with("http://")).then(|| value.to_string())
}

fn markdown_link_context(before: &str, after: &str) -> Option<(usize, String, usize)> {
    if !after.starts_with(')') || !before.ends_with("](") {
        return None;
    }
    let opener = before[..before.len() - 2].rfind('[')?;
    let label = &before[opener + 1..before.len() - 2];
    Some((opener, label.to_string(), 1))
}

fn faq_items(tokens: &[Value], source: &str, start: usize, end: usize) -> Vec<Value> {
    let mut items = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if token["name"] != "faq-item" || token["closing"] == true {
            continue;
        }
        let item_start = token["sourceRange"]["startByte"].as_u64().unwrap_or(0) as usize;
        if item_start < start || item_start >= end {
            continue;
        }
        let Some(close) = tokens.iter().skip(index + 1).find(|candidate| {
            candidate["name"] == "faq-item"
                && candidate["closing"] == true
                && candidate["sourceRange"]["startByte"].as_u64().unwrap_or(0) as usize > item_start
        }) else {
            continue;
        };
        let item_end = close["sourceRange"]["endByte"].as_u64().unwrap_or(0) as usize;
        if item_end > end {
            continue;
        }
        let question = token["parameters"]
            .as_array()
            .and_then(|params| params.iter().find(|param| param["name"] == "question"))
            .and_then(|param| param["value"].as_str())
            .unwrap_or("Question");
        let mut answer = String::new();
        let mut cursor = token["sourceRange"]["endByte"].as_u64().unwrap_or(0) as usize;
        for nested in tokens.iter().skip(index + 1) {
            let nested_start = nested["sourceRange"]["startByte"].as_u64().unwrap_or(0) as usize;
            let nested_end = nested["sourceRange"]["endByte"].as_u64().unwrap_or(0) as usize;
            if nested_start >= item_end {
                break;
            }
            if nested_start >= cursor {
                answer.push_str(&source[cursor..nested_start]);
                cursor = nested_end;
            }
        }
        if cursor <= item_end {
            answer.push_str(&source[cursor..item_end]);
        }
        let (answer_blocks, _) = markdown_blocks(&answer);
        items.push(
            json!({"question":question, "answer":manifest_blocks(&answer_blocks),
            "sourceRange":source_range(source, item_start, item_end)}),
        );
    }
    items
}

fn has_closing_shortcode(tokens: &[Value], current: &Value, name: &str) -> bool {
    let current_start = current["sourceRange"]["startByte"].as_u64().unwrap_or(0);
    tokens.iter().any(|candidate| {
        candidate["closing"] == true
            && candidate["name"].as_str() == Some(name)
            && candidate["sourceRange"]["startByte"].as_u64().unwrap_or(0) > current_start
    })
}

fn append_text_flow(flow: &mut Vec<Value>, text: &str) {
    let (blocks, _) = markdown_blocks(text);
    if !blocks.is_empty() {
        flow.push(json!({"type":"text", "blocks":manifest_blocks(&blocks)}));
    }
}

fn shortcode_words(s: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    for c in s.chars() {
        if let Some(q) = quote {
            word.push(c);
            if c == q {
                quote = None;
            }
        } else if c == '"' || c == '\'' {
            quote = Some(c);
            word.push(c);
        } else if c.is_whitespace() {
            if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
        } else {
            word.push(c);
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
}

fn source_range(source: &str, start: usize, end: usize) -> Value {
    let line = |byte: usize| {
        source.as_bytes()[..byte.min(source.len())]
            .iter()
            .filter(|&&b| b == b'\n')
            .count()
            + 1
    };
    json!({"startByte":start, "endByte":end, "startLine":line(start), "endLine":line(end)})
}

fn source_body_start(source: &str) -> usize {
    let Some(first_end) = source.find('\n') else {
        return 0;
    };
    let first = source[..first_end].trim_end_matches('\r');
    let close = match first {
        "---" => "---",
        "+++" => "+++",
        ";;;" => ";;;",
        _ => return 0,
    };
    source[first_end + 1..]
        .find(close)
        .map(|i| first_end + 1 + i + close.len())
        .map(|i| {
            if source.as_bytes().get(i) == Some(&b'\n') {
                i + 1
            } else {
                i
            }
        })
        .unwrap_or(0)
}

fn local_site_path(permalink: &str, base_url: &str) -> String {
    if permalink.is_empty() {
        return "/".to_string();
    }
    let base = base_url.trim_end_matches('/');
    if !base.is_empty() && permalink.starts_with(base) {
        let suffix = &permalink[base.len()..];
        if suffix.is_empty() {
            return "/".to_string();
        }
        return if suffix.starts_with('/') {
            suffix.to_string()
        } else {
            format!("/{suffix}")
        };
    }
    if permalink.starts_with('/')
        || permalink.starts_with("http://")
        || permalink.starts_with("https://")
    {
        permalink.to_string()
    } else {
        format!("/{permalink}")
    }
}

fn start_hugo_preview(root: &Path, hugo: &Path, state: &PreviewServer) -> Result<String, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("Cannot reserve a local preview port: {error}"))?;
    let port = listener
        .local_addr()
        .map_err(|error| error.to_string())?
        .port();
    drop(listener);

    let cache_root = std::env::temp_dir()
        .join("hugo-native-client")
        .join(format!("{}-{port}", std::process::id()));
    let cache_dir = cache_root.join("cache");
    let resource_dir = cache_root.join("resources");
    let destination_dir = cache_root.join("public");
    std::fs::create_dir_all(&cache_dir).map_err(|error| {
        let _ = std::fs::remove_dir_all(&cache_root);
        format!(
            "Cannot create Hugo's temporary cache directory {}: {error}",
            cache_dir.display()
        )
    })?;
    std::fs::create_dir_all(&resource_dir).map_err(|error| {
        let _ = std::fs::remove_dir_all(&cache_root);
        format!(
            "Cannot create Hugo's temporary resource directory {}: {error}",
            resource_dir.display()
        )
    })?;
    std::fs::create_dir_all(&destination_dir).map_err(|error| {
        let _ = std::fs::remove_dir_all(&cache_root);
        format!(
            "Cannot create Hugo's temporary output directory {}: {error}",
            destination_dir.display()
        )
    })?;
    let base_url = format!("http://127.0.0.1:{port}/");
    let port_text = port.to_string();
    let destination_text = destination_dir.to_string_lossy().to_string();
    let mut child = Command::new(hugo)
        .args([
            "server",
            "--bind",
            "127.0.0.1",
            "--port",
            &port_text,
            "--baseURL",
            &base_url,
            "--noBuildLock",
            "--disableFastRender",
            "--buildDrafts",
        ])
        .arg("--destination")
        .arg(destination_text)
        .arg("--cacheDir")
        .arg(&cache_dir)
        .current_dir(root)
        .env("HUGO_RESOURCEDIR", &resource_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            let _ = std::fs::remove_dir_all(&cache_root);
            format!(
                "Could not start `hugo server` in {}: {error}",
                root.display()
            )
        })?;

    let preview_diagnostics = Arc::new(Mutex::new(String::new()));
    let mut diagnostics_reader = child.stderr.take().map(|mut stderr| {
        let captured = Arc::clone(&preview_diagnostics);
        thread::spawn(move || {
            let mut buffer = [0u8; 2048];
            loop {
                match stderr.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => {
                        if let Ok(mut output) = captured.lock() {
                            output.push_str(&String::from_utf8_lossy(&buffer[..read]));
                            const MAX_DIAGNOSTICS_BYTES: usize = 16 * 1024;
                            if output.len() > MAX_DIAGNOSTICS_BYTES {
                                let trim_to = output.len() - MAX_DIAGNOSTICS_BYTES;
                                let boundary = output
                                    .char_indices()
                                    .find(|(index, _)| *index >= trim_to)
                                    .map(|(index, _)| index)
                                    .unwrap_or(output.len());
                                output.drain(..boundary);
                            }
                        }
                    }
                }
            }
        })
    });

    let deadline = Instant::now() + Duration::from_secs(60);
    while Instant::now() < deadline {
        if preview_http_ready(port) {
            if let Err(error) = state.replace(Some(child), Some(cache_root.clone())) {
                let _ = std::fs::remove_dir_all(&cache_root);
                return Err(error);
            }
            return Ok(base_url);
        }
        let status = match child.try_wait() {
            Ok(status) => status,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = std::fs::remove_dir_all(&cache_root);
                return Err(format!(
                    "Could not monitor `hugo server` in {}: {error}",
                    root.display()
                ));
            }
        };
        if let Some(status) = status {
            if let Some(reader) = diagnostics_reader.take() {
                let _ = reader.join();
            }
            let diagnostics = preview_diagnostics
                .lock()
                .map(|text| text.trim().to_string())
                .unwrap_or_default();
            let _ = std::fs::remove_dir_all(&cache_root);
            let details = if diagnostics.is_empty() {
                String::new()
            } else {
                format!("\nHugo diagnostics:\n{diagnostics}")
            };
            return Err(format!(
                "`hugo server` stopped during startup in {} with status {status}.{details}",
                root.display()
            ));
        }
        thread::sleep(Duration::from_millis(100));
    }
    let _ = child.kill();
    let _ = child.wait();
    if let Some(reader) = diagnostics_reader.take() {
        let _ = reader.join();
    }
    let diagnostics = preview_diagnostics
        .lock()
        .map(|text| text.trim().to_string())
        .unwrap_or_default();
    let _ = std::fs::remove_dir_all(&cache_root);
    let details = if diagnostics.is_empty() {
        String::new()
    } else {
        format!("\nHugo diagnostics:\n{diagnostics}")
    };
    Err(format!(
        "`hugo server` did not become ready within 60 seconds for {}.{details}",
        root.display()
    ))
}

fn preview_http_ready(port: u16) -> bool {
    let Ok(mut stream) = TcpStream::connect(("127.0.0.1", port)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(300)));
    let request = format!(
        "GET / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\nAccept-Encoding: identity\r\n\r\n"
    );
    if stream.write_all(request.as_bytes()).is_err() {
        return false;
    }
    let mut response = [0u8; 4096];
    let Ok(read) = stream.read(&mut response) else {
        return false;
    };
    String::from_utf8_lossy(&response[..read]).starts_with("HTTP/1.1 200")
}

fn canonical_project_root(path: &str) -> Result<PathBuf, String> {
    let root = PathBuf::from(path)
        .canonicalize()
        .map_err(|error| format!("Cannot open the selected folder: {error}"))?;
    if !root.is_dir() {
        return Err("The selected path is not a directory.".to_string());
    }
    Ok(root)
}

fn summarize_project(root: &Path) -> ProjectSummary {
    let config = find_hugo_config(root);
    let content = root.join("content");
    let content_exists = content.is_dir();
    let is_hugo = config.is_some() || content_exists;
    let content_count = if content_exists {
        count_content_files(&content)
    } else {
        0
    };
    let name = root
        .file_name()
        .and_then(|part| part.to_str())
        .unwrap_or("Hugo project")
        .to_string();
    let message = match (is_hugo, config.is_some(), content_exists) {
        (false, _, _) => "No Hugo configuration or content directory was found.".to_string(),
        (true, false, true) => {
            "Found a content directory. No Hugo config file was found; Hugo defaults may apply."
                .to_string()
        }
        (true, true, false) => {
            "Found a Hugo config file. The expected content directory is missing.".to_string()
        }
        (true, true, true) => "Hugo configuration and content directory found.".to_string(),
        _ => "Project inspection complete.".to_string(),
    };

    ProjectSummary {
        name,
        path: root.display().to_string(),
        is_hugo,
        config_path: config.map(|path| path.display().to_string()),
        content_path: content_exists.then(|| content.display().to_string()),
        hugo_version: hugo_version(),
        content_count,
        message,
    }
}

fn find_hugo_config(root: &Path) -> Option<PathBuf> {
    let names = [
        "hugo.toml",
        "hugo.yaml",
        "hugo.yml",
        "hugo.json",
        "config.toml",
        "config.yaml",
        "config.yml",
        "config.json",
    ];
    names
        .iter()
        .map(|name| root.join(name))
        .chain(
            names
                .iter()
                .map(|name| root.join("config").join("_default").join(name)),
        )
        .find(|path| path.is_file())
}

fn count_content_files(root: &Path) -> usize {
    let mut count = 0;
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| {
                extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("markdown")
            }) {
                count += 1;
            }
        }
    }
    count
}

fn find_hugo_binary() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&path) {
            let candidate = directory.join("hugo");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    [
        "/opt/homebrew/bin/hugo",
        "/usr/local/bin/hugo",
        "/usr/bin/hugo",
    ]
    .iter()
    .map(PathBuf::from)
    .find(|candidate| candidate.is_file())
}

fn run_hugo(binary: &Path, root: &Path, args: &[&str]) -> Result<Output, String> {
    let command = std::iter::once("hugo")
        .chain(args.iter().copied())
        .collect::<Vec<_>>()
        .join(" ");
    let output = Command::new(binary)
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| format!("Could not start `{command}` in {}: {error}", root.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let diagnostics = match (stderr.is_empty(), stdout.is_empty()) {
            (false, false) => format!("\nstderr:\n{stderr}\nstdout:\n{stdout}"),
            (false, true) => format!("\nstderr:\n{stderr}"),
            (true, false) => format!("\nstdout:\n{stdout}"),
            (true, true) => String::new(),
        };
        return Err(format!(
            "`{command}` failed in {} with status {}.{diagnostics}",
            root.display(),
            output.status
        ));
    }
    Ok(output)
}

fn hugo_version() -> Option<String> {
    let binary = find_hugo_binary()?;
    let output = Command::new(binary).arg("version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!text.is_empty()).then_some(text)
}

fn parse_menu(config: &Value, name: &str) -> Vec<MenuItem> {
    let Some(items) = config
        .get("menus")
        .and_then(|menus| menus.get(name))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };
    let mut result: Vec<MenuItem> = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let label = item
                .get("name")
                .or_else(|| item.get("identifier"))
                .and_then(Value::as_str)?
                .to_string();
            let target = item
                .get("url")
                .or_else(|| item.get("pageref"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let external = target.starts_with("https://") || target.starts_with("http://");
            let url = if external || target.starts_with('/') {
                target.to_string()
            } else if target.is_empty() {
                String::new()
            } else {
                format!("/{}/", target.trim_matches('/'))
            };
            Some(MenuItem {
                id: item
                    .get("identifier")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("{name}-{index}")),
                label,
                url,
                parent: item
                    .get("parent")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                weight: item
                    .get("weight")
                    .and_then(Value::as_i64)
                    .unwrap_or(index as i64),
                external,
            })
        })
        .collect();
    result.sort_by_key(|item| item.weight);
    result
}

fn parse_csv(input: &str) -> Vec<Vec<String>> {
    let mut records = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut chars = input.chars().peekable();
    let mut quoted = false;
    while let Some(character) = chars.next() {
        match character {
            '"' if quoted && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => row.push(std::mem::take(&mut field)),
            '\n' if !quoted => {
                row.push(std::mem::take(&mut field));
                if row.iter().any(|cell| !cell.is_empty()) {
                    records.push(std::mem::take(&mut row));
                } else {
                    row.clear();
                }
            }
            '\r' if !quoted => {}
            _ => field.push(character),
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        records.push(row);
    }
    records
}

fn safe_read_content(root: &Path, relative_path: &str) -> Result<String, String> {
    let candidate = root.join(relative_path);
    let canonical = candidate
        .canonicalize()
        .map_err(|error| format!("Cannot read Hugo content file {relative_path}: {error}"))?;
    if !canonical.starts_with(root) {
        return Err(format!(
            "Hugo returned a content path outside the repository: {relative_path}"
        ));
    }
    std::fs::read_to_string(canonical)
        .map_err(|error| format!("Cannot read Hugo content file {relative_path}: {error}"))
}

fn markdown_blocks(source: &str) -> (Vec<(String, String)>, bool) {
    let body = strip_front_matter(source);
    let mut blocks = Vec::new();
    let mut paragraph = Vec::new();
    let mut unsupported = false;
    let mut in_code = false;
    let mut in_raw_text_element: Option<String> = None;
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(tag) = in_raw_text_element.as_deref() {
            if trimmed.to_ascii_lowercase().contains(&format!("</{tag}>")) {
                in_raw_text_element = None;
            }
            unsupported = true;
            continue;
        }
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            flush_paragraph(&mut paragraph, &mut blocks, "p");
            in_code = !in_code;
            unsupported = true;
            continue;
        }
        if in_code {
            continue;
        }
        if trimmed.is_empty() {
            flush_paragraph(&mut paragraph, &mut blocks, "p");
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        if ["style", "script"].iter().any(|tag| {
            lower.starts_with(&format!("<{tag}>")) || lower.starts_with(&format!("<{tag} "))
        }) {
            flush_paragraph(&mut paragraph, &mut blocks, "p");
            unsupported = true;
            let tag = if lower.starts_with("<style") {
                "style"
            } else {
                "script"
            };
            if !lower.contains(&format!("</{tag}>")) {
                in_raw_text_element = Some(tag.to_string());
            }
            continue;
        }
        if trimmed.starts_with("{{<") || trimmed.starts_with("{{%") {
            flush_paragraph(&mut paragraph, &mut blocks, "p");
            continue;
        }
        if trimmed.starts_with('<') {
            flush_paragraph(&mut paragraph, &mut blocks, "p");
            unsupported = true;
            continue;
        }
        if trimmed.starts_with('#') {
            flush_paragraph(&mut paragraph, &mut blocks, "p");
            let level = trimmed
                .chars()
                .take_while(|character| *character == '#')
                .count();
            let text = clean_inline_markdown(trimmed[level..].trim());
            if !text.is_empty() {
                blocks.push((format!("h{}", level.min(6)), text));
            }
        } else if let Some(quote) = trimmed.strip_prefix("> ") {
            flush_paragraph(&mut paragraph, &mut blocks, "p");
            blocks.push(("quote".to_string(), clean_inline_markdown(quote)));
        } else if trimmed.starts_with("---") || trimmed.starts_with("***") {
            flush_paragraph(&mut paragraph, &mut blocks, "p");
        } else if trimmed.starts_with("| ") {
            flush_paragraph(&mut paragraph, &mut blocks, "p");
            unsupported = true;
        } else if trimmed.starts_with("![") {
            flush_paragraph(&mut paragraph, &mut blocks, "p");
            unsupported = true;
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            flush_paragraph(&mut paragraph, &mut blocks, "p");
            blocks.push((
                "p".to_string(),
                format!("• {}", clean_inline_markdown(&trimmed[2..])),
            ));
        } else {
            paragraph.push(trimmed.to_string());
        }
    }
    flush_paragraph(&mut paragraph, &mut blocks, "p");
    (blocks, unsupported)
}

fn strip_front_matter(source: &str) -> &str {
    let mut lines = source.lines();
    let Some(first) = lines.next() else {
        return source;
    };
    let close = match first.trim() {
        "---" => "---",
        "+++" => "+++",
        ";;;" => ";;;",
        _ => return source,
    };
    let mut offset = first.len() + 1;
    for line in lines {
        let line_end = offset + line.len();
        if line.trim() == close {
            let body_start = (line_end + 1).min(source.len());
            return &source[body_start..];
        }
        offset = line_end + 1;
    }
    source
}

fn frontmatter_scalar(source: &str, key: &str) -> Option<String> {
    let front_matter = front_matter(source)?;
    let lines: Vec<&str> = front_matter.lines().collect();
    lines.iter().enumerate().find_map(|(index, line)| {
        let (name, value) = frontmatter_key_value(line)?;
        if name != key {
            return None;
        }
        let value = value.trim();
        if matches!(value, "|" | ">" | "|-" | ">-") {
            let block = lines
                .iter()
                .skip(index + 1)
                .take_while(|next| next.is_empty() || next.starts_with(char::is_whitespace))
                .map(|next| next.trim())
                .filter(|next| !next.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            return (!block.is_empty()).then_some(block);
        }
        if value.is_empty() {
            return None;
        }
        let value = value.trim_matches(['"', '\'']);
        Some(value.to_string())
    })
}

fn frontmatter_list(source: &str, key: &str) -> Vec<String> {
    let Some(front_matter) = front_matter(source) else {
        return Vec::new();
    };
    let lines: Vec<&str> = front_matter.lines().collect();
    let Some(index) = lines
        .iter()
        .position(|line| frontmatter_key_value(line).is_some_and(|(name, _)| name == key))
    else {
        return Vec::new();
    };
    let Some((_, first_value)) = frontmatter_key_value(lines[index]) else {
        return Vec::new();
    };
    let first_value = first_value.trim();
    let values: Vec<String> = if first_value.starts_with('[') {
        let mut collected = first_value.to_string();
        for line in lines.iter().skip(index + 1) {
            if collected.contains(']') {
                break;
            }
            collected.push_str(line.trim());
        }
        let inner = collected
            .trim_start_matches('[')
            .split(']')
            .next()
            .unwrap_or("");
        split_quoted_values(inner)
    } else {
        lines
            .iter()
            .skip(index + 1)
            .take_while(|line| line.is_empty() || line.starts_with(char::is_whitespace))
            .filter_map(|line| line.trim().strip_prefix("- "))
            .map(|value| value.trim().trim_matches(['"', '\'']).to_string())
            .filter(|value| !value.is_empty())
            .collect()
    };
    values
}

fn frontmatter_key_value(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim();
    let (key, value) = trimmed
        .split_once(':')
        .or_else(|| trimmed.split_once('='))?;
    Some((key.trim().trim_matches(['"', '\'']), value.trim()))
}

fn split_quoted_values(input: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut value = String::new();
    let mut quote = None;
    for character in input.chars() {
        match character {
            '\'' | '"' if quote == Some(character) => quote = None,
            '\'' | '"' if quote.is_none() => quote = Some(character),
            ',' if quote.is_none() => {
                let item = value.trim().trim_matches(['"', '\'']).to_string();
                if !item.is_empty() {
                    values.push(item);
                }
                value.clear();
            }
            _ => value.push(character),
        }
    }
    let item = value.trim().trim_matches(['"', '\'']).to_string();
    if !item.is_empty() {
        values.push(item);
    }
    values
}

fn front_matter(source: &str) -> Option<&str> {
    let opening = source.lines().next()?.trim();
    let close = match opening {
        "---" => "---",
        "+++" => "+++",
        ";;;" => ";;;",
        _ => return None,
    };
    let start = source.find('\n')? + 1;
    let rest = &source[start..];
    let end = rest
        .split_inclusive('\n')
        .scan(0usize, |offset, line| {
            let current = *offset;
            *offset += line.len();
            Some((current, line))
        })
        .find(|(_, line)| line.trim() == close)
        .map(|(offset, _)| offset)?;
    Some(&rest[..end])
}

fn flush_paragraph(paragraph: &mut Vec<String>, blocks: &mut Vec<(String, String)>, kind: &str) {
    if paragraph.is_empty() {
        return;
    }
    let text = clean_inline_markdown(&paragraph.join(" "));
    if !text.is_empty() {
        blocks.push((kind.to_string(), text));
    }
    paragraph.clear();
}

fn first_paragraph(blocks: &[(String, String)]) -> String {
    blocks
        .iter()
        .find(|(kind, text)| kind == "p" && !text.starts_with("• "))
        .map(|(_, text)| text.chars().take(220).collect())
        .unwrap_or_default()
}

fn clean_inline_markdown(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut output = String::new();
    let mut index = 0;
    while index < chars.len() {
        match chars[index] {
            '*' | '_' | '`' => index += 1,
            '[' => {
                if let Some(close) = chars[index + 1..].iter().position(|ch| *ch == ']') {
                    let label_end = index + 1 + close;
                    output.extend(chars[index + 1..label_end].iter());
                    if chars.get(label_end + 1) == Some(&'(') {
                        if let Some(end) = chars[label_end + 2..].iter().position(|ch| *ch == ')') {
                            index = label_end + 3 + end;
                        } else {
                            index = label_end + 1;
                        }
                    } else {
                        index = label_end + 1;
                    }
                } else {
                    output.push(chars[index]);
                    index += 1;
                }
            }
            '!' if chars.get(index + 1) == Some(&'[') => {
                if let Some(close) = chars[index + 2..].iter().position(|ch| *ch == ']') {
                    output.push_str("Image: ");
                    output.extend(chars[index + 2..index + 2 + close].iter());
                    index += 2 + close + 1;
                } else {
                    output.push('!');
                    index += 1;
                }
            }
            character => {
                output.push(character);
                index += 1;
            }
        }
    }
    output.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn title_from_path(path: &str) -> String {
    let filename = Path::new(path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(path);
    filename
        .split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().to_string() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn strip_html_tags(value: &str) -> String {
    let mut output = String::new();
    let mut in_tag = false;
    for character in value.chars() {
        match character {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(character),
            _ => {}
        }
    }
    output.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(PreviewServer::default())
        .setup(|app| {
            let open_repository =
                MenuItemBuilder::with_id("open-hugo-repository", "Open Hugo Repository…")
                    .accelerator("CmdOrCtrl+O")
                    .build(app)?;
            let file_menu = SubmenuBuilder::new(app, "File")
                .item(&open_repository)
                .separator()
                .quit()
                .build()?;
            let edit_menu = SubmenuBuilder::new(app, "Edit")
                .undo()
                .redo()
                .separator()
                .cut()
                .copy()
                .paste()
                .separator()
                .select_all()
                .build()?;

            #[cfg(target_os = "macos")]
            let menu = {
                let app_menu = SubmenuBuilder::new(app, "Hugo Native Client")
                    .about(None)
                    .separator()
                    .services()
                    .separator()
                    .hide()
                    .hide_others()
                    .show_all()
                    .separator()
                    .quit()
                    .build()?;
                MenuBuilder::new(app)
                    .item(&app_menu)
                    .item(&file_menu)
                    .item(&edit_menu)
                    .build()?
            };

            #[cfg(not(target_os = "macos"))]
            let menu = MenuBuilder::new(app)
                .item(&file_menu)
                .item(&edit_menu)
                .build()?;

            app.set_menu(menu)?;
            Ok(())
        })
        .on_menu_event(|app, event| {
            if event.id() == "open-hugo-repository" {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.eval("window.dispatchEvent(new Event('menu-open-project'))");
                }
            }
        })
        .invoke_handler(tauri::generate_handler![inspect_project, load_project])
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                window.app_handle().state::<PreviewServer>().stop();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Hugo Native Client");
}

#[cfg(test)]
mod tests {
    use super::{
        first_paragraph, frontmatter_list, frontmatter_scalar, load_project_root, markdown_blocks,
        parse_csv, strip_front_matter, PreviewServer,
    };

    fn preview_get(address: &str, path: &str) -> String {
        let mut stream =
            std::net::TcpStream::connect(address).expect("preview server should accept requests");
        use std::io::{Read, Write};
        stream.write_all(format!("GET {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\nAccept-Encoding: identity\r\n\r\n").as_bytes()).expect("request should be sent");
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(10)))
            .expect("preview response timeout should be set");
        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .expect("preview response should be readable");
        String::from_utf8_lossy(&response).into_owned()
    }

    #[test]
    fn parses_hugo_csv_fields_with_quoted_commas() {
        let records = parse_csv("path,title,draft\ncontent/a.md,\"Scale, then exit\",false\n");
        assert_eq!(records[1][1], "Scale, then exit");
    }

    #[test]
    fn extracts_yaml_front_matter_and_basic_markdown() {
        let source = "---\ntitle: Demo\n---\n\nIntro paragraph.\n\n## A heading\n";
        let body = strip_front_matter(source);
        let (blocks, unsupported) = markdown_blocks(source);
        assert_eq!(body.trim(), "Intro paragraph.\n\n## A heading");
        assert_eq!(blocks[0], ("p".to_string(), "Intro paragraph.".to_string()));
        assert_eq!(blocks[1], ("h2".to_string(), "A heading".to_string()));
        assert!(!unsupported);
        assert_eq!(first_paragraph(&blocks), "Intro paragraph.");
    }

    #[test]
    fn reads_yaml_and_toml_front_matter_values() {
        let yaml = "---\nsummary: >-\n  One useful line\n  continued here\ntags:\n  - hugo\n  - native-app\n---\nBody";
        assert_eq!(
            frontmatter_scalar(yaml, "summary").as_deref(),
            Some("One useful line continued here")
        );
        assert_eq!(frontmatter_list(yaml, "tags"), vec!["hugo", "native-app"]);
        let toml =
            "+++\nsummary = \"A short summary\"\ntags = [\"hugo\", \"content-tools\"]\n+++\nBody";
        assert_eq!(
            frontmatter_scalar(toml, "summary").as_deref(),
            Some("A short summary")
        );
        assert_eq!(
            frontmatter_list(toml, "tags"),
            vec!["hugo", "content-tools"]
        );
    }

    #[test]
    fn flags_shortcodes_and_raw_html_for_web_fallback() {
        let (blocks, unsupported) =
            markdown_blocks("Text.\n\n{{< feature-grid >}}\n\n<div>theme</div>");
        assert!(unsupported);
        assert_eq!(first_paragraph(&blocks), "Text.");
    }

    #[test]
    fn source_document_preserves_shortcode_lexemes_and_exact_ranges() {
        let source = "---\ntitle: Demo\n---\nBefore {{< note tone=\"warm\" 42 >}}inside {{% child key='v' %}}tail{{% /child %}}{{< /note >}} after";
        let document = super::source_document(source);
        assert_eq!(document["rawSource"], source);
        assert_eq!(document["bodyStartByte"], 20);
        let tokens = document["shortcodeTokens"].as_array().unwrap();
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0]["name"], "note");
        assert_eq!(tokens[0]["parameters"][0]["raw"], "tone=\"warm\"");
        assert_eq!(tokens[0]["parameters"][1]["kind"], "positional");
        assert_eq!(tokens[0]["disposition"], "website-required");
        let range = tokens[0]["sourceRange"]["startByte"].as_u64().unwrap() as usize;
        let end = tokens[0]["sourceRange"]["endByte"].as_u64().unwrap() as usize;
        assert_eq!(&source[range..end], tokens[0]["raw"].as_str().unwrap());
        assert!(tokens[1]["syntax"] == "percent");
        assert_eq!(tokens[2]["closing"], true);
    }

    #[test]
    fn manifest_schema_accepts_source_document_and_rejects_invalid_disposition() {
        let doc = super::source_document("{{< card title=\"Hello\" >}}child{{< /card >}}");
        let flow = super::source_content_flow(
            "{{< card title=\"Hello\" >}}child{{< /card >}}",
            &doc,
            "content/demo.md",
            &std::collections::HashMap::new(),
        );
        let manifest = serde_json::json!({"schemaVersion":1,"site":{"title":"Demo"},"pages":[{
            "id":"content/demo.md","path":"/demo/","title":"Demo",
            "rendering":{"mode":"mixed"},"sourceDocument":doc,"contentFlow":flow
        }]});
        super::validate_manifest_v1(&manifest).expect("fixture should satisfy schema");
        let mut invalid = manifest;
        invalid["pages"][0]["sourceDocument"]["shortcodeTokens"][0]["disposition"] =
            serde_json::json!("guess");
        assert!(super::validate_manifest_v1(&invalid).is_err());
    }

    #[test]
    fn quick_answer_maps_to_a_native_callout_between_surrounding_blocks() {
        let source = "Before.\n\n{{< quick-answer >}}\nReadable **child** text.\n\n{{< /quick-answer >}}\n\nAfter.";
        let doc = super::source_document(source);
        let flow = super::source_content_flow(
            source,
            &doc,
            "content/demo.md",
            &std::collections::HashMap::new(),
        );
        assert_eq!(flow.len(), 3);
        assert_eq!(flow[0]["blocks"][0]["type"], "paragraph");
        assert_eq!(flow[0]["blocks"][0]["inlines"][0]["text"], "Before.");
        assert_eq!(flow[1]["type"], "callout");
        assert_eq!(flow[1]["title"], "Quick Answer");
        assert_eq!(
            flow[1]["children"][0]["inlines"][0]["text"],
            "Readable child text."
        );
        assert_eq!(flow[2]["blocks"][0]["inlines"][0]["text"], "After.");
        let mut doc_with_dispositions = doc;
        super::apply_adapter_dispositions(&mut doc_with_dispositions, &flow);
        assert!(doc_with_dispositions["shortcodeTokens"]
            .as_array()
            .unwrap()
            .iter()
            .all(|token| token["disposition"] == "native"));
    }

    #[test]
    fn malformed_native_wrapper_stays_website_required() {
        let source = "{{< quick-answer >}}No closing tag.";
        let mut doc = super::source_document(source);
        let flow = super::source_content_flow(
            source,
            &doc,
            "content/demo.md",
            &std::collections::HashMap::new(),
        );
        super::apply_adapter_dispositions(&mut doc, &flow);
        assert_eq!(flow[0]["type"], "shortcode");
        assert_eq!(doc["shortcodeTokens"][0]["disposition"], "website-required");
    }

    #[test]
    fn relref_maps_to_a_resolved_native_link_and_faq_items_keep_questions() {
        let source = "A [guide]({{< relref \"/guide\" >}}) link.\n\n{{< faq >}}\n{{% faq-item question=\"Why?\" %}}An answer.{{% /faq-item %}}\n{{< /faq >}}";
        let doc = super::source_document(source);
        let refs =
            std::collections::HashMap::from([(String::from("guide"), String::from("/guide/"))]);
        let flow = super::source_content_flow(source, &doc, "content/index.md", &refs);
        let link = flow.iter().find(|item| item["type"] == "link").unwrap();
        assert_eq!(link["label"], "guide");
        assert_eq!(link["href"], "/guide/");
        assert_eq!(link["inline"], true);
        let faq = flow.iter().find(|item| item["type"] == "faq").unwrap();
        assert_eq!(faq["items"][0]["question"], "Why?");
        assert_eq!(
            faq["items"][0]["answer"][0]["inlines"][0]["text"],
            "An answer."
        );
    }

    #[test]
    fn lead_flattens_and_button_becomes_an_accessible_link() {
        let source = "{{< lead >}}A lead paragraph.{{< /lead >}}\n\n{{< button href=\"/contact\" >}}Contact us{{< /button >}}";
        let doc = super::source_document(source);
        let refs =
            std::collections::HashMap::from([(String::from("contact"), String::from("/contact/"))]);
        let flow = super::source_content_flow(source, &doc, "content/index.md", &refs);
        assert!(flow.iter().any(|item| item["type"] == "text"
            && item["blocks"][0]["inlines"][0]["text"] == "A lead paragraph."));
        assert!(flow.iter().any(|item| item["type"] == "link"
            && item["label"] == "Contact us"
            && item["href"] == "/contact/"));
    }

    #[test]
    fn omits_style_and_script_payloads_from_native_blocks() {
        let source = "Before.\n\n<style>\n.trust-logo { display: flex; }\n</style>\n\n<script>\nrunThemeCode();\n</script>\n\nAfter.";
        let (blocks, unsupported) = markdown_blocks(source);
        assert!(unsupported);
        assert_eq!(first_paragraph(&blocks), "Before.");
        assert!(blocks
            .iter()
            .all(|(_, text)| { !text.contains("trust-logo") && !text.contains("runThemeCode") }));
        assert!(blocks.iter().any(|(_, text)| text == "After."));
    }

    #[test]
    fn loads_a_real_hugo_site_when_pilot_path_is_set() {
        let Ok(path) = std::env::var("HUGO_CLIENT_PILOT_PATH") else {
            return;
        };
        let root = std::path::Path::new(&path)
            .canonicalize()
            .expect("pilot path should resolve");
        let preview = PreviewServer::default();
        let snapshot = load_project_root(&root, &preview).expect("pilot Hugo site should load");
        let pages = snapshot.manifest["pages"].as_array().unwrap();
        let navigation = snapshot.manifest["navigation"].as_array().unwrap();
        let main_menu = navigation
            .iter()
            .filter(|item| item["menu"] == "main")
            .collect::<Vec<_>>();
        let footer_menu = navigation
            .iter()
            .filter(|item| item["menu"] == "footer")
            .collect::<Vec<_>>();
        assert!(pages.len() > 1);
        assert!(!main_menu.is_empty());
        assert!(!snapshot.manifest["site"]["title"]
            .as_str()
            .unwrap()
            .is_empty());
        assert!(pages.iter().any(|page| page["kind"] == "home"));
        assert!(main_menu
            .iter()
            .any(|item| { item["label"] == "Advisory" && item["parent"] == "Services" }));
        assert!(footer_menu.iter().any(|item| item["label"] == "Research"));
        assert!(pages.iter().any(|page| {
            page["sourcePath"]
                .as_str()
                .unwrap_or_default()
                .contains("credit-bureaus")
                && !page["taxonomies"]["tags"]
                    .as_array()
                    .unwrap_or(&Vec::new())
                    .is_empty()
        }));
        let shortcode_article = pages
            .iter()
            .find(|page| {
                page["sourcePath"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("10-million-inbox")
            })
            .expect("known shortcode fixture page should be in the manifest");
        let shortcode_flow = shortcode_article["contentFlow"].as_array().unwrap();
        assert!(
            shortcode_flow
                .iter()
                .any(|item| item["type"] == "callout" && item["title"] == "Quick Answer"),
            "native Quick Answer adapter missing"
        );
        assert!(
            shortcode_flow.iter().any(|item| item["type"] == "faq"
                && item["items"]
                    .as_array()
                    .is_some_and(|items| !items.is_empty())),
            "native FAQ adapter missing"
        );
        let home = pages.iter().find(|page| page["kind"] == "home").unwrap();
        assert!(home["contentFlow"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["type"] == "link" && item["href"] == "/about/"));
        assert_eq!(snapshot.manifest["schemaVersion"], 1);
        assert_eq!(
            snapshot.manifest["site"]["baseURL"],
            "https://januszczak.org/"
        );
        assert!(snapshot.preview_url.starts_with("http://127.0.0.1:"));
        let address = snapshot
            .preview_url
            .trim_start_matches("http://")
            .trim_end_matches('/');
        let response = preview_get(address, "/");
        let expected = "Don’t Just Launch. Scale to Exit.";
        assert!(response.contains("HTTP/1.1 200"));
        assert!(response.contains(expected));
        assert!(
            response.contains("/about/"),
            "Hugo Website output should resolve the home-page relref"
        );
        let button_page = pages
            .iter()
            .find(|page| {
                page["sourcePath"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("corporate-innovation")
            })
            .expect("button adapter fixture page should be in the manifest");
        assert!(button_page["contentFlow"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| {
                item["type"] == "link" && item["label"] == "Fix Your Innovation Strategy"
            }));
        let button_response = preview_get(address, button_page["path"].as_str().unwrap());
        assert!(
            button_response.contains("Fix Your Innovation Strategy"),
            "Hugo Website output should preserve the button shortcode label"
        );
        let article_path = shortcode_article["path"].as_str().unwrap();
        let article_response = preview_get(address, article_path);
        assert!(article_response.contains("HTTP/1.1 200"));
        assert!(
            article_response.contains("Did Google buy Spirit Airlines"),
            "Hugo Website output should retain FAQ questions"
        );
        assert!(
            article_response.contains("The Spirit Airlines data auction is a warning"),
            "Hugo Website output should retain quick-answer content"
        );
        eprintln!(
            "Pilot: {} pages, {} main-menu items, {} footer-menu items",
            pages.len(),
            main_menu.len(),
            footer_menu.len()
        );
        preview.stop();
    }
}
