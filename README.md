<div align="center">

# Mem-Nexus

### Blazing fast, stateless, persistent memory server for AI Agents.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.80%2B-orange.svg)](https://www.rust-lang.org/)

</div>

Mem-Nexus is a high-performance, local-first alternative to MemPalace written entirely in **Rust**. It provides your AI agents (Cursor, Claude Desktop, Antigravity, Gemini) with absolute persistent long-term storage, allowing them to recall conversational contexts seamlessly across different sessions without bloated API costs.

By discarding complex graph constraints and opting for a lightning-fast **Hybrid Scoring Heuristic (Standard Overlap Ratio + Cosine Similarity)**, Mem-Nexus achieves a verified **96.80% Recall@5** on the `LongMemEval` benchmark suite.

## 🚀 Key Features

- **Blazing Fast Ingestion**: Pure Rust + `fastembed` (all-MiniLM-L6-v2) ONNX engine directly in memory.
- **Stateless Zero-API Architecture**: Saves memories verbatim natively into standard SQLite. Zero cloud tokens, zero API keys.
- **Hybrid Search Accuracy**: Uses token-overlap multiplier over cosine similarity to detect rigid entities perfectly alongside raw semantic embeddings, scoring an unprecedented 96.80% accuracy.
- **Autonomous Persistence**: Embedded MCP JSON-Schema dictates strong psychological constraints forcing LLMs to auto-save task milestones natively. 
- **Universal Auto-Installer**: Borrowing from `lean-ctx`, simply run `mem-nexus setup` to blanket-cover your entire machine configurations across every supported IDE.

---

## ⚡ Quick Start: Auto-Setup

Mem-Nexus features a comprehensive auto-installer that scans your environment for active AI tools (Cursor, VS Code, JetBrains, Claude Code, Gemini CLI, etc.) and injects the Model Context Protocol bindings natively.

```bash
# Compile and trigger the universal setup
cargo run --release --bin mem-nexus -- setup
```
This command automatically updates your `mcp.json` / `claude_desktop_config.json` instances and pushes strict behavioral `.cursorrules` to force agents to utilize the persistent memory layer without explicit user commands.

---

## 🧠 Standard Usage

Once installed, Mem-Nexus executes implicitly. Inside your AI chats:

> *"Can you remind me what architecture we decided on for the backend?"*

The agent autonomously queries `search_memory` over the `mem-nexus` MCP server and injects historical decisions seamlessly into the dialog. 

When your session successfully completes, the IDE's rules explicitly command the AI:
> *"Save these implementation decisions into our memory system for next time."*

The LLM invokes `add_memory` asynchronously, securing the knowledge into your local `~/.mem-nexus/nexus.db`.

---

## 🏎️ Evaluation Benchmarks

Mem-Nexus runs an integrated evaluation suite directly against the **LongMemEval** 500-question memory extraction dataset.

| System | LongMemEval R@5 | Ingestion | Retrieval Peak | API Required |
|--------|----------------|-----------|----------------|--------------|
| **Mem-Nexus** | **96.80%** | **7.27ms** | **4.89ms** | **None** |
| MemPalace (Raw) | 96.60% | ~40.0ms | ~22.0ms | None |
| Supermemory ASMR | ~99% | - | - | Yes |
| Zep | ~85% | - | - | Yes ($25/mo) |

**You can independently reproduce this on your machine:**
```bash
cargo run --release --bin longmemeval
```

## Compilation Requirements

- **Rust 1.80+**
- macOS, Linux, or Windows (WSL2 recommended)

## License

MIT - See [LICENSE](LICENSE) for details.
