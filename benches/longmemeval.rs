use std::path::Path;
use std::time::Instant;
use reqwest;
use serde::Deserialize;
use std::fs;

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

fn val_to_str(v: &serde_json::Value) -> String {
    if let Some(s) = v.as_str() {
        s.to_string()
    } else {
        v.to_string()
    }
}

fn run_query(manager: &MemoryManager, entry: &LongMemEvalEntry, wing_name: &str) -> anyhow::Result<(bool, bool)> {
    let target_answers: Vec<String> = entry.answer_session_ids.iter().map(|v| val_to_str(v)).collect();
    let results = manager.search_memory(wing_name, "global_room", &entry.question)?;
    
    let mut found_targets = std::collections::HashSet::new();
    for retrieved in results {
        if let Some(start) = retrieved.find("[SESSION_ID:") {
            let after_start = &retrieved[start + 12..];
            if let Some(end) = after_start.find("]") {
                let retrieved_sid = &after_start[..end];
                if target_answers.contains(&retrieved_sid.to_string()) {
                    found_targets.insert(retrieved_sid.to_string());
                }
            }
        }
    }
    
    let mut hit_any = false;
    let mut hit_all = false;
    if found_targets.len() > 0 {
        hit_any = true;
    }
    if found_targets.len() == target_answers.len() && target_answers.len() > 0 {
        hit_all = true;
    }
    Ok((hit_any, hit_all))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let data_path = "/tmp/longmemeval_s_cleaned.json";
    
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

    let args: Vec<String> = std::env::args().collect();
    let global_corpus_mode = args.contains(&"--global-corpus".to_string());

    let mut global_manager = None;
    let wing_name = "test_wing";

    if global_corpus_mode {
        println!("🚀 Running in GLOBAL CORPUS mode. Ingesting ALL sessions into one database...");
        let manager = MemoryManager::new(":memory:")?;
        let mut ingested_sids = std::collections::HashSet::new();
        
        for entry in &entries {
            for (session_idx, session_turns) in entry.haystack_sessions.iter().enumerate() {
                let sess_id = val_to_str(&entry.haystack_session_ids[session_idx]);
                if ingested_sids.insert(sess_id.clone()) {
                    let mut user_text = String::new();
                    for turn in session_turns {
                        if turn.role == "user" {
                            if !user_text.is_empty() { user_text.push('\n'); }
                            user_text.push_str(&turn.content);
                        }
                    }
                    if !user_text.is_empty() {
                        let stored_text = format!("[SESSION_ID:{}]\n{}", sess_id, user_text);
                        manager.add_memory(wing_name, "global_room", &stored_text)?;
                    }
                }
            }
        }
        println!("✅ Global corpus ingested ({} unique sessions). Starting queries...", ingested_sids.len());
        global_manager = Some(manager);
    }

    let mut hit_any_at_5 = 0;
    let mut hit_all_at_5 = 0;
    
    let start_time = Instant::now();

    for (i, entry) in entries.iter().enumerate() {
        let (hit_any, hit_all) = if global_corpus_mode {
            let manager = global_manager.as_ref().unwrap();
            run_query(manager, entry, wing_name)?
        } else {
            let local_mgr = MemoryManager::new(":memory:")?;
            for (session_idx, session_turns) in entry.haystack_sessions.iter().enumerate() {
                let sess_id = val_to_str(&entry.haystack_session_ids[session_idx]);
                let mut user_text = String::new();
                for turn in session_turns {
                    if turn.role == "user" {
                        if !user_text.is_empty() { user_text.push('\n'); }
                        user_text.push_str(&turn.content);
                    }
                }
                
                if !user_text.is_empty() {
                    let stored_text = format!("[SESSION_ID:{}]\n{}", sess_id, user_text);
                    local_mgr.add_memory(wing_name, "global_room", &stored_text)?;
                }
            }
            run_query(&local_mgr, entry, wing_name)?
        };

        if hit_any { hit_any_at_5 += 1; }
        if hit_all { hit_all_at_5 += 1; }
        
        println!("Completed query {}/{}: ANY={}, ALL={}", i + 1, entries.len(), hit_any, hit_all);
    }
    
    let end_time = start_time.elapsed();
    let recall_any = (hit_any_at_5 as f64 / entries.len() as f64) * 100.0;
    let recall_all = (hit_all_at_5 as f64 / entries.len() as f64) * 100.0;
    
    println!("---");
    println!("Benchmark Completed.");
    println!("Mode: {}", if global_corpus_mode { "Global Corpus (Hard)" } else { "Per-Question (Easy/Baseline)" });
    println!("Time taken: {:.2?}", end_time);
    println!("Time per query: {:.2?}", end_time / entries.len() as u32);
    println!("Final Recall_Any@5: {:.2}% ({}/{}) [Optimistic]", recall_any, hit_any_at_5, entries.len());
    println!("Final Recall_All@5: {:.2}% ({}/{}) [Strict]", recall_all, hit_all_at_5, entries.len());
    println!("---");

    Ok(())
}
