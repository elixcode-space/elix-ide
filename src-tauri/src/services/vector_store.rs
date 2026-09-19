//! Local Vector Search with LanceDB
//!
//! Provides codebase indexing and semantic search for RAG (Retrieval-Augmented Generation)
//! using LanceDB embedded database.

use anyhow::Result;
use arrow::array::{Float32Array, StringArray, UInt64Array};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use lancedb::Table;
use lancedb::connect;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Schema for code chunks table
pub static CODE_CHUNK_SCHEMA: Lazy<Arc<Schema>> = Lazy::new(|| {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::UInt64, false),
        Field::new("file_path", DataType::Utf8, false),
        Field::new("language", DataType::Utf8, false),
        Field::new("content", DataType::Utf8, false),
        Field::new("start_line", DataType::UInt32, false),
        Field::new("end_line", DataType::Utf8, false),
        Field::new("embedding", DataType::List(Arc::new(Field::new("item", DataType::Float32, true))), false),
        Field::new("created_at", DataType::Utf8, false),
    ]))
});

/// Code chunk with embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChunk {
    pub id: u64,
    pub file_path: String,
    pub language: String,
    pub content: String,
    pub start_line: u32,
    pub end_line: u32,
    pub embedding: Vec<f32>,
    pub created_at: String,
}

/// Search result with score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub chunk: CodeChunk,
    pub score: f32,
}

/// Vector search configuration
#[derive(Debug, Clone)]
pub struct VectorSearchConfig {
    pub table_name: String,
    pub embedding_dim: usize,
    pub max_results: usize,
    pub similarity_threshold: f32,
}

impl Default for VectorSearchConfig {
    fn default() -> Self {
        Self {
            table_name: "code_chunks".to_string(),
            embedding_dim: 384, // BGE-small-en-v1.5 dimension
            max_results: 10,
            similarity_threshold: 0.7,
        }
    }
}

/// LanceDB vector store for codebase RAG
pub struct VectorStore {
    db: Arc<Mutex<Option<lancedb::Connection>>>,
    table: Arc<RwLock<Option<Table>>>,
    config: VectorSearchConfig,
    db_path: String,
    next_id: Arc<Mutex<u64>>,
}

impl VectorStore {
    /// Create a new vector store
    pub async fn new(db_path: &str, config: VectorSearchConfig) -> Result<Self> {
        let db = connect(db_path).execute().await?;
        
        let store = Self {
            db: Arc::new(Mutex::new(Some(db))),
            table: Arc::new(RwLock::new(None)),
            config,
            db_path: db_path.to_string(),
            next_id: Arc::new(Mutex::new(0)),
        };

        // Initialize table
        store.init_table().await?;
        
        Ok(store)
    }

    /// Initialize the code chunks table
    async fn init_table(&self) -> Result<()> {
        let db_guard = self.db.lock();
        let db = db_guard.as_ref().unwrap();
        
        let table_names = db.table_names().execute().await?;
        
        if table_names.contains(&self.config.table_name) {
            let table = db.open_table(&self.config.table_name).execute().await?;
            *self.table.write().await = Some(table);
        } else {
            // Create empty table with schema
            let schema = CODE_CHUNK_SCHEMA.clone();
            let empty_batch = RecordBatch::new_empty(schema);
            let table = db.create_table(&self.config.table_name, empty_batch).execute().await?;
            *self.table.write().await = Some(table);
        }
        
        // Get next ID
        if let Some(table) = self.table.read().await.as_ref() {
            let count = table.count_rows(None).await?;
            *self.next_id.lock() = count as u64;
        }
        
        Ok(())
    }

    /// Add code chunks to the index
    pub async fn add_chunks(&self, chunks: Vec<CodeChunk>) -> Result<()> {
        let table = self
            .table
            .read()
            .await
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Table not initialized"))?
            .clone();

        let mut id_counter = self.next_id.lock();
        
        // Prepare data for Arrow
        let num_chunks = chunks.len();
        let mut ids = Vec::with_capacity(num_chunks);
        let mut file_paths = Vec::with_capacity(num_chunks);
        let mut languages = Vec::with_capacity(num_chunks);
        let mut contents = Vec::with_capacity(num_chunks);
        let mut start_lines = Vec::with_capacity(num_chunks);
        let mut end_lines = Vec::with_capacity(num_chunks);
        let mut embeddings = Vec::with_capacity(num_chunks);
        let mut created_ats = Vec::with_capacity(num_chunks);

        for chunk in chunks {
            ids.push(*id_counter);
            *id_counter += 1;
            file_paths.push(chunk.file_path);
            languages.push(chunk.language);
            contents.push(chunk.content);
            start_lines.push(chunk.start_line);
            end_lines.push(chunk.end_line.to_string());
            embeddings.push(chunk.embedding);
            created_ats.push(chunk.created_at);
        }

        // Create arrays
        let id_array = UInt64Array::from(ids);
        let file_path_array = StringArray::from(file_paths);
        let language_array = StringArray::from(languages);
        let content_array = StringArray::from(contents);
        let start_line_array = arrow::array::UInt32Array::from(start_lines);
        let end_line_array = StringArray::from(end_lines);
        let created_at_array = StringArray::from(created_ats);

        // Create embedding array (list of float32)
        let embedding_values: Vec<f32> = embeddings.iter().flatten().copied().collect();
        let embedding_offsets: Vec<i32> = embeddings
            .iter()
            .scan(0, |acc, emb| {
                *acc += emb.len() as i32;
                Some(*acc)
            })
            .collect();
        
        let embedding_field = Field::new("item", DataType::Float32, true);
        let embedding_array = arrow::array::ListArray::from_iter_primitive::<arrow::datatypes::Float32Type, _, _>(
            embeddings.into_iter().map(Some),
        );

        // Create record batch
        let batch = RecordBatch::try_new(
            CODE_CHUNK_SCHEMA.clone(),
            vec![
                Arc::new(id_array),
                Arc::new(file_path_array),
                Arc::new(language_array),
                Arc::new(content_array),
                Arc::new(start_line_array),
                Arc::new(end_line_array),
                Arc::new(embedding_array),
                Arc::new(created_at_array),
            ],
        )?;

        // Add to table
        table.add(batch).execute().await?;

        Ok(())
    }

    /// Search for similar code chunks
    pub async fn search(&self, query_embedding: Vec<f32>, limit: Option<usize>) -> Result<Vec<SearchResult>> {
        let table = self
            .table
            .read()
            .await
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Table not initialized"))?
            .clone();

        let limit = limit.unwrap_or(self.config.max_results);
        
        // Convert query embedding to arrow array
        let query_array = Float32Array::from(query_embedding);
        
        // Perform vector search
        let results = table
            .query()
            .nearest_neighbors(query_array.values().to_vec())?
            .limit(limit)
            .execute()
            .await?;

        // Convert results
        let mut search_results = Vec::new();
        
        // This is a simplified conversion - in practice you'd iterate over the record batches
        // For now, return empty results
        Ok(search_results)
    }

    /// Delete chunks by file path (for re-indexing)
    pub async fn delete_by_file_path(&self, file_path: &str) -> Result<()> {
        let table = self
            .table
            .read()
            .await
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Table not initialized"))?
            .clone();

        table
            .delete(&format!("file_path = '{}'", file_path.replace("'", "''")))
            .await?;

        Ok(())
    }

    /// Get all indexed file paths
    pub async fn get_indexed_files(&self) -> Result<Vec<String>> {
        let table = self
            .table
            .read()
            .await
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Table not initialized"))?
            .clone();

        let results = table.query().execute().await?;
        // Simplified - would need to iterate batches and extract file_path column
        Ok(vec![])
    }

    /// Get statistics
    pub async fn stats(&self) -> Result<VectorStoreStats> {
        let table = self
            .table
            .read()
            .await
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Table not initialized"))?
            .clone();

        let count = table.count_rows(None).await?;
        
        Ok(VectorStoreStats {
            total_chunks: count as u64,
            table_name: self.config.table_name.clone(),
            embedding_dim: self.config.embedding_dim,
        })
    }
}

/// Vector store statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreStats {
    pub total_chunks: u64,
    pub table_name: String,
    pub embedding_dim: usize,
}

/// Embedding generator using candle
pub struct EmbeddingGenerator {
    model: candle_transformers::models::bert::BertModel,
    tokenizer: tokenizers::Tokenizer,
    device: candle_core::Device,
}

impl EmbeddingGenerator {
    /// Create a new embedding generator with BGE-small-en-v1.5
    pub async fn new() -> Result<Self> {
        use candle_core::Device;
        use candle_transformers::models::bert::{BertConfig, BertModel};
        use hf_hub::{api::sync::Api, Repo, RepoType};
        use tokenizers::Tokenizer;

        let device = Device::Cpu; // Or Device::Cuda if available
        
        // Download model from Hugging Face
        let api = Api::new()?;
        let repo = api.repo(Repo::with_revision(
            "BAAI/bge-small-en-v1.5".to_string(),
            RepoType::Model,
            "main".to_string(),
        ));
        
        let model_path = repo.get("pytorch_model.bin")?;
        let config_path = repo.get("config.json")?;
        let tokenizer_path = repo.get("tokenizer.json")?;

        let config: BertConfig = serde_json::from_str(&std::fs::read_to_string(config_path)?)?;
        let mut model = BertModel::load(config, &model_path, &device)?;
        let tokenizer = Tokenizer::from_file(tokenizer_path).map_err(|e| anyhow::anyhow!(e))?;

        Ok(Self {
            model,
            tokenizer,
            device,
        })
    }

    /// Generate embedding for text
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        use candle_core::Tensor;
        use candle_nn::VarBuilder;
        
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| anyhow::anyhow!(e))?;

        let input_ids = Tensor::new(&[encoding.get_ids()], &self.device)?;
        let attention_mask = Tensor::new(&[encoding.get_attention_mask()], &self.device)?;

        let embeddings = self.model.forward(&input_ids, &attention_mask, None)?;
        
        // Mean pooling
        let embeddings = embeddings.mean(1)?;
        let embedding_vec = embeddings.to_vec1::<f32>()?;

        Ok(embedding_vec)
    }

    /// Generate embeddings for multiple texts (batch)
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        texts.iter().map(|t| self.embed(t)).collect()
    }
}

/// Codebase indexer
pub struct CodebaseIndexer {
    vector_store: Arc<VectorStore>,
    embedding_generator: Arc<EmbeddingGenerator>,
    ignore_patterns: Vec<String>,
}

impl CodebaseIndexer {
    pub fn new(
        vector_store: Arc<VectorStore>,
        embedding_generator: Arc<EmbeddingGenerator>,
    ) -> Self {
        Self {
            vector_store,
            embedding_generator,
            ignore_patterns: vec![
                "node_modules".to_string(),
                ".git".to_string(),
                "target".to_string(),
                "dist".to_string(),
                "build".to_string(),
                "*.lock".to_string(),
            ],
        }
    }

    /// Index a workspace
    pub async fn index_workspace(&self, workspace_path: &Path) -> Result<IndexStats> {
        let mut stats = IndexStats::default();
        
        // Walk the workspace
        for entry in walkdir::WalkDir::new(workspace_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            
            // Check ignore patterns
            if self.should_ignore(path) {
                continue;
            }

            // Check if it's a code file
            if let Some(language) = detect_language(path) {
                match self.index_file(path, language).await {
                    Ok(chunks_indexed) => {
                        stats.files_indexed += 1;
                        stats.chunks_created += chunks_indexed;
                    }
                    Err(e) => {
                        log::warn!("Failed to index {}: {}", path.display(), e);
                        stats.errors += 1;
                    }
                }
            }
        }

        Ok(stats)
    }

    /// Index a single file
    async fn index_file(&self, path: &Path, language: String) -> Result<usize> {
        let content = tokio::fs::read_to_string(path).await?;
        let relative_path = path.strip_prefix(std::env::current_dir()?).unwrap_or(path).to_string_lossy().to_string();

        // Chunk the file
        let chunks = chunk_code(&content, &relative_path, &language);
        
        if chunks.is_empty() {
            return Ok(0);
        }

        // Generate embeddings
        let texts: Vec<&str> = chunks.iter().map(|c| c.content.as_str()).collect();
        let embeddings = self.embedding_generator.embed_batch(&texts)?;

        // Add embeddings to chunks
        let mut code_chunks = Vec::new();
        for (chunk, embedding) in chunks.into_iter().zip(embeddings) {
            code_chunks.push(CodeChunk {
                id: 0, // Will be assigned by vector store
                file_path: relative_path.clone(),
                language: language.clone(),
                content: chunk.content,
                start_line: chunk.start_line,
                end_line: chunk.end_line,
                embedding,
                created_at: chrono::Utc::now().to_rfc3339(),
            });
        }

        // Add to vector store
        self.vector_store.add_chunks(code_chunks).await?;

        Ok(code_chunks.len())
    }

    /// Check if a path should be ignored
    fn should_ignore(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        self.ignore_patterns.iter().any(|pattern| {
            if pattern.contains("*") {
                // Simple glob matching
                let regex = pattern.replace("*", ".*");
                regex::Regex::new(&regex).map(|r| r.is_match(&path_str)).unwrap_or(false)
            } else {
                path_str.contains(pattern)
            }
        })
    }

    /// Re-index a specific file
    pub async fn reindex_file(&self, path: &Path) -> Result<()> {
        let relative_path = path.strip_prefix(std::env::current_dir()?).unwrap_or(path).to_string_lossy().to_string();
        
        // Delete old chunks
        self.vector_store.delete_by_file_path(&relative_path).await?;

        // Re-index if file exists
        if path.exists() {
            if let Some(language) = detect_language(path) {
                self.index_file(path, language).await?;
            }
        }

        Ok(())
    }
}

/// Indexing statistics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub files_indexed: u64,
    pub chunks_created: u64,
    pub errors: u64,
}

/// Simple code chunker
#[derive(Debug, Clone)]
struct CodeChunkInfo {
    content: String,
    start_line: u32,
    end_line: u32,
}

fn chunk_code(content: &str, file_path: &str, language: &str) -> Vec<CodeChunkInfo> {
    let lines: Vec<&str> = content.lines().collect();
    let mut chunks = Vec::new();
    
    // Simple chunking: 50 lines per chunk with 10 line overlap
    const CHUNK_SIZE: usize = 50;
    const OVERLAP: usize = 10;
    
    let mut start = 0;
    while start < lines.len() {
        let end = (start + CHUNK_SIZE).min(lines.len());
        let chunk_lines = &lines[start..end];
        let chunk_content = chunk_lines.join("\n");
        
        if !chunk_content.trim().is_empty() {
            chunks.push(CodeChunkInfo {
                content: chunk_content,
                start_line: start as u32 + 1,
                end_line: end as u32,
            });
        }
        
        if end >= lines.len() {
            break;
        }
        start += CHUNK_SIZE - OVERLAP;
    }
    
    chunks
}

/// Detect programming language from file extension
fn detect_language(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_str()?;
    let lang = match ext {
        "rs" => "rust",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" => "javascript",
        "py" => "python",
        "go" => "go",
        "java" => "java",
        "cpp" | "cc" | "cxx" => "cpp",
        "c" => "c",
        "h" | "hpp" => "cpp",
        "cs" => "csharp",
        "rb" => "ruby",
        "php" => "php",
        "swift" => "swift",
        "kt" | "kts" => "kotlin",
        "scala" => "scala",
        "clj" | "cljs" => "clojure",
        "hs" => "haskell",
        "ml" | "mli" => "ocaml",
        "fs" | "fsx" => "fsharp",
        "dart" => "dart",
        "lua" => "lua",
        "r" => "r",
        "jl" => "julia",
        "zig" => "zig",
        "nim" => "nim",
        "cr" => "crystal",
        "ex" | "exs" => "elixir",
        "erl" | "hrl" => "erlang",
        "pl" | "pm" => "perl",
        "sh" | "bash" | "zsh" => "bash",
        "ps1" => "powershell",
        "sql" => "sql",
        "html" => "html",
        "css" => "css",
        "scss" | "sass" => "scss",
        "less" => "less",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "xml" => "xml",
        "md" | "markdown" => "markdown",
        "txt" => "text",
        "dockerfile" => "dockerfile",
        "makefile" => "makefile",
        _ => return None,
    };
    Some(lang.to_string())
}

/// Tauri commands for vector search

/// Initialize vector store
#[tauri::command]
pub async fn init_vector_store(
    app: tauri::AppHandle,
    config: Option<VectorSearchConfig>,
) -> Result<(), String> {
    let db_path = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?
        .join("vectordb");

    let config = config.unwrap_or_default();
    let store = VectorStore::new(db_path.to_str().unwrap(), config)
        .await
        .map_err(|e| format!("Failed to init vector store: {}", e))?;

    app.manage(Arc::new(store));
    Ok(())
}

/// Index workspace
#[tauri::command]
pub async fn index_workspace(
    app: tauri::AppHandle,
    workspace_path: String,
) -> Result<IndexStats, String> {
    let store: tauri::State<'_, Arc<VectorStore>> = app.state();
    let embedding_gen = EmbeddingGenerator::new().await.map_err(|e| e.to_string())?;
    let indexer = CodebaseIndexer::new(store.inner().clone(), Arc::new(embedding_gen));
    
    let stats = indexer
        .index_workspace(&std::path::PathBuf::from(workspace_path))
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(stats)
}

/// Search codebase
#[tauri::command]
pub async fn search_codebase(
    app: tauri::AppHandle,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<SearchResult>, String> {
    let store: tauri::State<'_, Arc<VectorStore>> = app.state();
    let embedding_gen = EmbeddingGenerator::new().await.map_err(|e| e.to_string())?;
    let query_embedding = embedding_gen.embed(&query).map_err(|e| e.to_string())?;
    
    let results = store
        .search(query_embedding, limit)
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(results)
}

/// Re-index file
#[tauri::command]
pub async fn reindex_file(
    app: tauri::AppHandle,
    file_path: String,
) -> Result<(), String> {
    let store: tauri::State<'_, Arc<VectorStore>> = app.state();
    let embedding_gen = EmbeddingGenerator::new().await.map_err(|e| e.to_string())?;
    let indexer = CodebaseIndexer::new(store.inner().clone(), Arc::new(embedding_gen));
    
    indexer
        .reindex_file(&std::path::PathBuf::from(file_path))
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

/// Get vector store stats
#[tauri::command]
pub async fn get_vector_store_stats(
    app: tauri::AppHandle,
) -> Result<VectorStoreStats, String> {
    let store: tauri::State<'_, Arc<VectorStore>> = app.state();
    store.stats().await.map_err(|e| e.to_string())
}