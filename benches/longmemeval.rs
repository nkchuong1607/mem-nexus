use std::path::Path;
use std::time::Instant;
use reqwest;
use serde::Deserialize;
use std::fs;

// Assuming lib exposes `MemoryManager` or we access it identically.
// Since `mem-nexus` is a binary crate with `main.rs`, we might need to make it a library
// or just include the modules here directly. Wait, better to put this in `src/bin/` and
// expose modules in `src/lib.rs`.

#[path = "../src/db.rs"]
mod db;
#[path = "../src/embed.rs"]
mod embed;
#[path = "../src/manager.rs"]
mod manager;

use manager::MemoryManager;

#[derive(Debug, Deserialize)]
struct Turn {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct LongMemEvalEntry {
    question_id: serde_json::Value,
    question_type: String,
    question: String,
    answer_session_ids: Vec<serde_json::Value>,
    haystack_sessions: Vec<Vec<Turn>>,
    haystack_session_ids: Vec<serde_json::Value>,
    #[serde(default)]
    haystack_dates: Vec<String>,
    #[serde(default)]
    question_date: String,
    #[serde(default)]
    answer: serde_json::Value,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let data_path = "/tmp/longmemeval_s_cleaned.json";
    
    // 1. Download dataset if not exists
    if !Path::new(data_path).exists() {
        println!("Downloading LongMemEval dataset...");
        let url = "https://huggingface.co/datasets/xiaowu0162/longmemeval-cleaned/resolve/main/longmemeval_s_cleaned.json";
        let responses = reqwest::get(url).await?.text().await?;
        fs::write(data_path, responses)?;
    }

    println!("Loading dataset from {}...", data_path);
    let data_str = fs::read_to_string(data_path)?;
    let entries: Vec<LongMemEvalEntry> = serde_json::from_str(&data_str)?;
    
    println!("Loaded {} questions.", entries.len());

    // 2. Initialize in-memory database
    let manager = MemoryManager::new(":memory:")?;

    let mut hit_at_5 = 0;
    
    let start_time = Instant::now();

    for (i, entry) in entries.iter().enumerate() {
        // Reset the memory manager for each question to isolate context 
        // (LongMemEval retrieves over the specific haystack of that entry)
        let manager = MemoryManager::new(":memory:")?;
        
fn val_to_str(v: &serde_json::Value) -> String {
    if let Some(s) = v.as_str() {
        s.to_string()
    } else {
        v.to_string()
    }
}

        let wing_name = "test_wing";
        
        let target_answers: Vec<String> = entry.answer_session_ids.iter().map(|v| val_to_str(v)).collect();

        // Index haystack
        for (session_idx, session_turns) in entry.haystack_sessions.iter().enumerate() {
            let sess_id = val_to_str(&entry.haystack_session_ids[session_idx]);
            
            // Only index user turns to strictly match MemPalace Raw Verbatim baseline
            let mut user_text = String::new();
            for turn in session_turns {
                if turn.role == "user" {
                    if !user_text.is_empty() {
                        user_text.push('\n');
                    }
                    user_text.push_str(&turn.content);
                }
            }
            
            if !user_text.is_empty() {
                // Prepend session_id so we can track which document was retrieved
                let stored_text = format!("[SESSION_ID:{}]\n{}", sess_id, user_text);
                manager.add_memory(wing_name, "global_room", &stored_text)?;
            }
        }
        
        let results = manager.search_memory(wing_name, "global_room", &entry.question)?;
        
        let mut hit = false;
        for retrieved in results {
            if let Some(start) = retrieved.find("[SESSION_ID:") {
                let after_start = &retrieved[start + 12..];
                if let Some(end) = after_start.find("]") {
                    let retrieved_sid = &after_start[..end];
                    if target_answers.contains(&retrieved_sid.to_string()) {
                        hit = true;
                        break;
                    }
                }
            }
        }
        
        if hit {
            hit_at_5 += 1;
        }
        
        println!("Completed query {}/{}: {}", i + 1, entries.len(), if hit { "HIT" } else { "MISS" });
    }
    
    let end_time = start_time.elapsed();
    let recall = (hit_at_5 as f64 / entries.len() as f64) * 100.0;
    
    println!("---");
    println!("Benchmark Completed.");
    println!("Time taken: {:.2?}", end_time);
    println!("Time per query: {:.2?}", end_time / entries.len() as u32);
    println!("Final Recall@5: {:.2}% ({}/{})", recall, hit_at_5, entries.len());
    println!("Expected Baseline (MemPalace): 96.6%");
    println!("---");

    Ok(())
}
