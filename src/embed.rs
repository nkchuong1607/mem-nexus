use fastembed::{TextEmbedding, InitOptions};
use std::sync::Mutex;
use std::path::PathBuf;

pub struct Embedder {
    model: Mutex<TextEmbedding>,
}

impl Embedder {
    pub fn new() -> anyhow::Result<Self> {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let cache_dir = PathBuf::from(format!("{}/.mem-nexus/models", home));
        std::fs::create_dir_all(&cache_dir).unwrap_or_default();
        
        let mut options: InitOptions = Default::default();
        options.cache_dir = cache_dir;

        let model = TextEmbedding::try_new(options)?;
        Ok(Self { model: Mutex::new(model) })
    }

    pub fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let mut model = self.model.lock().unwrap();
        let mut embeddings = model.embed(vec![text], None)?;
        Ok(embeddings.pop().unwrap())
    }
}
