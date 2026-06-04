use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskType {
    Planning,
    Research,
    Coding,
    Review,
    Testing,
    Documentation,
    Vision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Ready,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub dependencies: Vec<String>,
    pub result: Option<String>,
    pub assigned_agent: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskDag {
    pub tasks: HashMap<String, Task>,
}

impl TaskDag {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
        }
    }

    pub fn add_task(&mut self, id: String, title: String, description: String, task_type: TaskType, dependencies: Vec<String>) {
        let status = if dependencies.is_empty() {
            TaskStatus::Ready
        } else {
            TaskStatus::Pending
        };

        self.tasks.insert(
            id.clone(),
            Task {
                id,
                title,
                description,
                task_type,
                status,
                dependencies,
                result: None,
                assigned_agent: None,
            },
        );
    }

    /// Finds all tasks that are currently "Ready" to be executed.
    /// A task is ready if its status is Ready, or if it is Pending and all its dependencies are Completed.
    pub fn get_ready_tasks(&self) -> Vec<String> {
        let mut ready = Vec::new();
        for (id, task) in &self.tasks {
            if task.status == TaskStatus::Ready {
                ready.push(id.clone());
            } else if task.status == TaskStatus::Pending {
                let all_deps_done = task.dependencies.iter().all(|dep_id| {
                    if let Some(dep_task) = self.tasks.get(dep_id) {
                        dep_task.status == TaskStatus::Completed
                    } else {
                        // If dependency doesn't exist, assume we can skip or it's completed (safe fallback)
                        true
                    }
                });
                if all_deps_done {
                    ready.push(id.clone());
                }
            }
        }
        ready
    }

    /// Checks if the entire DAG is complete.
    pub fn is_complete(&self) -> bool {
        self.tasks.values().all(|t| t.status == TaskStatus::Completed)
    }

    /// Checks if any task has failed.
    pub fn has_failures(&self) -> bool {
        self.tasks.values().any(|t| t.status == TaskStatus::Failed)
    }

    /// Updates a task's status and resolves downstream tasks.
    pub fn update_status(&mut self, id: &str, status: TaskStatus, result: Option<String>) {
        if let Some(task) = self.tasks.get_mut(id) {
            task.status = status;
            if result.is_some() {
                task.result = result;
            }
        }

        // Re-evaluate downstream tasks
        if status == TaskStatus::Completed {
            let mut to_ready = Vec::new();
            for (tid, t) in &self.tasks {
                if t.status == TaskStatus::Pending {
                    let all_deps_done = t.dependencies.iter().all(|dep_id| {
                        if let Some(dep_t) = self.tasks.get(dep_id) {
                            dep_t.status == TaskStatus::Completed
                        } else {
                            true
                        }
                    });
                    if all_deps_done {
                        to_ready.push(tid.clone());
                    }
                }
            }

            for tid in to_ready {
                if let Some(t) = self.tasks.get_mut(&tid) {
                    t.status = TaskStatus::Ready;
                }
            }
        }
    }

    /// Visualizes the DAG structure.
    pub fn visualize(&self) -> String {
        let mut visual = String::new();
        visual.push_str("Task DAG:\n");
        for task in self.tasks.values() {
            let status_emoji = match task.status {
                TaskStatus::Pending => "⏳",
                TaskStatus::Ready => "⚡",
                TaskStatus::Running => "🌀",
                TaskStatus::Completed => "✅",
                TaskStatus::Failed => "❌",
            };
            visual.push_str(&format!(
                "  {} [{}] {} - Type: {:?}\n",
                status_emoji, task.id, task.title, task.task_type
            ));
            if !task.dependencies.is_empty() {
                visual.push_str(&format!("     Depends on: {}\n", task.dependencies.join(", ")));
            }
        }
        visual
    }
}
