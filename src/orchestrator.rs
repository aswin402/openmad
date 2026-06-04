use std::sync::{Arc, Mutex};
use tracing::{info, warn, error};
use futures_util::future::join_all;
use crate::dag_planner::{TaskDag, TaskStatus, TaskType};
use crate::spawner::AgentSpawner;
use crate::model_router::ModelRouter;
use crate::tool_router::ToolRouter;
use crate::memory::{MemoryEngine, SharedAgentMemory};
use crate::reflection::ReflectionSystem;

pub struct Orchestrator {
    spawner: AgentSpawner,
    model_router: Arc<ModelRouter>,
    memory_engine: Arc<MemoryEngine>,
    shared_memory: Arc<SharedAgentMemory>,
    reflection_system: Arc<ReflectionSystem>,
}

impl Orchestrator {
    pub fn new() -> Self {
        Self {
            spawner: AgentSpawner::new(),
            model_router: Arc::new(ModelRouter::new()),
            memory_engine: Arc::new(MemoryEngine::new()),
            shared_memory: Arc::new(SharedAgentMemory::new()),
            reflection_system: Arc::new(ReflectionSystem::new(2)), // 2 self-repair retries max
        }
    }

    /// Primary entrypoint to execute a high-level user goal.
    pub async fn run_goal(&self, goal: &str) -> anyhow::Result<()> {
        info!("=== STARTING ORCHESTRATION FOR GOAL: '{}' ===", goal);

        // 1. Spawning dynamic agent team based on complexity
        let team = self.spawner.spawn_team_for_goal(goal);
        info!("Spawned dynamic agent team of size: {}", team.len());
        for agent in &team {
            info!("  * Agent: {} ({}) for TaskType::{:?}", agent.persona.name, agent.persona.title, agent.task_type);
        }

        // 2. Generate initial plan DAG using Planner agent (John)
        info!("Planning task Directed Acyclic Graph (DAG)...");
        let planner_agent = team.iter().find(|a| a.task_type == TaskType::Planning)
            .cloned()
            .unwrap_or_else(|| self.spawner.spawn_agent_for_task(TaskType::Planning));

        let system_prompt = format!(
            "{}\n\nYour goal is to break down the user objective into 3-5 sequential, structured tasks with strict dependencies. You must return a valid JSON array of tasks where each entry has: 'id', 'title', 'description', 'type' (Planning, Research, Coding, Review, Testing, Documentation), and 'dependencies' (array of string IDs).",
            planner_agent.persona.system_prompt
        );

        let user_prompt = format!("Goal to decompose: '{}'", goal);
        let (plan_output, model_used) = self.model_router.execute_prompt(
            TaskType::Planning,
            &system_prompt,
            &user_prompt,
        ).await?;

        info!("Decomposition planning completed using model: {}", model_used);

        // Parse generated JSON into TaskDag
        let mut dag = TaskDag::new();
        if let Ok(tasks_json) = serde_json::from_str::<serde_json::Value>(&plan_output) {
            if let Some(arr) = tasks_json.as_array() {
                for t_val in arr {
                    let id = t_val["id"].as_str().unwrap_or_default().to_string();
                    let title = t_val["title"].as_str().unwrap_or_default().to_string();
                    let desc = t_val["description"].as_str().unwrap_or_default().to_string();
                    let t_type_str = t_val["type"].as_str().unwrap_or_default();
                    let t_type = match t_type_str {
                        "Planning" => TaskType::Planning,
                        "Research" => TaskType::Research,
                        "Coding" => TaskType::Coding,
                        "Review" => TaskType::Review,
                        "Testing" => TaskType::Testing,
                        "Vision" => TaskType::Vision,
                        _ => TaskType::Documentation,
                    };
                    let deps: Vec<String> = t_val["dependencies"]
                        .as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .map(|v| v.as_str().unwrap_or_default().to_string())
                        .collect();

                    dag.add_task(id, title, desc, t_type, deps);
                }
            }
        }

        // Safe fallback if parsing fails
        if dag.tasks.is_empty() {
            warn!("JSON parsing failed or empty. Applying fallback standard DAG.");
            dag.add_task("T1".to_string(), "Analyze design".to_string(), "Assess design requirements.".to_string(), TaskType::Planning, vec![]);
            dag.add_task("T2".to_string(), "Implement framework".to_string(), "Write core codes.".to_string(), TaskType::Coding, vec!["T1".to_string()]);
            
            let goal_lower = goal.to_lowercase();
            if goal_lower.contains("ui") || goal_lower.contains("design") || goal_lower.contains("css") || goal_lower.contains("visual") || goal_lower.contains("vision") {
                dag.add_task("T3".to_string(), "Visual audit interface".to_string(), "Perform UI styling verification, grid spacing check, and design fidelity audit.".to_string(), TaskType::Vision, vec!["T2".to_string()]);
                dag.add_task("T4".to_string(), "Run verifications".to_string(), "Perform testing checks on backend and visual elements.".to_string(), TaskType::Testing, vec!["T3".to_string()]);
            } else {
                dag.add_task("T3".to_string(), "Run verifications".to_string(), "Perform testing checks.".to_string(), TaskType::Testing, vec!["T2".to_string()]);
            }
        }

        println!("\n{}", dag.visualize());

        // 3. Execution loop (with parallel branch execution support using Tokio joining)
        let dag_mutex = Arc::new(Mutex::new(dag));

        while {
            let dag_guard = dag_mutex.lock().unwrap();
            !dag_guard.is_complete() && !dag_guard.has_failures()
        } {
            // Get all tasks ready for execution
            let ready_tasks = {
                let dag_guard = dag_mutex.lock().unwrap();
                dag_guard.get_ready_tasks()
            };

            if ready_tasks.is_empty() {
                // Circular dependency fallback or waiting
                break;
            }

            info!("Spawning parallel execution for ready tasks: {:?}", ready_tasks);

            let mut task_futures = Vec::new();
            for task_id in ready_tasks {
                // Mark task as Running
                {
                    let mut dag_guard = dag_mutex.lock().unwrap();
                    dag_guard.update_status(&task_id, TaskStatus::Running, None);
                }

                // Copy required context
                let dag_clone = dag_mutex.clone();
                let spawner_ref = &self.spawner;
                let router_clone = self.model_router.clone();
                let memory_clone = self.memory_engine.clone();
                let shared_clone = self.shared_memory.clone();
                let reflection_clone = self.reflection_system.clone();

                let task_future = async move {
                    let task = {
                        let dag_guard = dag_clone.lock().unwrap();
                        dag_guard.tasks.get(&task_id).cloned().unwrap()
                    };

                    // Retrieve specialized agent instance
                    let mut agent = spawner_ref.spawn_agent_for_task(task.task_type);
                    info!("Executing task '{}' via Agent '{}' ({})", task.id, agent.persona.name, agent.persona.title);

                    // Search semantic memory for context
                    let memory_context = memory_clone.query_semantic(&task.description, 3);
                    let mut semantic_notes = String::new();
                    if !memory_context.is_empty() {
                        semantic_notes.push_str("\n--- Relevant Memory Context ---\n");
                        for (text, score) in memory_context {
                            semantic_notes.push_str(&format!("* {} (Similarity: {:.2})\n", text, score));
                        }
                    }

                    // Build user prompt
                    let user_prompt = format!(
                        "Goal: {}\nTask Title: {}\nDescription: {}\n{}\nInputs from Shared Memory:\n{:?}",
                        goal, task.title, task.description, semantic_notes, shared_clone.list_keys()
                    );

                    // Execute task with self-repair reflection loop
                    let final_result = reflection_clone.run_self_repair_loop(
                        &router_clone,
                        &task.title,
                        &task.description,
                        "".to_string(), // starting empty to trigger model run inside loop
                        |prev_output, feedback| {
                            let router = router_clone.clone();
                            let system_prompt = format!(
                                "{}\n\n=== LETTA-STYLE HIERARCHICAL CORE MEMORY ===\n{}\n\n=== MEMORY UPDATE PROTOCOL ===\nYou can autonomously update your core memory blocks. If you learn anything new about the user or your goals, or wish to refine your persona instructions, output your changes using this exact tag format:\n<update_core_memory block=\"human\">new info about the user</update_core_memory>\n<update_core_memory block=\"persona\">new self-instructions or skill notes</update_core_memory>\nDO NOT output placeholders. Write the full updated value.",
                                agent.persona.system_prompt,
                                agent.core_memory.to_xml()
                            );
                            let user_prompt_run = if prev_output.is_empty() {
                                user_prompt.clone()
                            } else {
                                format!(
                                    "{}\n\nPrevious attempt failed review.\nFeedback: {}\nPlease correct the output according to the feedback.",
                                    user_prompt, feedback
                                )
                            };

                            async move {
                                let (out, _) = router.execute_prompt(
                                    task.task_type,
                                    &system_prompt,
                                    &user_prompt_run,
                                ).await?;
                                Ok(out)
                            }
                        }
                    ).await;

                    match final_result {
                        Ok(res) => {
                            info!("Successfully completed task: '{}'", task.id);
                            
                            // Parse and apply autonomous Letta-style memory updates
                            parse_and_apply_memory_updates(&res, &agent.persona.name, &mut agent.core_memory, &spawner_ref.memory_store);

                            if task.task_type == TaskType::Coding {
                                let mut tr = ToolRouter::new();
                                if let Ok(ast_report) = tr.execute_tool("ast_parse", &res) {
                                    info!("AST analysis report:\n{}", ast_report);
                                }
                            }

                            // Store in long-term and shared memory
                            memory_clone.store_memory(&res, &format!("task-{}", task.id));
                            shared_clone.publish(&task.id, &res);

                            // Update DAG status
                            let mut dag_guard = dag_clone.lock().unwrap();
                            dag_guard.update_status(&task.id, TaskStatus::Completed, Some(res));
                        }
                        Err(e) => {
                            error!("Task '{}' execution failed: {}", task.id, e);
                            let mut dag_guard = dag_clone.lock().unwrap();
                            dag_guard.update_status(&task.id, TaskStatus::Failed, None);
                        }
                    }
                };

                task_futures.push(task_future);
            }

            // Join all concurrent tasks
            join_all(task_futures).await;
        }

        let dag_final = dag_mutex.lock().unwrap();
        if dag_final.is_complete() {
            info!("=== ORCHESTRATION SUCCESS: ALL TASKS COMPLETED ===");
            println!("\nFinal Generated Artifacts:");
            for key in self.shared_memory.list_keys() {
                if let Some(artifact) = self.shared_memory.get(&key) {
                    println!("\nArtifact [{}]:\n{}", key, artifact);
                }
            }
        } else {
            error!("=== ORCHESTRATION FAILED: TASK TREE CONTAINS FAILURES OR BLOCKED DEPENDENCIES ===");
        }

        Ok(())
    }
}

fn parse_and_apply_memory_updates(
    output: &str,
    agent_name: &str,
    core_memory: &mut crate::memory::CoreMemory,
    memory_store: &crate::memory::AgentMemoryStore,
) {
    let start_tag_prefix = "<update_core_memory block=\"";
    let end_tag = "</update_core_memory>";

    let mut current_pos = 0;
    while let Some(start_idx) = output[current_pos..].find(start_tag_prefix) {
        let absolute_start = current_pos + start_idx;
        let block_name_start = absolute_start + start_tag_prefix.len();
        
        if let Some(quote_idx) = output[block_name_start..].find('"') {
            let block_name_end = block_name_start + quote_idx;
            let block_name = &output[block_name_start..block_name_end];
            
            let val_start = block_name_end + 2; // skip `">`
            if val_start < output.len() {
                if let Some(end_idx) = output[val_start..].find(end_tag) {
                    let absolute_end = val_start + end_idx;
                    let val = &output[val_start..absolute_end];
                    
                    // Update core memory block
                    core_memory.set_block(block_name, val.trim());
                    info!("[Letta Memory System] Agent '{}' autonomously updated core memory block '{}' to: '{}'", agent_name, block_name, val.trim());
                    
                    // Save memory
                    if let Err(e) = memory_store.save_memory(agent_name, core_memory) {
                        error!("Failed to save persistent memory for agent '{}': {}", agent_name, e);
                    }
                    
                    current_pos = absolute_end + end_tag.len();
                    continue;
                }
            }
        }
        current_pos += start_tag_prefix.len();
    }
}
