use criterion::{criterion_group, criterion_main, Criterion, BatchSize};
use rusqlite::Connection;

#[path = "../src/db.rs"]
mod db;
#[path = "../src/embed.rs"]
mod embed;
#[path = "../src/manager.rs"]
mod manager;

use manager::MemoryManager;

fn bench_ingestion(c: &mut Criterion) {
    c.bench_function("ingest_1_memory", |b| {
        b.iter_batched(
            || MemoryManager::new(":memory:").unwrap(),
            |manager| {
                manager.add_memory("bench_wing", "bench_room", "This is a simple bench test memory.")
            },
            BatchSize::PerIteration,
        );
    });
}

fn bench_search(c: &mut Criterion) {
    let mut manager = MemoryManager::new(":memory:").unwrap();
    // Seed with 100 entries
    for i in 0..100 {
        manager.add_memory("bench_wing", "bench_room", &format!("Context memory chunk number {}", i)).unwrap();
    }
    
    c.bench_function("search_100_memories", |b| {
        b.iter(|| {
            manager.search_memory("bench_wing", "bench_room", "querying for chunk 55")
        })
    });
}

criterion_group!(benches, bench_ingestion, bench_search);
criterion_main!(benches);
