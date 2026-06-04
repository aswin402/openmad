#![allow(dead_code)]
use fastembed::{TextEmbedding, InitOptions};
use std::sync::{Arc, RwLock};
use tracing::{info, error};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone)]
pub struct MemoryEntry {
    pub text: String,
    pub embedding: Vec<f32>,
    pub metadata: String, // e.g. task_id, agent_name, step
}

pub struct MemoryEngine {
    model: Option<TextEmbedding>,
    long_term_memories: Arc<RwLock<Vec<MemoryEntry>>>,
    short_term_history: Arc<RwLock<Vec<String>>>,
}

impl MemoryEngine {
    pub fn new() -> Self {
        // Initialize model. If it fails or takes too long, we will degrade gracefully.
        let model = match TextEmbedding::try_new(InitOptions::default()) {
            Ok(m) => Some(m),
            Err(e) => {
                error!("Failed to initialize fastembed model: {}. Running with offline/empty embeddings.", e);
                None
            }
        };

        Self {
            model,
            long_term_memories: Arc::new(RwLock::new(Vec::new())),
            short_term_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Stores a text entry in short-term history and also generates an embedding for long-term semantic search.
    pub fn store_memory(&self, text: &str, metadata: &str) {
        // 1. Store in short term
        {
            let mut history = self.short_term_history.write().unwrap();
            history.push(format!("[{}] {}", metadata, text));
        }

        // 2. Generate embedding and store in long term if model is available
        if let Some(ref model) = self.model {
            match model.embed(vec![text], None) {
                Ok(embeddings) => {
                    if !embeddings.is_empty() {
                        let mut long_term = self.long_term_memories.write().unwrap();
                        long_term.push(MemoryEntry {
                            text: text.to_string(),
                            embedding: embeddings[0].clone(),
                            metadata: metadata.to_string(),
                        });
                        info!("Stored semantic memory: '{}'", text);
                    }
                }
                Err(e) => {
                    error!("Error generating embedding: {}", e);
                }
            }
        } else {
            // Offline fallback
            let mut long_term = self.long_term_memories.write().unwrap();
            long_term.push(MemoryEntry {
                text: text.to_string(),
                embedding: vec![],
                metadata: metadata.to_string(),
            });
        }
    }

    /// Performs semantic search over stored long-term memories using cosine similarity.
    pub fn query_semantic(&self, query: &str, limit: usize) -> Vec<(String, f32)> {
        let query_embedding = if let Some(ref model) = self.model {
            match model.embed(vec![query], None) {
                Ok(embeddings) => {
                    if embeddings.is_empty() {
                        return vec![];
                    }
                    embeddings[0].clone()
                }
                Err(e) => {
                    error!("Error generating embedding for query: {}", e);
                    return vec![];
                }
            }
        } else {
            return vec![];
        };

        let memories = self.long_term_memories.read().unwrap();
        let mut results = Vec::new();

        for entry in memories.iter() {
            if entry.embedding.is_empty() || query_embedding.is_empty() {
                continue;
            }
            let score = cosine_similarity(&query_embedding, &entry.embedding);
            results.push((format!("[{}] {}", entry.metadata, entry.text), score));
        }

        // Sort by similarity score descending
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
    }

    pub fn get_recent_history(&self, limit: usize) -> Vec<String> {
        let history = self.short_term_history.read().unwrap();
        let start = history.len().saturating_sub(limit);
        history[start..].to_vec()
    }
}

/// Helper function to compute cosine similarity between two float vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot_product = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for i in 0..a.len() {
        dot_product += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a.sqrt() * norm_b.sqrt())
    }
}

pub struct SharedAgentMemory {
    artifacts: DashMap<String, String>,
}

impl SharedAgentMemory {
    pub fn new() -> Self {
        Self {
            artifacts: DashMap::new(),
        }
    }

    pub fn publish(&self, key: &str, content: &str) {
        self.artifacts.insert(key.to_string(), content.to_string());
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.artifacts.get(key).map(|v| v.clone())
    }

    pub fn list_keys(&self) -> Vec<String> {
        self.artifacts.iter().map(|kv| kv.key().clone()).collect()
    }
}

// === LETTA-STYLE CORE MEMORY IMPLEMENTATION ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreMemory {
    pub blocks: std::collections::HashMap<String, String>,
}

impl CoreMemory {
    pub fn new(persona_desc: &str, human_desc: &str) -> Self {
        let mut blocks = std::collections::HashMap::new();
        blocks.insert("persona".to_string(), persona_desc.to_string());
        blocks.insert("human".to_string(), human_desc.to_string());
        Self { blocks }
    }

    pub fn get_block(&self, label: &str) -> Option<&String> {
        self.blocks.get(label)
    }

    pub fn set_block(&mut self, label: &str, value: &str) {
        self.blocks.insert(label.to_string(), value.to_string());
    }

    pub fn to_xml(&self) -> String {
        let mut xml = String::new();
        xml.push_str("<core_memory>\n");
        for (label, val) in &self.blocks {
            xml.push_str(&format!("  <{}>\n    {}\n  </{}>\n", label, val, label));
        }
        xml.push_str("</core_memory>");
        xml
    }
}

pub struct AgentMemoryStore {
    file_path: String,
}

impl AgentMemoryStore {
    pub fn new(path: &str) -> Self {
        Self {
            file_path: path.to_string(),
        }
    }

    /// Loads the core memory for a given agent name.
    pub fn load_memory(&self, agent_name: &str, default_persona: &str) -> CoreMemory {
        if std::path::Path::new(&self.file_path).exists() {
            if let Ok(content) = std::fs::read_to_string(&self.file_path) {
                if let Ok(mut store) = serde_json::from_str::<std::collections::HashMap<String, CoreMemory>>(&content) {
                    if let Some(mem) = store.remove(agent_name) {
                        info!("Loaded persistent Letta-style memory for agent '{}'", agent_name);
                        return mem;
                    }
                }
            }
        }
        
        info!("No persistent memory found for agent '{}'. Initializing with default.", agent_name);
        CoreMemory::new(default_persona, "The user wants to complete the orchestrator goals. Prefers clean code and clear logs.")
    }

    /// Saves the core memory for a given agent name.
    pub fn save_memory(&self, agent_name: &str, memory: &CoreMemory) -> anyhow::Result<()> {
        let mut store = if std::path::Path::new(&self.file_path).exists() {
            if let Ok(content) = std::fs::read_to_string(&self.file_path) {
                serde_json::from_str::<std::collections::HashMap<String, CoreMemory>>(&content)
                    .unwrap_or_default()
            } else {
                std::collections::HashMap::new()
            }
        } else {
            std::collections::HashMap::new()
        };

        store.insert(agent_name.to_string(), memory.clone());
        let serialized = serde_json::to_string_pretty(&store)?;
        std::fs::write(&self.file_path, serialized)?;
        info!("Saved persistent Letta-style memory for agent '{}'", agent_name);
        Ok(())
    }
}

