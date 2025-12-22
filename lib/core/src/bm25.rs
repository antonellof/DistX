// Simple BM25 implementation for text search
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BM25Index {
    // term -> (doc_id -> term_frequency)
    inverted_index: HashMap<String, HashMap<String, u32>>,
    // doc_id -> document length
    doc_lengths: HashMap<String, u32>,
    // term -> document frequency
    term_dfs: HashMap<String, u32>,
    total_docs: u64,
    k1: f32, // term frequency saturation parameter
    b: f32,  // length normalization parameter
}

impl BM25Index {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            inverted_index: HashMap::new(),
            doc_lengths: HashMap::new(),
            term_dfs: HashMap::new(),
            total_docs: 0,
            k1: 1.5,
            b: 0.75,
        }
    }

    /// Tokenize text for BM25 indexing
    /// Uses lowercase normalization and removes punctuation
    #[inline]
    pub fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
            .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
            .filter(|s| !s.is_empty() && s.len() > 1)  // Filter single chars
            .collect()
    }

    pub fn insert_doc(&mut self, doc_id: &str, text: &str) {
        // Remove old document if exists
        self.delete_doc(doc_id);

        let tokens = Self::tokenize(text);
        let doc_len = tokens.len() as u32;

        // Count term frequencies
        let mut term_freqs: HashMap<String, u32> = HashMap::new();
        for token in &tokens {
            *term_freqs.entry(token.clone()).or_insert(0) += 1;
        }

        // Update inverted index
        for (term, tf) in &term_freqs {
            self.inverted_index
                .entry(term.clone())
                .or_default()
                .insert(doc_id.to_string(), *tf);
        }

        // Update document length
        self.doc_lengths.insert(doc_id.to_string(), doc_len);

        // Update term document frequencies
        for term in term_freqs.keys() {
            *self.term_dfs.entry(term.clone()).or_insert(0) += 1;
        }

        self.total_docs += 1;
    }

    pub fn delete_doc(&mut self, doc_id: &str) {
        if self.doc_lengths.remove(doc_id).is_some() {
            // Remove from inverted index and update DFs
            let mut terms_to_update = Vec::new();
            for (term, docs) in &mut self.inverted_index {
                if docs.remove(doc_id).is_some() {
                    terms_to_update.push(term.clone());
                }
            }

            // Update document frequencies
            for term in terms_to_update {
                if let Some(df) = self.term_dfs.get_mut(&term) {
                    *df = df.saturating_sub(1);
                }
            }

            self.total_docs = self.total_docs.saturating_sub(1);
        }
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<(String, f32)> {
        if self.total_docs == 0 {
            return Vec::new();
        }

        let query_terms = Self::tokenize(query);
        if query_terms.is_empty() {
            return Vec::new();
        }

        // Calculate average document length
        let avgdl = if self.total_docs > 0 {
            self.doc_lengths.values().sum::<u32>() as f32 / self.total_docs as f32
        } else {
            0.0
        };

        // Score each document
        let mut doc_scores: HashMap<String, f32> = HashMap::new();

        for term in &query_terms {
            if let Some(docs) = self.inverted_index.get(term) {
                let df = self.term_dfs.get(term).copied().unwrap_or(0) as f32;
                let idf = if df > 0.0 {
                    ((self.total_docs as f32 - df + 0.5) / (df + 0.5)).ln().max(0.0)
                } else {
                    0.0
                };

                for (doc_id, &tf) in docs {
                    if let Some(&doc_len) = self.doc_lengths.get(doc_id) {
                        let score = self.calculate_bm25_score(tf, doc_len, df as u32, self.total_docs, avgdl, idf);
                        *doc_scores.entry(doc_id.clone()).or_insert(0.0) += score;
                    }
                }
            }
        }

        // Sort by score and return top N
        let mut results: Vec<(String, f32)> = doc_scores.into_iter().collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
    }

    fn calculate_bm25_score(
        &self,
        tf: u32,
        doc_len: u32,
        _df: u32,
        _total_docs: u64,
        avgdl: f32,
        idf: f32,
    ) -> f32 {
        let tf_f32 = tf as f32;
        let doc_len_f32 = doc_len as f32;

        // BM25 formula: idf * (tf * (k1 + 1)) / (tf + k1 * (1 - b + b * (doc_len / avgdl)))
        let numerator = tf_f32 * (self.k1 + 1.0);
        let denominator = tf_f32 + self.k1 * (1.0 - self.b + self.b * (doc_len_f32 / avgdl));
        
        idf * (numerator / denominator)
    }

    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.doc_lengths.len()
    }

    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.doc_lengths.is_empty()
    }
}

impl Default for BM25Index {
    fn default() -> Self {
        Self::new()
    }
}

