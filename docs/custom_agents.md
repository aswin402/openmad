# Dynamic Agent Configuration Guide (OpenMAD)

This document explains how to define, customize, and load dynamic agent personas within OpenMAD. It is designed to serve as both user reference material and context for AI coding assistants to understand and manipulate OpenMAD's multi-agent registry.

---

## 1. Agent Registry Architecture

OpenMAD maintains a centralized agent catalog called the `AgentRegistry` (defined in [src/agent.rs](file:///home/aswin/programming/vscode/myProjects/ai_agent_tools/openmad/src/agent.rs)).

Each agent is defined by an `AgentPersona` structure:
```rust
pub struct AgentPersona {
    pub name: String,          // The agent's display name (e.g., "Vivian")
    pub title: String,         // The agent's professional title (e.g., "Vision Auditor")
    pub system_prompt: String, // Instruction set defining behaviors & boundaries
    pub principles: Vec<String> // Guiding principles that govern quality control
}
```

### Registration Lifecycle
1. **Defaults**: On startup, the orchestrator registers default personas for core task types (PM, Architect, Business Analyst, Coder, Tester, Writer, Vision Auditor).
2. **Dynamic Overlay**: The registry looks for an `agents.json` file in the directory where the binary is executed.
3. **Overwrite/Inject**: If any keys in `agents.json` match a default agent key (e.g., `"john"`), it overrides that agent persona with the user's custom definition. If a key is new, it injects a new agent into the registry.

---

## 2. Configuration Schema (`agents.json`)

To override or add agents, create an `agents.json` file in your workspace directory matching the following schema:

```json
{
  "<agent_key>": {
    "name": "<Agent Name>",
    "title": "<Professional Title>",
    "system_prompt": "<Core Instructions for LLM System Prompt>",
    "principles": [
      "<Principle 1>",
      "<Principle 2>",
      "<Principle 3>"
    ]
  }
}
```

### Standard Predefined Agent Keys
Override these keys to customize default behaviors for standard workflows:
- `"john"`: Product Manager (TaskType::Planning)
- `"mary"`: Business Analyst (TaskType::Research)
- `"amelia"`: Senior Software Engineer (TaskType::Coding)
- `"winston"`: System Architect (TaskType::Review)
- `"tester"`: QA Test Automator (TaskType::Testing)
- `"paige"`: Technical Writer (TaskType::Documentation)
- `"vivian"`: Vision Auditor (TaskType::Vision)

---

## 3. Dynamic Spawner Lifecycle

When a goal is launched:
1. **Dynamic Selection**: `AgentSpawner::spawn_team_for_goal` evaluates keywords in your goal description to choose the appropriate subset of agents (e.g. vision auditor for UI tasks, developer/architect for refactors).
2. **Resolution**: If a custom agent exists in `agents.json`, the spawner automatically instantiates that customized persona.
3. **Fallback**: If no custom definitions are present, it uses the default agent personas.

---

## 4. Example Override Setup

### Overriding the Product Manager
Create `agents.json` in your workspace root:
```json
{
  "john": {
    "name": "SuperJohn",
    "title": "Lead Product Manager & Architect",
    "system_prompt": "You are SuperJohn, the ultimate Product Manager. You decompose objectives into highly concurrent, independent tasks.",
    "principles": [
      "Aim for maximal parallel execution.",
      "Ensure robust and precise task description outputs."
    ]
  }
}
```

When you execute:
```bash
openmad "Decompose a new database feature"
```
The console log will show:
```text
INFO openmad::orchestrator: Executing task 'T1' via Agent 'SuperJohn' (Lead Product Manager & Architect)
```
