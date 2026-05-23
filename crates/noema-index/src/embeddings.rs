use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use noema_core::error::{NoemaError, Result};
use ort::session::Session;
use ort::value::TensorRef;
use tokenizers::Tokenizer;
use tracing::{debug, info};

pub struct EmbeddingEngine {
    session: std::sync::Mutex<Session>,
    tokenizer: Tokenizer,
    dimension: usize,
    loaded: AtomicBool,
}

impl EmbeddingEngine {
    pub fn load(model_dir: &Path) -> Result<Self> {
        let model_path = model_dir.join("model.onnx");
        let tokenizer_path = model_dir.join("tokenizer.json");

        if !model_path.exists() {
            return Err(NoemaError::NotFound { path: model_path });
        }
        if !tokenizer_path.exists() {
            return Err(NoemaError::NotFound { path: tokenizer_path });
        }

        let session = Session::builder()
            .map_err(|e| NoemaError::Ai { detail: format!("ORT session builder: {}", e) })?
            .with_intra_threads(2)
            .map_err(|e| NoemaError::Ai { detail: format!("ORT threads: {}", e) })?
            .commit_from_file(&model_path)
            .map_err(|e| NoemaError::Ai { detail: format!("ORT model load: {}", e) })?;

        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| NoemaError::Ai { detail: format!("Tokenizer load: {}", e) })?;

        let dimension = 384; // BGE-small

        info!(dim = dimension, "Embedding engine loaded");

        Ok(Self {
            session: std::sync::Mutex::new(session),
            tokenizer,
            dimension,
            loaded: AtomicBool::new(true),
        })
    }

    pub fn dimension(&self) -> usize {
        self.dimension
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded.load(Ordering::Relaxed)
    }

    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let encodings = self.tokenizer
            .encode_batch(texts.to_vec(), true)
            .map_err(|e| NoemaError::Ai { detail: format!("Tokenize: {}", e) })?;

        let batch_size = encodings.len();
        let max_len = encodings.iter().map(|e| e.get_ids().len()).max().unwrap_or(0).min(512);

        let mut input_ids = vec![0i64; batch_size * max_len];
        let mut attention_mask = vec![0i64; batch_size * max_len];
        let mut token_type_ids = vec![0i64; batch_size * max_len];

        for (i, encoding) in encodings.iter().enumerate() {
            let ids = encoding.get_ids();
            let mask = encoding.get_attention_mask();
            let len = ids.len().min(max_len);
            for j in 0..len {
                input_ids[i * max_len + j] = ids[j] as i64;
                attention_mask[i * max_len + j] = mask[j] as i64;
            }
        }

        let shape: [usize; 2] = [batch_size, max_len];

        let ids_tensor = TensorRef::from_array_view((shape, &input_ids[..]))
            .map_err(|e| NoemaError::Ai { detail: format!("Tensor ids: {}", e) })?;
        let mask_tensor = TensorRef::from_array_view((shape, &attention_mask[..]))
            .map_err(|e| NoemaError::Ai { detail: format!("Tensor mask: {}", e) })?;
        let type_tensor = TensorRef::from_array_view((shape, &token_type_ids[..]))
            .map_err(|e| NoemaError::Ai { detail: format!("Tensor types: {}", e) })?;

        let mut session = self.session.lock()
            .map_err(|_| NoemaError::Ai { detail: "Session lock poisoned".into() })?;

        let outputs = session.run(ort::inputs![ids_tensor, mask_tensor, type_tensor])
            .map_err(|e| NoemaError::Ai { detail: format!("ORT run: {}", e) })?;

        let output = outputs.values().next()
            .ok_or_else(|| NoemaError::Ai { detail: "No output tensor".into() })?;

        let (out_shape, out_data) = output.try_extract_tensor::<f32>()
            .map_err(|e| NoemaError::Ai { detail: format!("Extract: {}", e) })?;

        let dims: Vec<i64> = out_shape.iter().copied().collect();

        let results = if dims.len() == 3 {
            // [batch, seq_len, dim] — mean pool with attention mask
            let seq_len = dims[1] as usize;
            let dim = dims[2] as usize;
            (0..batch_size)
                .map(|i| {
                    let mask_len = encodings[i].get_attention_mask().iter()
                        .filter(|&&m| m == 1).count().min(max_len);
                    let offset = i * seq_len * dim;
                    let pooled: Vec<f32> = (0..dim)
                        .map(|d| {
                            let sum: f32 = (0..mask_len).map(|s| out_data[offset + s * dim + d]).sum();
                            sum / mask_len as f32
                        })
                        .collect();
                    normalize(&pooled)
                })
                .collect()
        } else if dims.len() == 2 {
            // [batch, dim] — already pooled
            let dim = dims[1] as usize;
            (0..batch_size)
                .map(|i| {
                    let offset = i * dim;
                    normalize(&out_data[offset..offset + dim])
                })
                .collect()
        } else {
            return Err(NoemaError::Ai { detail: format!("Unexpected shape: {:?}", dims) });
        };

        debug!(batch = batch_size, "Embedded batch");
        Ok(results)
    }

    pub fn embed_query(&self, query: &str) -> Result<Vec<f32>> {
        let results = self.embed_batch(&[query])?;
        results.into_iter().next().ok_or(NoemaError::Ai { detail: "Empty result".into() })
    }
}

fn normalize(v: &[f32]) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm < 1e-12 {
        return v.to_vec();
    }
    v.iter().map(|x| x / norm).collect()
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

pub fn embedding_to_bytes(embedding: &[f32]) -> Vec<u8> {
    embedding.iter().flat_map(|f| f.to_le_bytes()).collect()
}

pub fn bytes_to_embedding(bytes: &[u8]) -> Vec<f32> {
    bytes.chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}
