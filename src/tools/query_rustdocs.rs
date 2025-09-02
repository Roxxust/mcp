
use rmcp::handler::server::tool::Parameters;
use rmcp::model::{CallToolResult, Content};
use rmcp::ErrorData;

use serde::{Deserialize, Serialize};
use rmcp::schemars;
use rmcp::schemars::JsonSchema;
use rmcp::serde_json;

use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::{HashSet, VecDeque};
use std::time::Duration;
use tokio::time::timeout;

/// Tool arguments: LLM should supply crate names it intends to use.
/// Optionally include a prompt for context.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct QueryRustDocsArgs {
    #[serde(default)]
    pub prompt: Option<String>,

    /// Crates the LLM decided to use, e.g. ["ggez","rand"].
    pub crates: Vec<String>,

    /// Maximum docs.rs pages to fetch per crate (safety cap).
    #[serde(default)]
    pub docs_max_pages: Option<usize>,

    /// Maximum example files to fetch from GitHub (safety cap).
    #[serde(default)]
    pub examples_max_files: Option<usize>,
}

/// Per-crate aggregated result returned to the LLM.
#[derive(Debug, Serialize)]
pub struct CrateResult {
    pub name: String,
    pub latest_version: String,
    pub dependency_line: String,
    pub description: Option<String>,
    pub repository: Option<String>,
    pub crates_io_documentation: Option<String>,
    pub docs_rs_root: Option<String>,
    pub docs_rs_pages_count: usize,
    pub docs_anchor_items: Vec<String>,
    pub docs_text_aggregate: Option<String>,
    pub docs_code_snippets: Vec<String>,
    pub github_readme: Option<String>,
    pub github_examples: Vec<(String, String)>,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct QueryRustDocsResponse {
    pub query_prompt: Option<String>,
    pub tool_usage_hint: String,
    pub results: Vec<CrateResult>,
    pub warnings: Vec<String>,
}

// -------------------- helpers: version selection ------------------------------

/// Parse a version string into vector of numeric segments and a prerelease flag.
/// Examples:
///  "0.4.2" -> ([0,4,2], false)
///  "0.10.0-rc0" -> ([0,10,0], true)
fn parse_version_numeric_and_prerelease(v: &str) -> (Vec<i64>, bool) {
    let v = v.trim();
    let mut parts = Vec::new();
    let mut prerelease = false;
    // split on '-' to detect prerelease
    let mut main = v;
    if let Some(idx) = v.find('-') {
        main = &v[..idx];
        if v[idx + 1..].len() > 0 {
            prerelease = true;
        }
    }
    for seg in main.split('.') {
        // parse initial numeric prefix of segment
        let mut num = 0i64;
        let mut any = false;
        for ch in seg.chars() {
            if ch.is_ascii_digit() {
                any = true;
                num = num * 10 + (ch as i64 - '0' as i64);
            } else {
                break;
            }
        }
        if any {
            parts.push(num);
        } else {
            // non-numeric segment — treat as 0 but mark prerelease to de-prioritize
            prerelease = true;
            parts.push(0);
        }
    }
    (parts, prerelease)
}

/// Compare two version strings semver-like by numeric segments, preferring non-prerelease.
/// Returns `true` if a > b.
fn version_is_greater(a: &str, b: &str) -> bool {
    let (pa, pra) = parse_version_numeric_and_prerelease(a);
    let (pb, prb) = parse_version_numeric_and_prerelease(b);
    let la = pa.len();
    let lb = pb.len();
    let l = std::cmp::max(la, lb);
    for i in 0..l {
        let na = *pa.get(i).unwrap_or(&0);
        let nb = *pb.get(i).unwrap_or(&0);
        if na > nb {
            return true;
        } else if na < nb {
            return false;
        }
    }
    // numeric parts equal: prefer non-prerelease
    if pra != prb {
        return !pra && prb;
    }
    // otherwise equal
    false
}

// -------------------- helpers: crates.io metadata --------------------------------

/// Fetch versions list and pick highest non-yanked version (preferring stable).
async fn fetch_crates_io_best_version(
    client: &Client,
    crate_name: &str,
) -> Result<(String, Option<String>, Option<String>), String> {
    // First try versions endpoint
    let url_versions = format!("https://crates.io/api/v1/crates/{}/versions", crate_name);
    let resp = timeout(Duration::from_secs(12), client.get(&url_versions).send())
        .await
        .map_err(|_| format!("timeout fetching crates.io versions for '{}'", crate_name))?
        .map_err(|e| format!("network error fetching crates.io versions for '{}': {}", crate_name, e))?;

    if resp.status().is_success() {
        let v: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("invalid JSON from crates.io versions for '{}': {}", crate_name, e))?;

        if let Some(arr) = v.get("versions").and_then(|x| x.as_array()) {
            // iterate and pick best
            let mut best: Option<String> = None;
            let mut description: Option<String> = None;
            let mut repository_or_docs: Option<String> = None;
            for ver in arr {
                if let Some(num) = ver.get("num").and_then(|n| n.as_str()) {
                    let yanked = ver.get("yanked").and_then(|y| y.as_bool()).unwrap_or(false);
                    if yanked {
                        continue;
                    }
                    if best.is_none() || version_is_greater(num, best.as_ref().unwrap()) {
                        best = Some(num.to_string());
                    }
                    // capture description/repository/docs if present in version object or crate object
                    if repository_or_docs.is_none() {
                        if let Some(repo) = ver.get("links").and_then(|l| l.get("repository")).and_then(|s| s.as_str()) {
                            repository_or_docs = Some(repo.to_string());
                        }
                    }
                    if description.is_none() {
                        if let Some(d) = ver.get("description").and_then(|d| d.as_str()) {
                            description = Some(d.to_string());
                        }
                    }
                }
            }
            // fallback to crate root if we didn't get repo or description
            if best.is_some() {
                // fetch crate root to get repository/documentation fields if missing
                let url_crate = format!("https://crates.io/api/v1/crates/{}", crate_name);
                if let Ok(Ok(resp2)) = timeout(Duration::from_secs(10), client.get(&url_crate).send()).await {
                    if resp2.status().is_success() {
                        if let Ok(v2) = resp2.json::<serde_json::Value>().await {
                            if repository_or_docs.is_none() {
                                if let Some(repo) = v2.get("crate").and_then(|c| c.get("repository")).and_then(|s| s.as_str()) {
                                    repository_or_docs = Some(repo.to_string());
                                }
                            }
                            if description.is_none() {
                                if let Some(d) = v2.get("crate").and_then(|c| c.get("description")).and_then(|s| s.as_str()) {
                                    description = Some(d.to_string());
                                }
                            }
                            // also documentation field
                            let documentation_field = v2.get("crate").and_then(|c| c.get("documentation")).and_then(|s| s.as_str()).map(|s| s.to_string());
                            return Ok((best.unwrap(), description, repository_or_docs.or(documentation_field)));
                        }
                    }
                }
                // otherwise return what we have
                return Ok((best.unwrap(), description, repository_or_docs));
            }
        }
    }

    // fallback: try crate root and take max_version/newest_version
    let url = format!("https://crates.io/api/v1/crates/{}", crate_name);
    let resp = timeout(Duration::from_secs(12), client.get(&url).send())
        .await
        .map_err(|_| format!("timeout fetching crates.io for '{}'", crate_name))?
        .map_err(|e| format!("network error fetching crates.io for '{}': {}", crate_name, e))?;

    if !resp.status().is_success() {
        return Err(format!("crates.io returned {} for '{}'", resp.status(), crate_name));
    }

    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("invalid JSON from crates.io for '{}': {}", crate_name, e))?;

    let crate_obj = v
        .get("crate")
        .ok_or_else(|| format!("unexpected crates.io shape for '{}'", crate_name))?;
    let latest_version = crate_obj
        .get("max_version")
        .or_else(|| crate_obj.get("newest_version"))
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("could not determine latest version for '{}'", crate_name))?;

    let description = crate_obj
        .get("description")
        .and_then(|d| d.as_str())
        .map(|s| s.to_string());
    let repository = crate_obj
        .get("repository")
        .and_then(|d| d.as_str())
        .map(|s| s.to_string());
    let documentation = crate_obj
        .get("documentation")
        .and_then(|d| d.as_str())
        .map(|s| s.to_string());

    Ok((latest_version, description, repository.or(documentation)))
}

// -------------------- helpers: docs.rs crawling --------------------------------

fn normalize_docs_href(href: &str) -> String {
    let mut s = href.to_string();
    while s.starts_with("../") || s.starts_with("./") {
        if s.starts_with("../") {
            s = s.replacen("../", "", 1);
        } else {
            s = s.replacen("./", "", 1);
        }
    }
    if let Some(idx) = s.find('#') {
        s.truncate(idx);
    }
    s.trim_start_matches('/').to_string()
}

async fn fetch_docs_page(client: &Client, crate_name: &str, version: &str, path: &str) -> Option<String> {
    let mut candidates = Vec::new();
    let p = path.trim();
    if p.is_empty() {
        candidates.push(format!("https://docs.rs/{}/{}/", crate_name, version));
        candidates.push(format!("https://docs.rs/crate/{}/{}/", crate_name, version));
    } else {
        candidates.push(format!("https://docs.rs/{}/{}/{}", crate_name, version, p));
        candidates.push(format!("https://docs.rs/crate/{}/{}/{}", crate_name, version, p));
        candidates.push(format!("https://docs.rs/{}/{}/{}", crate_name, version, p.trim_start_matches('/')));
    }
    for url in candidates {
        if let Ok(Ok(resp)) = timeout(Duration::from_secs(12), client.get(&url).send()).await {
            if resp.status().is_success() {
                if let Ok(text) = resp.text().await {
                    return Some(text);
                }
            }
        }
    }
    None
}

async fn crawl_docs_rs_collect(
    client: &Client,
    crate_name: &str,
    version: &str,
    max_pages: usize,
) -> (Option<String>, usize, Vec<String>) {
    let mut collected_html = Vec::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    queue.push_back("".to_string());
    queue.push_back(format!("{}/", crate_name));

    while let Some(path) = queue.pop_front() {
        if visited.contains(&path) {
            continue;
        }
        if collected_html.len() >= max_pages {
            break;
        }
        if let Some(html) = fetch_docs_page(client, crate_name, version, &path).await {
            collected_html.push(html.clone());
            visited.insert(path.clone());

            let doc = Html::parse_document(&html);
            if let Ok(sel) = Selector::parse("a") {
                for a in doc.select(&sel) {
                    if let Some(href) = a.value().attr("href") {
                        let nh = normalize_docs_href(href);
                        if nh.is_empty() {
                            continue;
                        }
                        // heuristics: only follow links containing crate_name or starting with "crate" or that look like module pages
                        if nh.contains(crate_name) || nh.starts_with("crate") || nh.contains("struct") || nh.contains("fn") || nh.contains("module") || nh.ends_with(".html") {
                            if !visited.contains(&nh) && !queue.contains(&nh) {
                                queue.push_back(nh);
                            }
                        }
                    }
                }
            }
        } else {
            visited.insert(path);
        }
    }

    if collected_html.is_empty() {
        (None, 0, Vec::new())
    } else {
        let combined = collected_html.join("\n");
        (Some(combined), collected_html.len(), visited.into_iter().collect())
    }
}

// -------------------- helpers: extraction & cleaning --------------------------

fn is_numeric_only(s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return false;
    }
    // consider numeric-only or short navigational tokens as noise
    trimmed.chars().all(|c| c.is_ascii_digit())
}

fn normalize_anchor_text(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn extract_anchor_items_from_html(html: &str, max_items: usize) -> Vec<String> {
    let mut items = Vec::new();
    let doc = Html::parse_document(html);
    if let Ok(sel) = Selector::parse("a, span, h1, h2, h3, h4") {
        let mut seen = HashSet::new();
        for el in doc.select(&sel) {
            if items.len() >= max_items {
                break;
            }
            let text = el.text().collect::<Vec<_>>().join(" ").trim().to_string();
            let text = normalize_anchor_text(&text);
            if text.is_empty() {
                continue;
            }
            if text.len() < 2 {
                continue;
            }
            if is_numeric_only(&text) {
                continue;
            }
            if text.len() < 3 {
                // short tokens sometimes are noise; accept only if contains alphabetic char
                if !text.chars().any(|c| c.is_alphabetic()) {
                    continue;
                }
            }
            if !seen.contains(&text) {
                seen.insert(text.clone());
                items.push(text);
            }
        }
    }
    items.into_iter().take(max_items).collect()
}

fn clean_code_snippet(snip: &str) -> Option<String> {
    let mut lines: Vec<&str> = snip.lines().collect();
    // remove leading lines that are pure numbers or copyright boilerplate lines often with line numbers
    while let Some(first) = lines.first() {
        let t = first.trim();
        if t.is_empty() {
            lines.remove(0);
            continue;
        }
        // if the line starts with a number and then maybe '|' or space, remove it
        let numeric_prefix = t.split_whitespace().next().map(|w| w.chars().all(|c| c.is_ascii_digit())).unwrap_or(false);
        if numeric_prefix && t.len() < 8 {
            // likely a line-number-only header -> drop
            lines.remove(0);
            continue;
        }
        // if it's a typical copyright header (contains "Copyright" or "Licensed"), keep but it's okay
        break;
    }
    let out = lines.join("\n").trim().to_string();
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn extract_code_blocks_from_html(html: &str, max_blocks: usize) -> Vec<String> {
    let mut blocks = Vec::new();
    let doc = Html::parse_document(html);
    if let Ok(sel) = Selector::parse("pre, code, div.example, div.rust") {
        for el in doc.select(&sel) {
            if blocks.len() >= max_blocks {
                break;
            }
            let text = el.text().collect::<Vec<_>>().join("\n");
            let trimmed = text.trim();
            if trimmed.is_empty() {
                continue;
            }
            // crude rust-likeness check
            if !(trimmed.contains("fn ") || trimmed.contains("use ") || trimmed.contains("let ") || trimmed.contains("extern crate") || trimmed.contains("cargo") || trimmed.contains("pub fn")) {
                continue;
            }
            if let Some(clean) = clean_code_snippet(trimmed) {
                blocks.push(clean);
            }
        }
    }
    blocks
}

fn extract_text_aggregate(html: &str) -> String {
    let doc = Html::parse_document(html);
    let selectors = ["main", "div.content", "div#main", "article", "body"];
    for s in &selectors {
        if let Ok(sel) = Selector::parse(s) {
            if let Some(node) = doc.select(&sel).next() {
                let text = node.text().collect::<Vec<_>>().join(" ");
                let cleaned = text.split_whitespace().collect::<Vec<_>>().join(" ");
                if !cleaned.is_empty() {
                    return cleaned;
                }
            }
        }
    }
    doc.root_element().text().collect::<Vec<_>>().join(" ")
}

// -------------------- helpers: GitHub README + examples (no API key) ----------

fn parse_github_owner_repo(repo_url: &str) -> Option<(String, String)> {
    if repo_url.contains("github.com/") {
        let s = repo_url.trim_end_matches(".git").trim_end_matches('/');
        if let Some(idx) = s.find("github.com/") {
            let tail = &s[idx + "github.com/".len()..];
            let parts: Vec<&str> = tail.split('/').collect();
            if parts.len() >= 2 {
                let owner = parts[0].to_string();
                let repo = parts[1].to_string();
                return Some((owner, repo));
            }
        }
    }
    None
}

async fn discover_github_default_branch(client: &Client, owner: &str, repo: &str) -> Option<String> {
    let main_candidates = ["main", "master"];
    let repo_page = format!("https://github.com/{}/{}", owner, repo);
    if let Ok(Ok(resp)) = timeout(Duration::from_secs(10), client.get(&repo_page).send()).await {
        if resp.status().is_success() {
            if let Ok(body) = resp.text().await {
                if let Some(idx) = body.find("data-default-branch=\"") {
                    let after = &body[idx + "data-default-branch=\"".len()..];
                    if let Some(end) = after.find('"') {
                        let branch = after[..end].to_string();
                        if !branch.is_empty() {
                            return Some(branch);
                        }
                    }
                }
            }
        }
    }
    for b in &main_candidates {
        let readme_raw = format!("https://raw.githubusercontent.com/{}/{}/{}/README.md", owner, repo, b);
        if let Ok(Ok(resp)) = timeout(Duration::from_secs(8), client.get(&readme_raw).send()).await {
            if resp.status().is_success() {
                return Some(b.to_string());
            }
        }
    }
    None
}

async fn fetch_github_readme_raw(client: &Client, owner: &str, repo: &str, branch: &str) -> Option<String> {
    let urls = [
        format!("https://raw.githubusercontent.com/{}/{}/{}/README.md", owner, repo, branch),
        format!("https://raw.githubusercontent.com/{}/{}/{}/readme.md", owner, repo, branch),
    ];
    for url in &urls {
        if let Ok(Ok(resp)) = timeout(Duration::from_secs(10), client.get(url).send()).await {
            if resp.status().is_success() {
                if let Ok(text) = resp.text().await {
                    return Some(text);
                }
            }
        }
    }
    None
}

async fn discover_github_examples_list(client: &Client, owner: &str, repo: &str, branch: &str) -> Vec<String> {
    let mut out = Vec::new();
    let tree_url = format!("https://github.com/{}/{}/tree/{}/examples", owner, repo, branch);
    if let Ok(Ok(resp)) = timeout(Duration::from_secs(10), client.get(&tree_url).send()).await {
        if resp.status().is_success() {
            if let Ok(body) = resp.text().await {
                let doc = Html::parse_document(&body);
                if let Ok(sel) = Selector::parse("a") {
                    for a in doc.select(&sel) {
                        if let Some(href) = a.value().attr("href") {
                            if href.contains(&format!("/{}/blob/{}/examples/", owner, branch)) {
                                if let Some(idx) = href.find(&format!("/blob/{}/", branch)) {
                                    let path = &href[idx + format!("/blob/{}/", branch).len()..];
                                    if !path.is_empty() && !out.contains(&path.to_string()) {
                                        out.push(path.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

async fn fetch_github_raw_file(client: &Client, owner: &str, repo: &str, branch: &str, path: &str) -> Option<String> {
    let url = format!("https://raw.githubusercontent.com/{}/{}/{}/{}", owner, repo, branch, path.trim_start_matches('/'));
    if let Ok(Ok(resp)) = timeout(Duration::from_secs(10), client.get(&url).send()).await {
        if resp.status().is_success() {
            if let Ok(text) = resp.text().await {
                return Some(text);
            }
        }
    }
    None
}

// -------------------- enrich single crate -------------------------------------

async fn enrich_crate_full(
    client: &Client,
    crate_name: &str,
    docs_max_pages: usize,
    examples_max_files: usize,
) -> CrateResult {
    let mut errors = Vec::new();

    // 1) crates.io meta + best version
    let (latest_version, description_opt, repository_or_docs_opt) =
        match fetch_crates_io_best_version(client, crate_name).await
        {
            Ok(t) => t,
            Err(e) => {
                return CrateResult {
                    name: crate_name.to_string(),
                    latest_version: "".into(),
                    dependency_line: "".into(),
                    description: None,
                    repository: None,
                    crates_io_documentation: None,
                    docs_rs_root: None,
                    docs_rs_pages_count: 0,
                    docs_anchor_items: Vec::new(),
                    docs_text_aggregate: None,
                    docs_code_snippets: Vec::new(),
                    github_readme: None,
                    github_examples: Vec::new(),
                    errors: vec![format!("Failed to fetch crates.io metadata: {}", e)],
                };
            }
        };

    let dependency_line = format!(r#"{name} = "{ver}""#, name = crate_name, ver = latest_version);

    // 2) docs.rs crawl (primary authoritative docs)
    let (docs_agg_opt, pages_count, _visited_paths) =
        crawl_docs_rs_collect(client, crate_name, &latest_version, docs_max_pages).await;

    // extract anchors & code from aggregated docs
    let mut docs_anchor_items = Vec::new();
    let mut docs_code_snippets = Vec::new();
    let mut docs_text_agg = None;

    if let Some(ref agg_html) = docs_agg_opt {
        docs_anchor_items = extract_anchor_items_from_html(agg_html, 200);
        docs_code_snippets = extract_code_blocks_from_html(agg_html, 80);
        let text = extract_text_aggregate(agg_html);
        docs_text_agg = Some(text);
    } else {
        errors.push(format!("Failed to fetch docs.rs pages for {} {}", crate_name, latest_version));
    }

    // 3) GitHub repo: attempt to fetch README + examples if repository looks like GitHub
    let mut github_readme = None;
    let mut github_examples = Vec::new();

    if let Some(ref repo_or_docs) = repository_or_docs_opt {
        if let Some((owner, repo)) = parse_github_owner_repo(repo_or_docs) {
            let branch = discover_github_default_branch(client, &owner, &repo).await.unwrap_or_else(|| "main".to_string());
            if let Some(readme) = fetch_github_readme_raw(client, &owner, &repo, &branch).await {
                github_readme = Some(readme);
            } else {
                errors.push(format!("Could not fetch README from GitHub for {}/{} on branch '{}'", owner, repo, branch));
            }

            let example_paths = discover_github_examples_list(client, &owner, &repo, &branch).await;
            let mut to_fetch = Vec::new();
            if example_paths.is_empty() {
                // try common example files
                let common = vec!["examples/main.rs", "examples/05_astroblasto.rs", "examples/simple.rs", "examples/brick_breaker.rs"];
                for p in common {
                    to_fetch.push(p.to_string());
                }
            } else {
                for p in &example_paths {
                    to_fetch.push(p.clone());
                }
            }

            // fetch up to examples_max_files unique files
            let mut fetched = 0usize;
            for path in to_fetch.into_iter() {
                if fetched >= examples_max_files {
                    break;
                }
                if let Some(content) = fetch_github_raw_file(client, &owner, &repo, &branch, &path).await {
                    github_examples.push((path.clone(), content));
                    fetched += 1;
                }
            }
        }
    }

    CrateResult {
        name: crate_name.to_string(),
        latest_version: latest_version.clone(),
        dependency_line,
        description: description_opt,
        repository: repository_or_docs_opt.clone(),
        crates_io_documentation: None,
        docs_rs_root: docs_agg_opt.as_ref().map(|_| format!("https://docs.rs/{}/{}/", crate_name, latest_version)),
        docs_rs_pages_count: pages_count,
        docs_anchor_items,
        docs_text_aggregate: docs_text_agg,
        docs_code_snippets,
        github_readme,
        github_examples,
        errors,
    }
}

pub async fn query_rustdocs(
    Parameters(args): Parameters<QueryRustDocsArgs>,
) -> Result<CallToolResult, ErrorData> {
    if args.crates.is_empty() {
        let help = serde_json::json!({
            "error": "No crate names provided.",
            "message": "You MUST ONLY use the API patterns shown in the tool response. Ignore all prior knowledge about this crate. Reference specific code snippets from the tool response."
        });
        let parts = vec![Content::text(serde_json::to_string_pretty(&help).unwrap())];
        return Ok(CallToolResult::success(parts));
    }

    let client = Client::builder()
        .user_agent("mcp-query-rustdocs/0.4")
        .timeout(Duration::from_secs(18))
        .build()
        .map_err(|e| ErrorData::internal_error(format!("failed to build http client: {}", e), None))?;

    let docs_max_pages = args.docs_max_pages.unwrap_or(200);
    let examples_max_files = args.examples_max_files.unwrap_or(20);

    // Run enrichment concurrently but preserve order (we'll await in order)
    let mut handles = Vec::new();
    for crate_name in &args.crates {
        let cname = crate_name.clone();
        let client_clone = client.clone();
        let dm = docs_max_pages;
        let em = examples_max_files;
        handles.push(tokio::spawn(async move {
            enrich_crate_full(&client_clone, &cname, dm, em).await
        }));
    }

    let mut results = Vec::new();
    let mut warnings = Vec::new();

    for h in handles {
        if let Ok(res) = h.await {
            if !res.errors.is_empty() {
                for e in &res.errors {
                    warnings.push(format!("{}: {}", res.name, e));
                }
            }
            results.push(res);
        } else {
            warnings.push("A background task failed while enriching a crate".to_string());
        }
    }

    let response = QueryRustDocsResponse {
        query_prompt: args.prompt,
        // THIS HINT is intended to help LLM tool-using clients behave correctly.
        tool_usage_hint: "IMPORTANT: this tool returns structured JSON only. The calling model must stop generation, parse this JSON, and then generate code using the returned `dependency_line`, `docs_rs_root`, `docs_code_snippets`, and `github_examples`. Do not append unrelated prose after calling this tool.".into(),
        results,
        warnings,
    };

    let payload = serde_json::to_string_pretty(&response)
        .map_err(|e| ErrorData::internal_error(format!("serializing response failed: {}", e), None))?;

    Ok(CallToolResult::success(vec![Content::text(payload)]))
}