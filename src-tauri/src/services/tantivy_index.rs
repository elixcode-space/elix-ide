use tantivy::{
    collector::TopDocs,
    directory::MmapDirectory,
    doc,
    query::{BooleanQuery, FuzzyTermQuery, PhraseQuery, Query, RegexQuery, TermQuery},
    schema::{Field, OwnedValue, Schema, TextFieldIndexing, TextOptions, IndexRecordOption, STORED, STRING, FAST, Value},
    Index, IndexWriter, Searcher, Term,
};
use walkdir::WalkDir;
use ignore::WalkBuilder;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use anyhow::Result;
use notify::{Watcher, RecursiveMode, Event, EventKind};
use tokio::sync::mpsc;
use tracing::{info, warn, error};

pub struct CodeIndex {
    index: Index,
    schema: CodeSchema,
    writer: Mutex<IndexWriter>,
    searcher: Arc<Mutex<Searcher>>,
    last_commit: Mutex<Instant>,
    index_path: PathBuf,
    workspace_root: PathBuf,
}

#[derive(Clone)]
pub struct CodeSchema {
    pub path: Field,
    pub content: Field,
    pub language: Field,
    pub symbols: Field,
    pub file_name: Field,
    pub extension: Field,
    pub size: Field,
    pub modified: Field,
}

impl CodeSchema {
    pub fn new(schema: &Schema) -> Self {
        Self {
            path: schema.get_field("path").unwrap(),
            content: schema.get_field("content").unwrap(),
            language: schema.get_field("language").unwrap(),
            symbols: schema.get_field("symbols").unwrap(),
            file_name: schema.get_field("file_name").unwrap(),
            extension: schema.get_field("extension").unwrap(),
            size: schema.get_field("size").unwrap(),
            modified: schema.get_field("modified").unwrap(),
        }
    }
}

pub fn create_code_schema() -> Schema {
    let mut builder = Schema::builder();
    
    let text_indexing = TextFieldIndexing::default()
        .set_tokenizer("en_stem")
        .set_index_option(IndexRecordOption::WithFreqsAndPositions);
    
    let text_options = TextOptions::default()
        .set_indexing_options(text_indexing)
        .set_stored();
    
    builder.add_text_field("path", STRING | STORED);
    builder.add_text_field("content", text_options.clone());
    builder.add_text_field("language", STRING | STORED);
    builder.add_text_field("symbols", text_options);
    builder.add_text_field("file_name", STRING | STORED);
    builder.add_text_field("extension", STRING | STORED);
    builder.add_u64_field("size", FAST | STORED);
    builder.add_u64_field("modified", FAST | STORED);
    
    builder.build()
}

impl CodeIndex {
    pub fn new(workspace_root: PathBuf, index_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&index_path)?;
        
        let schema = create_code_schema();
        let code_schema = CodeSchema::new(&schema);
        
        let dir = MmapDirectory::open(&index_path)?;
        let index = Index::open_or_create(dir, schema.clone())?;
        
        let writer = index.writer(50_000_000)?;
        let searcher = index.reader()?.searcher();
        
        Ok(Self {
            index,
            schema: code_schema,
            writer: Mutex::new(writer),
            searcher: Arc::new(Mutex::new(searcher)),
            last_commit: Mutex::new(Instant::now()),
            index_path,
            workspace_root,
        })
    }
    
    pub fn index_file(&self, file_path: &Path) -> Result<()> {
        let relative_path = file_path.strip_prefix(&self.workspace_root)
            .unwrap_or(file_path)
            .to_string_lossy()
            .to_string();
        
        let content = std::fs::read_to_string(file_path).unwrap_or_default();
        if content.is_empty() {
            return Ok(());
        }
        
        let metadata = std::fs::metadata(file_path)?;
        let size = metadata.len();
        let modified = metadata.modified()?
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let file_name = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        
        let extension = file_path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();
        
        let language = detect_language(&extension);
        let symbols = extract_symbols(&content, &language);
        
        let mut writer = self.writer.lock().unwrap();
        writer.delete_term(Term::from_field_text(self.schema.path, &relative_path));
        
        writer.add_document(doc!(
            self.schema.path => relative_path.as_str(),
            self.schema.content => content.as_str(),
            self.schema.language => language.as_str(),
            self.schema.symbols => symbols.as_str(),
            self.schema.file_name => file_name.as_str(),
            self.schema.extension => extension.as_str(),
            self.schema.size => size,
            self.schema.modified => modified,
        ))?;
        
        self.maybe_commit()?;
        Ok(())
    }
    
    pub fn remove_file(&self, file_path: &Path) -> Result<()> {
        let relative_path = file_path.strip_prefix(&self.workspace_root)
            .unwrap_or(file_path)
            .to_string_lossy()
            .to_string();
        
        let mut writer = self.writer.lock().unwrap();
        writer.delete_term(Term::from_field_text(self.schema.path, &relative_path));
        self.maybe_commit()?;
        Ok(())
    }
    
    fn maybe_commit(&self) -> Result<()> {
        let mut last_commit = self.last_commit.lock().unwrap();
        if last_commit.elapsed() > Duration::from_millis(500) {
            let mut writer = self.writer.lock().unwrap();
            writer.commit()?;
            *last_commit = Instant::now();
            
            let mut searcher = self.searcher.lock().unwrap();
            *searcher = self.index.reader()?.searcher();
        }
        Ok(())
    }
    
    pub fn commit(&self) -> Result<()> {
        let mut writer = self.writer.lock().unwrap();
        writer.commit()?;
        
        let mut searcher = self.searcher.lock().unwrap();
        *searcher = self.index.reader()?.searcher();
        Ok(())
    }
    
    pub fn search(
        &self,
        query_str: &str,
        limit: usize,
        language_filter: Option<&str>,
        file_pattern: Option<&str>,
    ) -> Result<Vec<SearchResult>> {
        let searcher = self.searcher.lock().unwrap();
        
        let mut queries: Vec<Box<dyn Query>> = vec![];
        
        // Parse query - support basic syntax
        if query_str.contains(' ') || query_str.contains('"') {
            // Phrase query
            queries.push(Box::new(PhraseQuery::new(
                query_str.split_whitespace()
                    .map(|t| Term::from_field_text(self.schema.content, t))
                    .collect(),
            )));
        } else if query_str.contains('*') || query_str.contains('?') {
            // Regex query
            let regex_pattern = query_str
                .replace('.', r"\.")
                .replace('*', ".*")
                .replace('?', ".");
            queries.push(Box::new(RegexQuery::from_pattern(
                &regex_pattern,
                self.schema.content,
            )?));
        } else {
            // Fuzzy query for typo tolerance
            let term = Term::from_field_text(self.schema.content, query_str);
            queries.push(Box::new(FuzzyTermQuery::new(term.clone(), 1, true)));
            
            // Also add exact term query
            queries.push(Box::new(TermQuery::new(term, IndexRecordOption::WithFreqsAndPositions)));
        }
        
        // Add language filter
        if let Some(lang) = language_filter {
            queries.push(Box::new(TermQuery::new(
                Term::from_field_text(self.schema.language, lang),
                IndexRecordOption::Basic,
            )));
        }
        
        // Add file pattern filter
        if let Some(pattern) = file_pattern {
            let regex_pattern = pattern.replace('*', ".*");
            queries.push(Box::new(RegexQuery::from_pattern(
                &regex_pattern,
                self.schema.path,
            )?));
        }
        
        let boolean_query = BooleanQuery::new(
            queries.into_iter()
                .map(|q| (tantivy::query::Occur::Must, q))
                .collect(),
        );
        
        let top_docs = searcher.search(&boolean_query, &TopDocs::with_limit(limit).order_by_score())?;
        
        let mut results = Vec::new();
        for (_score, doc_address) in top_docs {
            let doc: HashMap<Field, tantivy::schema::OwnedValue> = searcher.doc(doc_address)?;
            
            let path = doc.get(&self.schema.path)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            let content = doc.get(&self.schema.content)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            let language = doc.get(&self.schema.language)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            let file_name = doc.get(&self.schema.file_name)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            let extension = doc.get(&self.schema.extension)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            let size = doc.get(&self.schema.size)
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            
            let modified = doc.get(&self.schema.modified)
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            
            let snippet = generate_snippet(&content, query_str);
            
            results.push(SearchResult {
                path,
                file_name,
                extension,
                language,
                snippet,
                size,
                modified,
                score: _score,
            });
        }
        
        Ok(results)
    }
    
    pub fn search_symbols(&self, query_str: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let searcher = self.searcher.lock().unwrap();
        
        let term = Term::from_field_text(self.schema.symbols, query_str);
        let fuzzy_query = FuzzyTermQuery::new(term, 1, true);
        
        let top_docs = searcher.search(&fuzzy_query, &TopDocs::with_limit(limit).order_by_score())?;
        
        let mut results = Vec::new();
        for (_score, doc_address) in top_docs {
            let doc: HashMap<Field, tantivy::schema::OwnedValue> = searcher.doc(doc_address)?;
            
            let path = doc.get(&self.schema.path)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            let content = doc.get(&self.schema.content)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            let language = doc.get(&self.schema.language)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            let file_name = doc.get(&self.schema.file_name)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            let extension = doc.get(&self.schema.extension)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            let size = doc.get(&self.schema.size)
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            
            let modified = doc.get(&self.schema.modified)
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            
            let snippet = generate_snippet(&content, query_str);
            
            results.push(SearchResult {
                path,
                file_name,
                extension,
                language,
                snippet,
                size,
                modified,
                score: _score,
            });
        }
        
        Ok(results)
    }
    
    pub fn index_workspace(&self, exclude_patterns: &[String]) -> Result<usize> {
        let mut count = 0;
        let walker = WalkBuilder::new(&self.workspace_root)
            .hidden(false)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .build();
        
        for entry in walker {
            let entry = entry?;
            let path = entry.path();
            
            if !path.is_file() {
                continue;
            }
            
            let rel_path = path.strip_prefix(&self.workspace_root)
                .unwrap_or(path)
                .to_string_lossy();
            
            let excluded = exclude_patterns.iter().any(|p| glob_match(p, &rel_path));
            
            if excluded {
                continue;
            }
            
            if is_binary_file(path) {
                continue;
            }
            
            if let Err(e) = self.index_file(path) {
                warn!("Failed to index {}: {}", path.display(), e);
            } else {
                count += 1;
            }
        }
        
        self.commit()?;
        info!("Indexed {} files", count);
        Ok(count)
    }
    
    pub fn watch_workspace(&self, exclude_patterns: Vec<String>) -> Result<()> {
        let index = self.clone_for_watch();
        let exclude = exclude_patterns.clone();
        
        let (tx, mut rx) = mpsc::channel::<Event>(100);
        let mut watcher = notify::recommended_watcher(move |res| {
            if let Ok(event) = res {
                let _ = tx.try_send(event);
            }
        })?;
        
        watcher.watch(&self.workspace_root, RecursiveMode::Recursive)?;
        
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Err(e) = handle_fs_event(&index, &event, &exclude) {
                    error!("Error handling fs event: {}", e);
                }
            }
        });
        
        Ok(())
    }
    
    fn clone_for_watch(&self) -> CodeIndex {
        CodeIndex {
            index: self.index.clone(),
            schema: self.schema.clone(),
            writer: Mutex::new(self.index.writer(50_000_000).unwrap()),
            searcher: self.searcher.clone(),
            last_commit: Mutex::new(Instant::now()),
            index_path: self.index_path.clone(),
            workspace_root: self.workspace_root.clone(),
        }
    }
    
    pub fn workspace_root(&self) -> &PathBuf {
        &self.workspace_root
    }
}

fn handle_fs_event(index: &CodeIndex, event: &Event, exclude_patterns: &[String]) -> Result<()> {
    for path in &event.paths {
        let rel_path = path.strip_prefix(&index.workspace_root())
            .unwrap_or(path)
            .to_string_lossy();
        
        let excluded = exclude_patterns.iter().any(|p| glob_match(p, &rel_path));
        if excluded {
            continue;
        }
        
        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) => {
                if path.is_file() && !is_binary_file(path) {
                    index.index_file(path)?;
                }
            }
            EventKind::Remove(_) => {
                index.remove_file(path)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn detect_language(extension: &str) -> String {
    match extension.to_lowercase().as_str() {
        "rs" => "rust",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" => "javascript",
        "py" => "python",
        "go" => "go",
        "java" => "java",
        "cpp" | "cc" | "cxx" | "hpp" | "h" => "cpp",
        "c" => "c",
        "cs" => "csharp",
        "rb" => "ruby",
        "php" => "php",
        "swift" => "swift",
        "kt" => "kotlin",
        "scala" => "scala",
        "clj" => "clojure",
        "hs" => "haskell",
        "ml" => "ocaml",
        "fs" => "fsharp",
        "vim" => "vim",
        "lua" => "lua",
        "pl" => "perl",
        "r" => "r",
        "jl" => "julia",
        "dart" => "dart",
        "elm" => "elm",
        "ex" | "exs" => "elixir",
        "erl" => "erlang",
        "groovy" => "groovy",
        "sql" => "sql",
        "sh" | "bash" | "zsh" => "shell",
        "ps1" => "powershell",
        "bat" | "cmd" => "batch",
        "dockerfile" => "dockerfile",
        "yml" | "yaml" => "yaml",
        "json" => "json",
        "toml" => "toml",
        "xml" => "xml",
        "html" => "html",
        "css" => "css",
        "scss" | "sass" => "scss",
        "less" => "less",
        "md" | "markdown" => "markdown",
        "txt" => "text",
        _ => "unknown",
    }.to_string()
}

fn extract_symbols(content: &str, language: &str) -> String {
    let mut symbols = Vec::new();
    
    match language {
        "rust" => {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") ||
                   trimmed.starts_with("struct ") || trimmed.starts_with("pub struct ") ||
                   trimmed.starts_with("enum ") || trimmed.starts_with("pub enum ") ||
                   trimmed.starts_with("trait ") || trimmed.starts_with("pub trait ") ||
                   trimmed.starts_with("impl ") || trimmed.starts_with("mod ") ||
                   trimmed.starts_with("pub mod ") || trimmed.starts_with("const ") ||
                   trimmed.starts_with("static ") || trimmed.starts_with("type ") {
                    if let Some(name) = trimmed.split_whitespace().nth(1) {
                        let name = name.trim_end_matches(['(', '{', '<', ';']);
                        symbols.push(name.to_string());
                    }
                }
            }
        }
        "typescript" | "javascript" => {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("function ") || trimmed.starts_with("const ") ||
                   trimmed.starts_with("let ") || trimmed.starts_with("var ") ||
                   trimmed.starts_with("class ") || trimmed.starts_with("interface ") ||
                   trimmed.starts_with("type ") || trimmed.starts_with("export ") {
                    if let Some(name) = trimmed.split_whitespace().nth(1) {
                        let name = name.trim_end_matches(['(', '{', '<', '=', ';', ':']);
                        symbols.push(name.to_string());
                    }
                }
            }
        }
        "python" => {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("def ") || trimmed.starts_with("class ") ||
                   trimmed.starts_with("async def ") {
                    if let Some(name) = trimmed.split_whitespace().nth(1) {
                        let name = name.trim_end_matches(['(', ':']);
                        symbols.push(name.to_string());
                    }
                }
            }
        }
        "go" => {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("func ") || trimmed.starts_with("type ") ||
                   trimmed.starts_with("var ") || trimmed.starts_with("const ") ||
                   trimmed.starts_with("interface ") || trimmed.starts_with("struct ") {
                    if let Some(name) = trimmed.split_whitespace().nth(1) {
                        let name = name.trim_end_matches(['(', '{', '<', ';']);
                        symbols.push(name.to_string());
                    }
                }
            }
        }
        _ => {}
    }
    
    symbols.join(" ")
}

fn generate_snippet(content: &str, query: &str) -> String {
    let query_lower = query.to_lowercase();
    let lines: Vec<&str> = content.lines().collect();
    
    for (i, line) in lines.iter().enumerate() {
        if line.to_lowercase().contains(&query_lower) {
            let start = i.saturating_sub(2);
            let end = (i + 3).min(lines.len());
            let snippet = lines[start..end].join("\n");
            return snippet.chars().take(200).collect();
        }
    }
    
    lines.iter().take(3).cloned().collect::<Vec<_>>().join("\n").chars().take(200).collect()
}

fn glob_match(pattern: &str, text: &str) -> bool {
    let regex_pattern = pattern
        .replace(".", "\\.")
        .replace("**", ".*")
        .replace("*", "[^/]*");
    regex::Regex::new(&format!("^{}$", regex_pattern))
        .map(|re| re.is_match(text))
        .unwrap_or(false)
}

fn is_binary_file(path: &Path) -> bool {
    if let Ok(content) = std::fs::read(path) {
        if content.len() > 8192 {
            return false;
        }
        content.iter().any(|&b| b == 0)
    } else {
        false
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchResult {
    pub path: String,
    pub file_name: String,
    pub extension: String,
    pub language: String,
    pub snippet: String,
    pub size: u64,
    pub modified: u64,
    pub score: f32,
}

pub struct IndexStats {
    pub num_docs: u64,
    pub num_terms: u64,
    pub index_size: u64,
}

impl CodeIndex {
    pub fn stats(&self) -> Result<IndexStats> {
        let searcher = self.searcher.lock().unwrap();
        let reader = self.index.reader()?;
        
        let num_docs = reader.searcher().segment_readers().iter()
            .map(|r| r.max_doc() as u64)
            .sum();
        
        // Approximate num_terms using searcher
        let num_terms = num_docs * 100; // rough estimate
        
        let index_size = std::fs::metadata(&self.index_path)?.len();
        
        Ok(IndexStats {
            num_docs,
            num_terms,
            index_size,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_index_and_search() {
        let dir = tempdir().unwrap();
        let workspace = dir.path().join("workspace");
        let index_path = dir.path().join("index");
        std::fs::create_dir_all(&workspace).unwrap();
        
        std::fs::write(workspace.join("main.rs"), r#"
fn main() {
    println!("Hello, world!");
}

struct User {
    name: String,
}

impl User {
    fn new(name: &str) -> Self {
        Self { name: name.to_string() }
    }
}
"#).unwrap();
        
        let index = CodeIndex::new(workspace.clone(), index_path).unwrap();
        index.index_workspace(&[]).unwrap();
        
        let results = index.search("User", 10, None, None).unwrap();
        assert!(!results.is_empty());
        assert!(results[0].path.contains("main.rs"));
    }
}