//! Hook execution engine

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, warn};

use super::discovery::{HookDef, discover_hooks};
use super::event::HookEvent;

/// Decision returned by hook execution
#[derive(Debug, Clone)]
pub enum HookDecision {
    /// Allow the operation to proceed
    Allow,
    /// Block the operation with a reason
    Block(String),
}

impl HookDecision {
    /// Check if this decision allows the operation
    pub fn is_allowed(&self) -> bool {
        matches!(self, HookDecision::Allow)
    }
}

/// Hook engine that manages and executes hooks
pub struct HookEngine {
    hooks: Vec<HookDef>,
}

impl HookEngine {
    /// Create a new HookEngine, discovering hooks from the workspace
    pub fn new(workspace: &Path) -> Self {
        let hooks = discover_hooks(workspace);
        Self { hooks }
    }

    /// Create an empty HookEngine with no hooks
    pub fn empty() -> Self {
        Self { hooks: Vec::new() }
    }

    /// Get the number of registered hooks
    pub fn hook_count(&self) -> usize {
        self.hooks.len()
    }

    /// Fire an event to all matching hooks
    ///
    /// For modifying hooks (before_tool_call), returns HookDecision::Block
    /// if any hook returns non-zero exit code.
    ///
    /// For read-only hooks, always returns Allow.
    pub async fn fire(&self, event: &HookEvent) -> HookDecision {
        let event_name = event.event_name();
        let matching: Vec<_> = self
            .hooks
            .iter()
            .filter(|h| h.matches_event(event_name) && h.is_enabled())
            .collect();

        if matching.is_empty() {
            return HookDecision::Allow;
        }

        debug!("Firing event '{}' to {} hook(s)", event_name, matching.len());

        for hook in matching {
            match self.run_hook(hook, event).await {
                HookDecision::Allow => {
                    debug!("Hook '{}' allowed event", hook.name);
                }
                HookDecision::Block(reason) => {
                    warn!("Hook '{}' blocked event: {}", hook.name, reason);
                    // Only return Block for modifying events
                    if event.is_modifying() {
                        return HookDecision::Block(reason);
                    }
                }
            }
        }

        HookDecision::Allow
    }

    /// Run a single hook command
    async fn run_hook(&self, def: &HookDef, event: &HookEvent) -> HookDecision {
        let event_json = match serde_json::to_string(event) {
            Ok(json) => json,
            Err(e) => {
                warn!("Failed to serialize event for hook '{}': {}", def.name, e);
                return HookDecision::Allow; // Fail open
            }
        };

        let timeout_duration = Duration::from_millis(def.timeout_ms);

        let result = timeout(timeout_duration, async {
            let mut child = match Command::new("sh")
                .arg("-c")
                .arg(&def.command)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(child) => child,
                Err(e) => {
                    return Err(format!("Failed to spawn hook command: {}", e));
                }
            };

            // Write event JSON to stdin
            if let Some(mut stdin) = child.stdin.take()
                && let Err(e) = stdin.write_all(event_json.as_bytes()).await
            {
                return Err(format!("Failed to write to hook stdin: {}", e));
            }

            // Wait for completion
            match child.wait().await {
                Ok(status) => Ok(status),
                Err(e) => Err(format!("Hook wait failed: {}", e)),
            }
        })
        .await;

        match result {
            Ok(Ok(status)) => {
                if status.success() {
                    HookDecision::Allow
                } else {
                    let code = status.code().unwrap_or(-1);
                    HookDecision::Block(format!("Hook '{}' exited with code {}", def.name, code))
                }
            }
            Ok(Err(e)) => {
                warn!("Hook '{}' failed: {}", def.name, e);
                HookDecision::Allow // Fail open for read-only hooks
            }
            Err(_) => {
                warn!("Hook '{}' timed out after {}ms", def.name, def.timeout_ms);
                // Treat timeout as a block for modifying events
                if event.is_modifying() {
                    HookDecision::Block(format!("Hook '{}' timed out", def.name))
                } else {
                    HookDecision::Allow
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;

    #[test]
    fn test_empty_engine() {
        let engine = HookEngine::empty();
        assert_eq!(engine.hook_count(), 0);
    }

    #[tokio::test]
    async fn test_fire_no_matching_hooks() {
        let engine = HookEngine::empty();
        let event = HookEvent::BeforeToolCall {
            tool_name: "bash".to_string(),
            arguments: json!({}),
            session_id: "test".to_string(),
        };

        let decision = engine.fire(&event).await;
        assert!(decision.is_allowed());
    }

    #[tokio::test]
    async fn test_hook_decision_allow() {
        assert!(HookDecision::Allow.is_allowed());
    }

    #[tokio::test]
    async fn test_hook_decision_block() {
        let block = HookDecision::Block("test reason".to_string());
        assert!(!block.is_allowed());
    }

    #[test]
    fn test_engine_with_nonexistent_workspace() {
        let engine = HookEngine::new(PathBuf::from("/nonexistent/workspace").as_path());
        assert_eq!(engine.hook_count(), 0);
    }
}
