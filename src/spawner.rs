#![allow(dead_code)]
use tracing::info;
use uuid::Uuid;
use crate::agent::{AgentInstance, AgentPersona, AgentRegistry};
use crate::dag_planner::TaskType;

#[derive(Debug, Clone)]
pub struct AgentMessage {
    pub sender_id: String,
    pub sender_name: String,
    pub recipient_id: String,
    pub content: String,
}

pub struct AgentSpawner {
    registry: AgentRegistry,
    message_hub_tx: flume::Sender<AgentMessage>,
    message_hub_rx: flume::Receiver<AgentMessage>,
}

impl AgentSpawner {
    pub fn new() -> Self {
        let (tx, rx) = flume::unbounded();
        Self {
            registry: AgentRegistry::new(),
            message_hub_tx: tx,
            message_hub_rx: rx,
        }
    }

    /// Spawns a list of agents based on a goal complexity assessment.
    /// Returns the spawned agent instances.
    pub fn spawn_team_for_goal(&self, goal: &str) -> Vec<AgentInstance> {
        let goal_lower = goal.to_lowercase();
        let mut team = Vec::new();

        // Dynamically analyze complexity to build the team
        if goal_lower.contains("website") || goal_lower.contains("saas") || goal_lower.contains("app") {
            info!("Goal complexity evaluated as HIGH. Spawning full development team.");
            // Spawn PM (John), BA (Mary), UX (Sally), Architect (Winston), Developer (Amelia), and Tester
            if let Some(john) = self.registry.get("john") {
                team.push(self.create_instance("john-id", john, TaskType::Planning));
            }
            if let Some(mary) = self.registry.get("mary") {
                team.push(self.create_instance("mary-id", mary, TaskType::Research));
            }
            if let Some(winston) = self.registry.get("winston") {
                team.push(self.create_instance("winston-id", winston, TaskType::Planning));
            }
            if let Some(amelia) = self.registry.get("amelia") {
                team.push(self.create_instance("amelia-id", amelia, TaskType::Coding));
            }
            if let Some(tester) = self.registry.get("tester") {
                team.push(self.create_instance("tester-id", tester, TaskType::Testing));
            }
            if let Some(paige) = self.registry.get("paige") {
                team.push(self.create_instance("paige-id", paige, TaskType::Documentation));
            }
            if let Some(vivian) = self.registry.get("vivian") {
                team.push(self.create_instance("vivian-id", vivian, TaskType::Vision));
            }
        } else if goal_lower.contains("ui") || goal_lower.contains("design") || goal_lower.contains("css") || goal_lower.contains("frontend") || goal_lower.contains("visual") || goal_lower.contains("vision") {
            info!("Goal contains visual UI/design requests. Spawning Vision Auditor team.");
            if let Some(john) = self.registry.get("john") {
                team.push(self.create_instance("john-id", john, TaskType::Planning));
            }
            if let Some(amelia) = self.registry.get("amelia") {
                team.push(self.create_instance("amelia-id", amelia, TaskType::Coding));
            }
            if let Some(vivian) = self.registry.get("vivian") {
                team.push(self.create_instance("vivian-id", vivian, TaskType::Vision));
            }
        } else if goal_lower.contains("fix") || goal_lower.contains("bug") || goal_lower.contains("refactor") {
            info!("Goal complexity evaluated as MEDIUM. Spawning focused debugging team.");
            // Spawn just Coder (Amelia) and Reviewer (Winston)
            if let Some(amelia) = self.registry.get("amelia") {
                team.push(self.create_instance("amelia-id", amelia, TaskType::Coding));
            }
            if let Some(winston) = self.registry.get("winston") {
                team.push(self.create_instance("winston-id", winston, TaskType::Review));
            }
        } else {
            info!("Goal complexity evaluated as LOW. Spawning minimal execution team.");
            // Spawn just Amelia for direct execution
            if let Some(amelia) = self.registry.get("amelia") {
                team.push(self.create_instance("amelia-id", amelia, TaskType::Coding));
            }
        }

        team
    }

    /// Spawns a single specialized agent for a specific task.
    pub fn spawn_agent_for_task(&self, task_type: TaskType) -> AgentInstance {
        let id = format!("{}-{}", match task_type {
            TaskType::Planning => "planner",
            TaskType::Research => "researcher",
            TaskType::Coding => "coder",
            TaskType::Review => "reviewer",
            TaskType::Testing => "tester",
            TaskType::Documentation => "writer",
            TaskType::Vision => "vision",
        }, Uuid::new_v4().to_string().split_at(8).0);

        let key = match task_type {
            TaskType::Planning => "john",
            TaskType::Research => "mary",
            TaskType::Coding => "amelia",
            TaskType::Review => "winston",
            TaskType::Testing => "tester",
            TaskType::Documentation => "paige",
            TaskType::Vision => "vivian",
        };

        let persona = self.registry.get(key).cloned().unwrap_or_else(|| AgentPersona {
            name: "AgentBot".to_string(),
            title: "Autonomous Agent".to_string(),
            system_prompt: "You are an autonomous assistant. Work efficiently to solve the task.".to_string(),
            principles: vec!["Execute tasks with precision.".to_string()],
        });

        self.create_instance(&id, &persona, task_type)
    }

    pub fn get_message_hub(&self) -> (flume::Sender<AgentMessage>, flume::Receiver<AgentMessage>) {
        (self.message_hub_tx.clone(), self.message_hub_rx.clone())
    }

    fn create_instance(&self, id: &str, persona: &AgentPersona, task_type: TaskType) -> AgentInstance {
        AgentInstance {
            id: id.to_string(),
            persona: persona.clone(),
            task_type,
        }
    }
}
