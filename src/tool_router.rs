use std::fs;
use std::process::Command;
use tracing::info;
use tree_sitter::Parser;

pub struct ToolRouter {
    parser: Parser,
}

impl ToolRouter {
    pub fn new() -> Self {
        let parser = Parser::new();
        Self { parser }
    }

    /// Dispatches tool calls from agents to their actual implementations.
    pub fn execute_tool(&mut self, tool_name: &str, args: &str) -> anyhow::Result<String> {
        info!("Executing tool '{}' with arguments: {}", tool_name, args);
        match tool_name {
            "read_file" => self.read_file(args),
            "write_file" => self.write_file_arg(args),
            "run_command" => self.run_command(args),
            "ast_parse" => self.ast_parse(args),
            "web_search" => self.web_search(args),
            _ => Err(anyhow::anyhow!("Unknown tool: {}", tool_name)),
        }
    }

    fn read_file(&self, path: &str) -> anyhow::Result<String> {
        let trimmed_path = path.trim().trim_matches('"');
        let content = fs::read_to_string(trimmed_path)?;
        Ok(content)
    }

    fn write_file_arg(&self, args: &str) -> anyhow::Result<String> {
        // Simple parser for write_file tool args: "path|content"
        let parts: Vec<&str> = args.splitn(2, '|').collect();
        if parts.len() < 2 {
            return Err(anyhow::anyhow!("Invalid arguments for write_file. Format must be 'path|content'"));
        }
        let path = parts[0].trim().trim_matches('"');
        let content = parts[1];
        
        fs::create_dir_all(std::path::Path::new(path).parent().unwrap_or(std::path::Path::new(".")))?;
        fs::write(path, content)?;
        Ok(format!("Successfully wrote content to file: {}", path))
    }

    fn run_command(&self, cmd: &str) -> anyhow::Result<String> {
        let trimmed_cmd = cmd.trim().trim_matches('"');
        info!("Running terminal command: {}", trimmed_cmd);
        
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd").args(["/C", trimmed_cmd]).output()?
        } else {
            Command::new("sh").args(["-c", trimmed_cmd]).output()?
        };

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let err = String::from_utf8_lossy(&output.stderr).to_string();
            Err(anyhow::anyhow!("Command failed: {}", err))
        }
    }

    /// Uses tree-sitter to parse source code and analyze its structure (AST).
    fn ast_parse(&mut self, code: &str) -> anyhow::Result<String> {
        // In a full implementation, we set the language (e.g. parser.set_language(&tree_sitter_rust::language()))
        // Here, we parse the tree and walk the nodes, reporting the overall node count and type hierarchy.
        let tree = self.parser.parse(code, None)
            .ok_or_else(|| anyhow::anyhow!("Tree-sitter failed to parse code"))?;
        
        let mut node_count = 0;
        let mut depth_map = std::collections::HashMap::new();
        
        // Simple DFS traversal to count and analyze nodes
        let mut cursor = tree.walk();
        let mut reached_root = false;
        
        while !reached_root {
            node_count += 1;
            let kind = cursor.node().kind();
            *depth_map.entry(kind.to_string()).or_insert(0) += 1;
            
            if cursor.goto_first_child() {
                continue;
            }
            if cursor.goto_next_sibling() {
                continue;
            }
            
            loop {
                if !cursor.goto_parent() {
                    reached_root = true;
                    break;
                }
                if cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        let mut report = format!(
            "AST Analysis Report:\n- Total Syntax Nodes parsed: {}\n- Node types count:\n",
            node_count
        );
        
        for (kind, count) in depth_map {
            report.push_str(&format!("  * {}: {}\n", kind, count));
        }

        Ok(report)
    }

    fn web_search(&self, query: &str) -> anyhow::Result<String> {
        info!("Searching web for: {}", query);
        // Return structured results for search simulation
        Ok(format!(
            "SEARCH RESULTS FOR '{}':\n1. Rust Crate Ecosystem Guide: Using tokio for async loops, crossbeam for parallel message pipelines.\n2. Model Performance comparison: Qwen-coder-32B excels at low-level Rust syntax construction compared to generalist models.", 
            query
        ))
    }
}
