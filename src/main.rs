mod dag_planner;
mod agent;
mod spawner;
mod model_router;
mod memory;
mod tool_router;
mod reflection;
mod orchestrator;

use tracing_subscriber::EnvFilter;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    // 2. Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let goal = if args.len() > 1 {
        args[1..].join(" ")
    } else {
        // Default goal for demo demonstration
        "Build a simple Rust weather CLI application".to_string()
    };

    println!("====================================================");
    println!("     OpenMAD - Advanced Multi-Agent Orchestrator   ");
    println!("====================================================\n");

    // 3. Initialize and run the Orchestrator
    let orchestrator = orchestrator::Orchestrator::new();
    orchestrator.run_goal(&goal).await?;

    println!("\n====================================================");
    println!("   OpenMAD Execution Finished Successfully          ");
    println!("====================================================");

    Ok(())
}
