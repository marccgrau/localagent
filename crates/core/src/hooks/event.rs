//! Hook event types for the lifecycle hook system

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Events that can trigger hooks at key points in the agent pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum HookEvent {
    /// Fired before a tool is executed. Can block execution.
    BeforeToolCall {
        tool_name: String,
        arguments: Value,
        session_id: String,
    },
    /// Fired after a tool completes execution (read-only)
    AfterToolCall {
        tool_name: String,
        arguments: Value,
        result: String,
        duration_ms: u64,
    },
    /// Fired when a user message is received
    OnMessage {
        content: String,
        session_id: String,
        channel: String,
    },
    /// Fired when a new session is created
    OnSessionStart { session_id: String },
    /// Fired when a session ends
    OnSessionEnd {
        session_id: String,
        turn_count: usize,
    },
}

impl HookEvent {
    /// Get the event name as a string (matches HookDef.event field)
    pub fn event_name(&self) -> &'static str {
        match self {
            HookEvent::BeforeToolCall { .. } => "before_tool_call",
            HookEvent::AfterToolCall { .. } => "after_tool_call",
            HookEvent::OnMessage { .. } => "on_message",
            HookEvent::OnSessionStart { .. } => "on_session_start",
            HookEvent::OnSessionEnd { .. } => "on_session_end",
        }
    }

    /// Whether this event type can modify/block the operation
    pub fn is_modifying(&self) -> bool {
        matches!(self, HookEvent::BeforeToolCall { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_event_serialization() {
        let event = HookEvent::BeforeToolCall {
            tool_name: "bash".to_string(),
            arguments: json!({"command": "ls"}),
            session_id: "test-session".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        // Serde adjacently tagged format: {"event":"BeforeToolCall","data":{...}}
        assert!(json.contains("BeforeToolCall"));
        assert!(json.contains("bash"));
        assert!(json.contains("test-session"));
    }

    #[test]
    fn test_event_name() {
        let event = HookEvent::OnSessionStart {
            session_id: "test".to_string(),
        };
        assert_eq!(event.event_name(), "on_session_start");
    }

    #[test]
    fn test_is_modifying() {
        let before = HookEvent::BeforeToolCall {
            tool_name: "bash".to_string(),
            arguments: json!({}),
            session_id: "test".to_string(),
        };
        assert!(before.is_modifying());

        let after = HookEvent::AfterToolCall {
            tool_name: "bash".to_string(),
            arguments: json!({}),
            result: "ok".to_string(),
            duration_ms: 100,
        };
        assert!(!after.is_modifying());
    }
}
