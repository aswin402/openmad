# Features and Capabilities

This document details the core features and advanced capabilities of the **OpenMAD** multi-agent runtime.

---

## 👥 Dynamic Agent Spawning

Instead of maintaining a fixed roster of active threads, OpenMAD dynamically spins agents up and down based on task requirements:
*   **Goal Complexity Parsing**: Evaluates user inputs for complexity (e.g. standard fixes vs complex applications).
*   **Target Recruitment**: Allocates unique channel IDs and spawns specific personas (PM, Coder, Vision Auditor, Tester) onto the Tokio runtime.
*   **Resource Reclamation**: Destroys the agent instances and shuts down their respective message channels upon goal completion to minimize memory footprints.

---

## 🔀 Model Routing & Multi-Model Fallbacks

OpenMAD contains a resilient LLM routing system that prevents execution locks:
- **Provider Routing**: Task categories are routed to specialized models (e.g. Qwen Coder or DeepSeek for syntax tasks, Claude Sonnet for research, Gemini for planning and vision).
- **Fallback Chains**: If a model call fails (e.g. rate limits, network timeout, safety blocks), the orchestrator moves to the secondary model, and finally the backup model.
- **Success Mapping**: Logs model performance statistics. If a model fails frequently on a task category, the system dynamically decreases its priority and routes subsequent requests to a more successful model.

---

## 🧠 Letta-Style Stateful Memory (LLM-as-OS)

OpenMAD implements a stateful hierarchical memory system inspired by Letta (formerly MemGPT). This system splits memory into distinct operational tiers:

- **Core Memory (RAM)**: Prominently visible in the agent prompt as XML blocks (`<core_memory>`). It houses blocks like `persona` (agent self-instructions) and `human` (user profiles).
- **Agentic Memory Autonomy**: The prompt contains instructions for the model to update its own core memory. The orchestrator parses the output for `<update_core_memory block="...">` tags, updates the live agent state, and saves it.
- **Persistent JSON Storage**: The core memory is serialized and stored in `letta_memory_store.json`. Agents automatically load their persistent state in subsequent execution runs, meaning they remember user preferences and past execution specifications long-term.
- **Recall & Archival Memory (Local Vector)**: Generates vector embeddings locally using `fastembed` and queries historical files and logs using cosine similarity checks to retrieve semantic contexts.
- **Shared Workspace**: Thread-safe concurrent sharing of intermediate task results using `dashmap::DashMap`.

---

## 🔍 AST Parsing (Tree-Sitter)

Coding outputs are audited at the syntactic level before human or agent reviews:
- **Tree-Sitter Parser**: Parses code buffers into Abstract Syntax Tree (AST) node representations.
- **Syntax Validation**: Counts specific node structures (e.g. functions, struct boundaries, error handlers) to confirm syntactic compliance and catch basic syntax errors before submitting to model-based audits.

---

## 👁️ Visual Auditing (Vision Agent)

A specialized Vision Auditor Agent (Vivian) handles visual layout tasks:
- **Trigger**: Activated automatically if a goal includes terms related to visual design, CSS styling, layouts, screenshots, or diagrams.
- **Multimodal Routing**: Connects to vision-capable models (e.g. `gemini-2.5-flash` or `gpt-4o`) to inspect UI mockups, verify grid spacing, check color contrasts (WCAG compliance), and confirm fidelity against wireframe schemas.

---

## 🔁 Reflection and Self-Repair

A critique-and-correct loop is built directly into task execution:
1. **Auditing**: Winston (Architect/Reviewer) reviews outputs against acceptance criteria.
2. **Rejection**: If review fails, Winston produces detailed feedback (repro steps/bugs list).
3. **Self-Repair Loop**: The developer agent receives the feedback, updates its output, and submits it back to Winston. The loop continues recursively until approved.
