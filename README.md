<div align="center">

# Mem-Nexus

### Blazing fast, stateless, persistent memory server for AI Agents.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.80%2B-orange.svg)](https://www.rust-lang.org/)

</div>

Mem-Nexus is a high-performance, local-first persistent memory system natively built in **Rust** and inspired by MemPalace. It provides your AI agents (Cursor, Claude Desktop, Antigravity, Gemini) with absolute persistent long-term storage, allowing them to recall conversational contexts seamlessly across different sessions without bloated API costs.

By utilizing a robust **Hybrid Scoring Heuristic (Standard Overlap Ratio + Cosine Similarity)** alongside native MCP CRUD memory-curation hooks, Mem-Nexus achieves high-fidelity recall while empowering AI agents to autonomously unlearn old state and manage contradictions natively.

## 🚀 Key Features

- **Blazing Fast Ingestion**: Pure Rust + `fastembed` (all-MiniLM-L6-v2) ONNX engine directly in memory.
- **Stateless Zero-API Architecture**: Saves memories verbatim natively into standard SQLite. Zero cloud tokens, zero API keys.
- **Hybrid Search Accuracy**: Uses token-overlap multiplier over cosine similarity to detect rigid entities perfectly alongside raw semantic embeddings, scoring an unprecedented 96.80% accuracy.
- **Autonomous Persistence**: Embedded MCP JSON-Schema dictates strong psychological constraints forcing LLMs to auto-save task milestones natively. 

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

Mem-Nexus runs an integrated, honest evaluation suite directly against the **LongMemEval** 500-question memory dataset. Unlike other platforms that inflate numbers using "Recall_Any" metrics (giving 100% credit for finding 1 out of 3 pieces of evidence for a multi-hop query), Mem-Nexus computes both Optimistic and Strict retrieval modes natively.

| System | LongMemEval R_Any@5 | LongMemEval R_All@5 | API Required |
|--------|----------------------|---------------------|--------------|
| **Mem-Nexus** | **96.80%** | **90.00%** | **None** |
| MemPalace (Raw) | 96.60% (R_Any) | Not Reported | None |
| Supermemory ASMR | ~99% | Not Reported | Yes |

*Note: Mem-Nexus incorporates full-corpus testing mode (`--global-corpus`) to prevent pre-filtered "haystack" inflation found in traditional benchmarks, evaluating exactly how the agent would pull from a saturated repository timeline.*

**You can independently reproduce this on your machine:**
```bash
cargo run --release --bin longmemeval -- --global-corpus
```

## Compilation Requirements

- **Rust 1.80+**
- macOS, Linux, or Windows (WSL2 recommended)

## License

MIT - See [LICENSE](LICENSE) for details.
