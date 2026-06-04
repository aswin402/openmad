use tracing::{info, warn};
use crate::model_router::ModelRouter;
use crate::dag_planner::TaskType;

pub struct ReflectionSystem {
    max_retries: usize,
}

impl ReflectionSystem {
    pub fn new(max_retries: usize) -> Self {
        Self { max_retries }
    }

    /// Reviews the task output using a Reviewer agent persona.
    /// Returns true if the output is approved, false otherwise.
    pub async fn review_output(
        &self,
        router: &ModelRouter,
        task_title: &str,
        task_desc: &str,
        output: &str,
    ) -> anyhow::Result<(bool, String)> {
        info!("Reviewing output for task: '{}'...", task_title);

        let system_prompt = "You are Winston, the System Architect and Code Reviewer. Your role is to examine task results and determine if they satisfy the task requirements. You must output either 'STATUS: APPROVED' if everything is correct, or a detailed breakdown of failures and how to fix them starting with 'STATUS: REJECTED'.".to_string();
        
        let user_prompt = format!(
            "Task Title: {}\nTask Description: {}\n\nAgent Output to Review:\n{}\n\nDoes this meet all requirements? Please review and provide status.",
            task_title, task_desc, output
        );

        let (review_result, model_used) = router.execute_prompt(
            TaskType::Review,
            &system_prompt,
            &user_prompt,
        ).await?;

        let is_approved = review_result.contains("STATUS: APPROVED") || !review_result.contains("STATUS: REJECTED");
        
        if is_approved {
            info!("Task '{}' APPROVED by Reviewer (Model: {})", task_title, model_used);
        } else {
            warn!("Task '{}' REJECTED by Reviewer (Model: {}). Detailed feedback: {}", task_title, model_used, review_result);
        }

        Ok((is_approved, review_result))
    }

    /// Runs a self-repair loop where an agent has multiple attempts to fix its work based on reviewer feedback.
    pub async fn run_self_repair_loop<F, Fut>(
        &self,
        router: &ModelRouter,
        task_title: &str,
        task_desc: &str,
        initial_output: String,
        mut repair_fn: F,
    ) -> anyhow::Result<String>
    where
        F: FnMut(String, String) -> Fut,
        Fut: std::future::Future<Output = anyhow::Result<String>>,
    {
        let mut current_output = initial_output;
        if current_output.is_empty() {
            current_output = repair_fn("".to_string(), "".to_string()).await?;
        }
        let mut retry_count = 0;

        loop {
            let (approved, feedback) = self.review_output(router, task_title, task_desc, &current_output).await?;
            if approved {
                return Ok(current_output);
            }

            retry_count += 1;
            if retry_count > self.max_retries {
                warn!("Max reflection retries ({}) reached for '{}'. Proceeding with current state.", self.max_retries, task_title);
                return Ok(current_output);
            }

            info!("Triggering Self-Repair Loop iteration {}/{} for '{}'...", retry_count, self.max_retries, task_title);
            current_output = repair_fn(current_output, feedback).await?;
        }
    }
}
