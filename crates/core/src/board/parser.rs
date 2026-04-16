use crate::types::Op;

/// Errors that can occur parsing a single board line.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("unknown op type")]
    UnknownOp,
}

/// Parse a single line from board.jsonl into an Op.
///
/// Returns `Err(ParseError::UnknownOp)` for valid JSON with an unrecognized `op` field.
/// Returns `Err(ParseError::InvalidJson)` for malformed JSON.
pub fn parse_line(line: &str) -> Result<Op, ParseError> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err(ParseError::InvalidJson(
            serde_json::from_str::<Op>("").unwrap_err(),
        ));
    }

    match serde_json::from_str::<Op>(trimmed) {
        Ok(op) => Ok(op),
        Err(serde_err) => {
            // Check if it's valid JSON with an unknown op
            if let Ok(raw) = serde_json::from_str::<serde_json::Value>(trimmed) {
                if let Some(op_field) = raw.get("op").and_then(|v| v.as_str()) {
                    match op_field {
                        "add" | "clear" | "stale" => {
                            // Known op but bad fields — return the serde error
                            Err(ParseError::InvalidJson(serde_err))
                        }
                        _ => Err(ParseError::UnknownOp),
                    }
                } else {
                    Err(ParseError::InvalidJson(serde_err))
                }
            } else {
                Err(ParseError::InvalidJson(serde_err))
            }
        }
    }
}

/// Parse multiple lines, skipping blank and malformed lines.
/// Returns `(ops, skipped_count)`.
pub fn parse_lines(text: &str) -> (Vec<Op>, usize) {
    let mut ops = Vec::new();
    let mut skipped = 0;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match parse_line(trimmed) {
            Ok(op) => ops.push(op),
            Err(e) => {
                tracing::warn!(line = trimmed, error = %e, "skipping malformed board line");
                skipped += 1;
            }
        }
    }

    (ops, skipped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::NotificationType;

    #[test]
    fn test_parse_valid_add_op() {
        let line = r#"{"op":"add","ts":"2026-04-16T10:00:00Z","session_id":"abc-123","cwd":"/home/user/project","claude_pid":1234,"terminal_pid":5678,"transcript_path":"/tmp/transcript.jsonl","notification_type":"permission_prompt","message":"May I run ls?"}"#;
        let op = parse_line(line).unwrap();
        match op {
            Op::Add {
                session_id,
                notification_type,
                claude_pid,
                ..
            } => {
                assert_eq!(session_id, "abc-123");
                assert_eq!(notification_type, NotificationType::PermissionPrompt);
                assert_eq!(claude_pid, 1234);
            }
            _ => panic!("expected Add op"),
        }
    }

    #[test]
    fn test_parse_valid_clear_op() {
        let line = r#"{"op":"clear","ts":"2026-04-16T10:01:00Z","session_id":"abc-123","reason":"user_replied"}"#;
        let op = parse_line(line).unwrap();
        match op {
            Op::Clear {
                session_id, reason, ..
            } => {
                assert_eq!(session_id, "abc-123");
                assert_eq!(reason, "user_replied");
            }
            _ => panic!("expected Clear op"),
        }
    }

    #[test]
    fn test_parse_valid_stale_op() {
        let line = r#"{"op":"stale","ts":"2026-04-16T10:02:00Z","session_id":"abc-123","reason":"pid_dead"}"#;
        let op = parse_line(line).unwrap();
        match op {
            Op::Stale {
                session_id, reason, ..
            } => {
                assert_eq!(session_id, "abc-123");
                assert_eq!(reason, "pid_dead");
            }
            _ => panic!("expected Stale op"),
        }
    }

    #[test]
    fn test_parse_idle_prompt_type() {
        let line = r#"{"op":"add","ts":"2026-04-16T10:00:00Z","session_id":"def-456","cwd":"/tmp","claude_pid":999,"terminal_pid":null,"transcript_path":"/tmp/t.jsonl","notification_type":"idle_prompt","message":"Waiting"}"#;
        let op = parse_line(line).unwrap();
        match op {
            Op::Add {
                notification_type,
                terminal_pid,
                ..
            } => {
                assert_eq!(notification_type, NotificationType::IdlePrompt);
                assert!(terminal_pid.is_none());
            }
            _ => panic!("expected Add op"),
        }
    }

    #[test]
    fn test_parse_malformed_json() {
        let line = "not json at all{{{";
        assert!(parse_line(line).is_err());
    }

    #[test]
    fn test_parse_unknown_op() {
        let line = r#"{"op":"future_op","ts":"2026-04-16T10:00:00Z","session_id":"x"}"#;
        let err = parse_line(line).unwrap_err();
        assert!(matches!(err, ParseError::UnknownOp));
    }

    #[test]
    fn test_parse_empty_line() {
        assert!(parse_line("").is_err());
    }

    #[test]
    fn test_parse_lines_mixed() {
        let text = concat!(
            r#"{"op":"add","ts":"2026-04-16T10:00:00Z","session_id":"a","cwd":"/tmp","claude_pid":1,"terminal_pid":null,"transcript_path":"/tmp/t","notification_type":"permission_prompt","message":"m"}"#,
            "\n",
            "bad line\n",
            "\n",
            r#"{"op":"clear","ts":"2026-04-16T10:01:00Z","session_id":"a","reason":"stop"}"#,
            "\n",
        );
        let (ops, skipped) = parse_lines(text);
        assert_eq!(ops.len(), 2);
        assert_eq!(skipped, 1);
    }

    #[test]
    fn test_parse_line_with_extra_fields_is_forward_compatible() {
        let line = r#"{"op":"add","ts":"2026-04-16T10:00:00Z","session_id":"a","cwd":"/tmp","claude_pid":1,"terminal_pid":null,"transcript_path":"/tmp/t","notification_type":"permission_prompt","message":"m","new_field":"ignored"}"#;
        assert!(parse_line(line).is_ok());
    }
}
