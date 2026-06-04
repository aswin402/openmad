#![allow(dead_code)]
use fastembed::{TextEmbedding, InitOptions};
use std::sync::{Arc, RwLock};
use tracing::{info, error};
use dashmap::DashMap;

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
