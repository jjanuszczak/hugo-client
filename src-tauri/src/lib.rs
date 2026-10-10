use serde::Serialize;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
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
        if let Ok(mut slot) = self.child.lock() {
            if let Some(mut child) = slot.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        if let Ok(mut slot) = self.directory.lock() {
            if let Some(directory) = slot.take() {
                let _ = std::fs::remove_dir_all(directory);
            }
        }
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
            has_unsupported_blocks,
        });
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
    let preview_url = start_hugo_preview(root, &hugo, preview)?;
    Ok(ProjectSnapshot {
        summary,
        manifest,
        preview_url,
    })
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
                "blocks": blocks
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
    state.stop();
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
    std::fs::create_dir_all(&cache_dir).map_err(|error| error.to_string())?;
    std::fs::create_dir_all(&resource_dir).map_err(|error| error.to_string())?;
    std::fs::create_dir_all(&destination_dir).map_err(|error| error.to_string())?;
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
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("Could not start Hugo's local preview server: {error}"))?;

    let deadline = Instant::now() + Duration::from_secs(60);
    while Instant::now() < deadline {
        if preview_http_ready(port) {
            let mut slot = state
                .child
                .lock()
                .map_err(|_| "The preview server state is unavailable.".to_string())?;
            *slot = Some(child);
            drop(slot);
            if let Ok(mut directory_slot) = state.directory.lock() {
                *directory_slot = Some(cache_root);
            }
            return Ok(base_url);
        }
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            return Err(format!("Hugo's preview server stopped during startup ({status}). Check the Hugo site configuration."));
        }
        thread::sleep(Duration::from_millis(100));
    }
    let _ = child.kill();
    let _ = child.wait();
    Err("Hugo's local preview server did not become ready within 60 seconds.".to_string())
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
    let output = Command::new(binary)
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| format!("Could not run Hugo: {error}"))?;
    if !output.status.success() {
        let diagnostics = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if diagnostics.is_empty() {
            format!("Hugo exited with status {}.", output.status)
        } else {
            diagnostics
        });
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
        if trimmed.starts_with("{{<") || trimmed.starts_with("{{%") || trimmed.starts_with('<') {
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
        let mut stream =
            std::net::TcpStream::connect(address).expect("preview server should accept requests");
        use std::io::{Read, Write};
        stream
            .write_all(
                format!("GET / HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\nAccept-Encoding: identity\r\n\r\n")
                    .as_bytes(),
            )
            .expect("request should be sent");
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(10)))
            .expect("preview response timeout should be set");
        let mut response = Vec::new();
        let expected = "Don’t Just Launch. Scale to Exit.";
        loop {
            let mut buffer = [0u8; 8192];
            match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    response.extend_from_slice(&buffer[..read]);
                    if response
                        .windows(expected.len())
                        .any(|window| window == expected.as_bytes())
                    {
                        break;
                    }
                }
                Err(error) => panic!("preview response could not be read: {error}"),
            }
        }
        let response = String::from_utf8_lossy(&response);
        assert!(response.contains("HTTP/1.1 200"));
        assert!(response.contains(expected));
        eprintln!(
            "Pilot: {} pages, {} main-menu items, {} footer-menu items",
            pages.len(),
            main_menu.len(),
            footer_menu.len()
        );
        preview.stop();
    }
}
