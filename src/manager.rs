use rusqlite::{params, OptionalExtension};

pub struct MemoryManager {
    conn: std::sync::Mutex<rusqlite::Connection>,
    embedder: crate::embed::Embedder,
}

impl MemoryManager {
    pub fn new(path: &str) -> anyhow::Result<Self> {
        let conn = crate::db::init_db(path)?;
        let embedder = crate::embed::Embedder::new()?;
        Ok(Self {
            conn: std::sync::Mutex::new(conn),
            embedder,
        })
    }

    pub fn get_or_create_wing(&self, name: &str) -> anyhow::Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO wings (name, type) VALUES (?1, 'general')",
            params![name],
        )?;
        let id: i64 = conn.query_row(
            "SELECT id FROM wings WHERE name = ?1",
            params![name],
            |row| row.get(0),
        )?;
        Ok(id)
    }

    pub fn get_or_create_room(&self, wing_id: i64, name: &str) -> anyhow::Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO rooms (wing_id, name) VALUES (?1, ?2)",
            params![wing_id, name],
        )?;
        let id: i64 = conn.query_row(
            "SELECT id FROM rooms WHERE wing_id = ?1 AND name = ?2",
            params![wing_id, name],
            |row| row.get(0),
        )?;
        Ok(id)
    }

    pub fn add_memory(&self, wing: &str, room: &str, text: &str) -> anyhow::Result<()> {
        let wing_id = self.get_or_create_wing(wing)?;
        let room_id = self.get_or_create_room(wing_id, room)?;
        let embedding = self.embedder.embed(text)?;

        let conn = self.conn.lock().unwrap();
        
        // 1. Deduplication Engine (>95% cosine similarity)
        let mut stmt = conn.prepare("SELECT id, embedding FROM memories WHERE room_id = ?1")?;
        let rows = stmt.query_map(params![room_id], |row| {
            let id: i64 = row.get(0)?;
            let embed_bytes: Vec<u8> = row.get(1)?;
            let mut vec_f32 = Vec::new();
            for chunk in embed_bytes.chunks(4) {
                if chunk.len() == 4 {
                    vec_f32.push(f32::from_le_bytes(chunk.try_into().unwrap()));
                }
            }
            Ok((id, vec_f32))
        })?;

        let mut duplicate_id = None;
        for r in rows {
            if let Ok((id, existing_emb)) = r {
                let sim = cosine_similarity(&embedding, &existing_emb);
                if sim > 0.95 {
                    duplicate_id = Some(id);
                    break;
                }
            }
        }

        let bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();

        if let Some(id) = duplicate_id {
            conn.execute(
                "UPDATE memories SET content = ?1, embedding = ?2, created_at = CURRENT_TIMESTAMP WHERE id = ?3",
                params![text, bytes, id],
            )?;
            return Ok(());
        }

        // 2. Normal Insertion
        conn.execute(
            "INSERT INTO memories (room_id, content, embedding) VALUES (?1, ?2, ?3)",
            params![room_id, text, bytes],
        )?;
        Ok(())
    }

    pub fn add_memory_benchmark(&self, wing: &str, room: &str, text: &str) -> anyhow::Result<()> {
        let wing_id = self.get_or_create_wing(wing)?;
        let room_id = self.get_or_create_room(wing_id, room)?;
        let embedding = self.embedder.embed(text)?;

        let bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO memories (room_id, content, embedding) VALUES (?1, ?2, ?3)",
            params![room_id, text, bytes],
        )?;
        Ok(())
    }

    pub fn update_memory(&self, id: i64, text: &str) -> anyhow::Result<()> {
        let embedding = self.embedder.embed(text)?;
        let bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();

        let conn = self.conn.lock().unwrap();
        let changed = conn.execute(
            "UPDATE memories SET content = ?1, embedding = ?2, created_at = CURRENT_TIMESTAMP WHERE id = ?3",
            params![text, bytes, id],
        )?;
        if changed == 0 {
            anyhow::bail!("Memory ID not found");
        }
        Ok(())
    }

    pub fn delete_memory(&self, id: i64) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        let changed = conn.execute("DELETE FROM memories WHERE id = ?1", params![id])?;
        if changed == 0 {
            anyhow::bail!("Memory ID not found");
        }
        Ok(())
    }

    pub fn search_memory(&self, wing: Option<&str>, room: Option<&str>, query: &str) -> anyhow::Result<Vec<String>> {
        let query_embedding = self.embedder.embed(query)?;

        let conn = self.conn.lock().unwrap();

        let mut room_filter = String::new();
        if let (Some(w), Some(r)) = (wing, room) {
            let room_id_opt: Option<i64> = conn.query_row(
                "SELECT rooms.id FROM rooms JOIN wings ON wings.id = rooms.wing_id WHERE wings.name = ?1 AND rooms.name = ?2",
                params![w, r],
                |row| row.get(0),
            ).optional()?;

            if let Some(id) = room_id_opt {
                room_filter = format!("WHERE room_id = {}", id);
            } else {
                return Ok(vec![]);
            }
        }

        // 1. FTS5 Keyword Overlap scoring
        let query_keywords = extract_keywords(query);
        let mut keyword_hits: std::collections::HashMap<i64, f32> = std::collections::HashMap::new();
        
        if !query_keywords.is_empty() {
            // Escape each keyword in double quotes to prevent FTS5 syntax errors with reserved keywords
            let fts_terms: Vec<String> = query_keywords.iter().map(|k| format!("\"{}\"", k)).collect();
            let fts_match = fts_terms.join(" OR ");
            
            let fts_sql = "SELECT rowid, bm25(memories_fts) FROM memories_fts WHERE memories_fts MATCH ?1";
            if let Ok(mut stmt) = conn.prepare(fts_sql) {
                if let Ok(rows) = stmt.query_map(params![fts_match], |row| {
                    let id: i64 = row.get(0)?;
                    let score: f64 = row.get(1)?;
                    Ok((id, (-score) as f32)) // SQLite BM25 is negatively scored natively
                }) {
                    for r in rows {
                        if let Ok((id, score)) = r {
                            let normalized_overlap = (0.2 + (score / 3.0)).min(1.0);
                            keyword_hits.insert(id, normalized_overlap);
                        }
                    }
                }
            }
        }

        // 2. Fetch Vectors & Merge Semantic Overlap
        let sql = format!("SELECT id, content, embedding, created_at FROM memories {}", room_filter);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let content: String = row.get(1)?;
            let embed_bytes: Vec<u8> = row.get(2)?;
            let created_at: String = row.get(3)?;
            let mut vec_f32 = Vec::new();
            for chunk in embed_bytes.chunks(4) {
                if chunk.len() == 4 {
                    vec_f32.push(f32::from_le_bytes(chunk.try_into().unwrap()));
                }
            }
            Ok((id, content, vec_f32, created_at))
        })?;

        let mut results = Vec::new();
        for r in rows {
            let (id, content, embedding, created_at) = r?;
            let mut similarity = cosine_similarity(&query_embedding, &embedding);
            
            // Apply Hybrid Scoring: Use FTS5 hit instead of slow string loop check
            let overlap = keyword_hits.get(&id).unwrap_or(&0.0);
            if *overlap > 0.0 {
                let dist = 1.0 - similarity;
                let fused_dist = dist * (1.0 - 0.30 * overlap);
                similarity = 1.0 - fused_dist;
            }

            let formatted_content = format!("[ID={}] [TIME={}]: {}", id, created_at, content);
            results.push((formatted_content, similarity));
        }

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let top_contents: Vec<String> = results.into_iter().take(5).map(|(c, _)| c).collect();
        Ok(top_contents)
    }

    pub fn list_wings(&self) -> anyhow::Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT name FROM wings")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        let mut res = vec![];
        for r in rows { res.push(r?); }
        Ok(res)
    }

    pub fn list_rooms(&self, wing: &str) -> anyhow::Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT rooms.name FROM rooms JOIN wings ON wings.id = rooms.wing_id WHERE wings.name = ?1")?;
        let rows = stmt.query_map(params![wing], |row| row.get(0))?;
        let mut res = vec![];
        for r in rows { res.push(r?); }
        Ok(res)
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let mut dot_product = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;
    for i in 0..a.len() {
        dot_product += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot_product / (norm_a.sqrt() * norm_b.sqrt())
}

fn extract_keywords(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() > 3) // Filters out short/stop words trivially (the, a, is, of, etc)
        .map(|s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_manager() -> MemoryManager {
        MemoryManager::new(":memory:").unwrap()
    }

    #[test]
    fn test_deduplication() {
        let mgr = setup_manager();
        mgr.add_memory("wing1", "room1", "The backend uses warp in rust.").unwrap();
        mgr.add_memory("wing1", "room1", "The backend uses warp in rust.").unwrap(); // Duplicate
        
        let conn = mgr.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM memories", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1, "Duplicate memory should not increase total count");
    }

    #[test]
    fn test_global_routing() {
        let mgr = setup_manager();
        mgr.add_memory("wingA", "roomA", "Apples are red.").unwrap();
        mgr.add_memory("wingB", "roomB", "Bananas are yellow.").unwrap();

        // Scope restrict query
        let res1 = mgr.search_memory(Some("wingA"), Some("roomA"), "Apples").unwrap();
        assert_eq!(res1.len(), 1);

        // Global query
        let res2 = mgr.search_memory(None, None, "red yellow").unwrap();
        assert_eq!(res2.len(), 2, "Global routing should locate memories from both wings");
    }

    #[test]
    fn test_fts5_indexing() {
        let mgr = setup_manager();
        mgr.add_memory("test", "test", "UniqueKeywordAlpha match setup.").unwrap();
        
        // Assert FTS table synced automatically
        let conn = mgr.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM memories_fts WHERE memories_fts MATCH 'UniqueKeywordAlpha'", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1, "FTS5 trigger failed to sync memory insertion");
    }
}
