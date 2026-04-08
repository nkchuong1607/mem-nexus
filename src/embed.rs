use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
use std::sync::Mutex;

pub struct Embedder {
    model: Mutex<TextEmbedding>,
}

impl Embedder {
    pub fn new() -> anyhow::Result<Self> {
        let model = TextEmbedding::try_new(Default::default())?;
        Ok(Self { model: Mutex::new(model) })
    }

    pub fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let mut model = self.model.lock().unwrap();
        let mut embeddings = model.embed(vec![text], None)?;
        Ok(embeddings.pop().unwrap())
    }
}
