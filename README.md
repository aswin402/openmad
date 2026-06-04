# OpenMAD: Advanced Autonomous Multi-Agent Orchestrator

<p align="center">
  <img src="openmad_logo.png" alt="OpenMAD Logo" width="400" />
</p>

OpenMAD (**Open M**ulti-**A**gent **D**eveloper) is a high-performance, active multi-agent orchestrator runtime written in Rust. It is inspired by the agile software delivery phases of the [BMad Method](https://github.com/bmad-code-org/bmad-method) (Business Analyst, PM, Architect, Developer, QA, Writer), but evolves the methodology from a static prompt-based system into a fully automated, concurrent, and self-repairing agentic pipeline.

---

## 🚀 Key Features

*   **Dynamic Agent Spawning**: Analyzes the complexity of user goals and recruits only the specialized agents needed (Mary/BA, John/PM, Winston/Architect, Amelia/Developer, Vivian/Vision Auditor, TestBot/QA) to perform the task, teardown agents after goal accomplishment.
*   **Parallel Task Execution (DAG)**: Decomposes objectives into a Directed Acyclic Graph of tasks and runs non-dependent tasks concurrently using Tokio task joins.
*   **Multimodal Vision Integration**: Integrates a Vision Auditor Agent (Vivian) to view images, analyze grid spacing, verify layouts, and audit visual diagrams using multimodal LLMs (e.g. Gemini 2.5 Flash).
*   **Multi-Model Router & Fallbacks**: Routes task types to specialized LLMs (Qwen/DeepSeek for coding, Claude/Gemini for research) and handles fail-safe execution using fallback chains (Primary $\rightarrow$ Secondary $\rightarrow$ Fallback).
*   **Local Vector Memory**: Uses the `fastembed` crate to generate local vector embeddings, powering semantic context injection for active agents using Cosine Similarity checks.
*   **Tree-Sitter AST Parsing**: Evaluates developer agent outputs using a local tree-sitter compiler-level parser to count nodes and syntax types before submitting outputs for audit.
*   **Reflection & Self-Repair**: Reviewer agent (Winston) inspects outputs against acceptance criteria, initiating a loop-back correction run for failed items to fix bugs automatically.

---

## 📊 Workflow Graph

```mermaid
graph TD
    Goal["User Goal: 'Design a web UI with Rust backend'"] --> John["John (Product Manager)"]
    John --> DAG["Decomposed Task DAG"]
    DAG --> Orchestrator["Tokio Event Core"]
    
    Orchestrator --> Spawner["Spawner (Recruits Agents)"]
    Spawner --> Team["Spawned Team: PM, Coder, Vision Auditor, QA"]
    
    subgraph Execution Pipeline
        T1["T1: Design Specifications (John)"] --> T2["T2: Implement Code (Amelia)"]
        T2 --> T3["T3: Visual Layout Audit (Vivian)"]
        T3 --> T4["T4: Run Verification Tests (TestBot)"]
    end
    
    T2 -- "Saves Code Output" --> TS["Tree-Sitter AST Parse Check"]
    TS -- "Generates Report" --> T3
    
    T3 -- "Multimodal API" --> Gemini["Gemini 2.5 Flash (Inspected Layout UI)"]
    Gemini -- "Feedback: Spacing issue on sidebar" --> Repair["Self-Repair Run: Feedback to Amelia"]
    Repair --> T2
    
    T4 -- "Approved" --> Shared["Save to Shared DashMap Workspace"]
    Shared --> Final["Deliver Final Artifacts"]
```

---

## ⚡ Runtime Footprint & Benchmarks

OpenMAD is written in pure Rust, ensuring exceptionally lightweight footprints compared to Python or Node-based frameworks:

*   **Startup Latency**: <10ms (from CLI command invocation to initial task execution).
*   **RAM Consumption**: 
    *   *Idle / Sleep*: ~45MB.
    *   *Active Run (embeddings generation + AST parsing)*: ~90MB to 120MB.
*   **ROM Footprint**: ~12MB (fully compiled release binary containing fastembed, tree-sitter, and network drivers).
*   **CPU Optimization**: Leverages Tokio’s multi-threaded work-stealing scheduler to map agent loops concurrently to all available CPU cores.

---

## ⚖️ Comparison: BMAD Method vs OpenMAD

| Capability | BMad Method (Original) | OpenMAD (Evolved) |
| :--- | :--- | :--- |
| **System Model** | Static prompts/instructions for human-in-the-loop IDE agents | Autonomous orchestration runtime with active agent event loops |
| **Target Directory** | Nested `.claude/` or `.cursor/` folders | Directly under project root `openmad/` |
| **Concurrency** | Sequential chat execution | Concurrent parallel execution of independent tasks via Tokio |
| **Agent Roster** | Hardcoded configs (`mary`, `john`, `amelia`) | Dynamic agent spawner based on goal complexity |
| **LLM Router** | Static model selection per IDE | Dynamic model routing based on past success rates |
| **Errors / Failures** | Execution halt / user intervention | Multi-model fallback chains (Gemini $\rightarrow$ Claude $\rightarrow$ OpenAI) |
| **Memory** | Global context markdown files | Local Vector memory via `fastembed` with Cosine Similarity |
| **Code Verification** | Manual compilation check / linters | Tree-sitter AST validation before reviewer audit |
| **Design Audits** | Manual visual inspection | Vision Auditor (Vivian) using multimodal vision LLMs |

---

## 🛠️ Quick Start

### Prerequisites
*   Rust / Cargo 1.94+
*   ONNX Runtime binaries (automatically downloaded on first build via the `ort` dependency)

### Build and Run
1. Clone the project.
2. Build in release mode:
   ```bash
   cargo build --release
   ```
3. Run with a custom goal:
   ```bash
   cargo run -- "Build a landing page with a modern UI layout and Rust backend"
   ```

*Note: Model endpoints will automatically fall back to mock offline simulation modes if API keys (`GEMINI_API_KEY`, `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`) are not set in the environment.*

---

## 📂 Documentation

Detailed docs are available under the `docs/` folder:
- [Architecture](docs/architecture.md) — Internal system modules and connection graphs.
- [Features](docs/features.md) — Detailed analysis of dynamic spawning, fallbacks, and vector memory.
- [Code Explanations](docs/code_explanation.md) — Traversal of implementation scripts and structures.
