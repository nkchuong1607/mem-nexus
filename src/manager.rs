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

        let bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO memories (room_id, content, embedding) VALUES (?1, ?2, ?3)",
            params![room_id, text, bytes],
        )?;
        Ok(())
    }

    pub fn search_memory(&self, wing: &str, room: &str, query: &str) -> anyhow::Result<Vec<String>> {
        let query_embedding = self.embedder.embed(query)?;

        let conn = self.conn.lock().unwrap();
        let room_id_opt: Option<i64> = conn.query_row(
            "SELECT rooms.id FROM rooms JOIN wings ON wings.id = rooms.wing_id WHERE wings.name = ?1 AND rooms.name = ?2",
            params![wing, room],
            |row| row.get(0),
        ).optional()?;

        let room_id = match room_id_opt {
            Some(id) => id,
            None => return Ok(vec![]),
        };

        let mut stmt = conn.prepare("SELECT content, embedding FROM memories WHERE room_id = ?1")?;
        let rows = stmt.query_map(params![room_id], |row| {
            let content: String = row.get(0)?;
            let embed_bytes: Vec<u8> = row.get(1)?;
            let mut vec_f32 = Vec::new();
            for chunk in embed_bytes.chunks(4) {
                if chunk.len() == 4 {
                    vec_f32.push(f32::from_le_bytes(chunk.try_into().unwrap()));
                }
            }
            Ok((content, vec_f32))
        })?;

        let query_keywords = extract_keywords(query);

        let mut results = Vec::new();
        for r in rows {
            let (content, embedding) = r?;
            let mut similarity = cosine_similarity(&query_embedding, &embedding);
            
            // Apply Hybrid Scoring: MemPalace standard overlap heuristic
            let overlap = keyword_overlap(&query_keywords, &content);
            if overlap > 0.0 {
                // Convert similarity to distance (0.0 to 2.0 where 0 is perfect)
                let dist = 1.0 - similarity;
                
                // standard hybrid weight = 0.30 
                let fused_dist = dist * (1.0 - 0.30 * overlap);
                
                // convert back to similarity 
                similarity = 1.0 - fused_dist;
            }

            results.push((content, similarity));
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

fn keyword_overlap(keywords: &[String], doc: &str) -> f32 {
    if keywords.is_empty() {
        return 0.0;
    }
    let doc_lower = doc.to_lowercase();
    let mut matches = 0;
    
    // Convert to unique set to avoid over-counting duplicate words in the query
    let mut unique_keywords = keywords.to_vec();
    unique_keywords.sort();
    unique_keywords.dedup();
    
    for kw in &unique_keywords {
        if doc_lower.contains(kw) {
            matches += 1;
        }
    }
    matches as f32 / unique_keywords.len() as f32
}
