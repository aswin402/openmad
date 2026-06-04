# Code Explanations

This document walks through the implementation files under the `src/` directory.

---

## 📂 Source Code Files

### 1. `main.rs`
The entrypoint of the application.
- Initializes structured logging using `tracing_subscriber` with default filter parameters.
- Inspects CLI command line arguments to parse the target goal.
- Spawns the central `Orchestrator` using `#[tokio::main]` async runtime driver.

### 2. `orchestrator.rs`
Coordinates the agent workflow execution loop.
- **`Orchestrator::run_goal`**:
  1. Requests `AgentSpawner` to spawn the team.
  2. Submits the goal to the Planning agent to build the initial task dependency tree.
  3. Enters an execution loop while the DAG contains outstanding tasks.
  4. Identifies ready tasks, marks their states to `Running`, and schedules them concurrently using `tokio::spawn` and `futures_util::future::join_all`.
  5. Upon coding task completions, instantiates `ToolRouter` to run tree-sitter AST queries before storing results.

### 3. `dag_planner.rs`
Implements the workflow dependency graph.
- **`Task`**: Struct representing a single step containing ID, title, type, status, and dependee parent IDs list.
- **`TaskDag`**: Struct housing a `HashMap<String, Task>` representing the entire graph.
- **`get_ready_tasks`**: Scans the tasks map and returns IDs where status is `Ready` or `Pending` with all dependent parent tasks marked `Completed`.
- **`update_status`**: Marks a task's status and re-evaluates all children tasks to flip unblocked children to `Ready`.

### 4. `agent.rs`
Defines agent personas.
- **`AgentPersona`**: Struct detailing the name, title, principles, and system instructions of an agent.
- **`AgentRegistry`**: Thread-safe registry containing defaults for PM (John), BA (Mary), Architect (Winston), Developer (Amelia), Vision (Vivian), QA (TestBot), and Writer (Paige).

### 5. `spawner.rs`
Dynamically recruits and coordinates communication channels for agents.
- **`AgentMessage`**: Defines the data package (sender, recipient, contents) passed over inter-agent channels.
- **`spawn_team_for_goal`**: Evaluates visual keywords or complexity to construct and return a vector of `AgentInstance` profiles.
- **`spawn_agent_for_task`**: Spawns a single specialized agent for a particular task type on demand.

### 6. `model_router.rs`
Coordinates LLM clients, fallbacks, and success rates.
- **`ModelConfig` & `FallbackChain`**: Define model identities, providers (Gemini, OpenAI, Anthropic, Mock), and API key environments.
- **`ModelRouter::execute_prompt`**: Dispatches prompts to the fallback chain. It attempts the primary model. If it fails, it moves to the secondary model, and finally the backup/mock model.
- **`record_performance`**: Logs success/failure metrics in a thread-safe `RwLock<HashMap<String, ModelPerformance>>` map.
- **`call_gemini`/`call_anthropic`/`call_openai`**: Implements HTTPS REST client wrappers using `reqwest` and `serde_json` to call upstream providers.

### 7. `memory.rs`
Provides semantic search context, hierarchical memory structures, and persistence.
- **`MemoryEngine`**: Wraps the `fastembed` model loader. If fastembed fails or is offline, it degrades gracefully.
- **`store_memory`**: Embeds text snippets and saves them into the vector database list.
- **`query_semantic`**: Scans stored vector embeddings using **Cosine Similarity** to return the top matching semantic contexts for a query.
- **`SharedAgentMemory`**: Uses `dashmap::DashMap` to provide concurrent, lock-free artifact storage across worker threads.
- **`CoreMemory`**: Represents the structured blocks (e.g. `persona`, `human`) that act as agent RAM and are serialized to XML.
- **`AgentMemoryStore`**: Serializes core memory states to `letta_memory_store.json` and loads them back on demand.


### 8. `tool_router.rs`
Implements tools for agents.
- **`read_file` / `write_file_arg`**: Safe disk access.
- **`run_command`**: Spawns shell command processes (`sh` on Linux, `cmd` on Windows) and returns stdout/stderr.
- **`ast_parse`**: Invokes the `tree-sitter` parser to parse code inputs, traverses the tree nodes using `tree.walk()` cursor, and returns node types summary statistics.

### 9. `reflection.rs`
Defines audits and self-repair loops.
- **`review_output`**: Submits the generated task output to Winston (Architect) to check compliance, looking for a `STATUS: APPROVED` or `STATUS: REJECTED` string.
- **`run_self_repair_loop`**: Runs a loop-back query. If Winston rejects the work, it triggers a repair function, combining previous output and Winston's feedback to generate corrected updates. It retries up to the maximum retries limit.
