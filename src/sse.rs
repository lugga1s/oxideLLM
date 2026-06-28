// SPDX-License-Identifier: AGPL-3.0-or-later

//! SSE (Server-Sent Events) parser utility for oxideLLM.

#![allow(dead_code)]

/// A parsed Server-Sent Event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseEvent {
    /// The event name/type. Defaults to "message".
    pub event: String,
    /// The payload of the event.
    pub data: String,
    /// The event identifier, if any.
    pub id: Option<String>,
    /// The retry timeout, if any.
    pub retry: Option<u64>,
}

/// A parser state machine for SSE streams.
#[derive(Default)]
pub struct SseParser {
    buffer: String,
    current_event: String,
    current_data: String,
    current_id: Option<String>,
    current_retry: Option<u64>,
}

impl SseParser {
    /// Creates a new SSE parser.
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds new bytes/string chunk into the parser and returns any completed events.
    pub fn feed(&mut self, input: &str) -> Vec<SseEvent> {
        self.buffer.push_str(input);
        let mut events = Vec::new();

        // Process line by line
        while let Some(pos) = self.buffer.find('\n') {
            let line = self.buffer[..pos].trim_end_matches('\r').to_string();
            self.buffer.drain(..=pos);

            if line.is_empty() {
                // Empty line acts as event boundary
                if !self.current_data.is_empty() {
                    events.push(SseEvent {
                        event: if self.current_event.is_empty() {
                            "message".to_string()
                        } else {
                            std::mem::take(&mut self.current_event)
                        },
                        data: std::mem::take(&mut self.current_data),
                        id: self.current_id.take(),
                        retry: self.current_retry.take(),
                    });
                }
            } else if line.starts_with(':') {
                // Comment line, ignore
            } else {
                let (field, value) = match line.split_once(':') {
                    Some((f, v)) => {
                        // Strip leading space if present
                        let v = v.strip_prefix(' ').unwrap_or(v);
                        (f, v)
                    }
                    None => (line.as_str(), ""),
                };

                match field {
                    "event" => {
                        self.current_event = value.to_string();
                    }
                    "data" => {
                        if !self.current_data.is_empty() {
                            self.current_data.push('\n');
                        }
                        self.current_data.push_str(value);
                    }
                    "id" => {
                        self.current_id = Some(value.to_string());
                    }
                    "retry" => {
                        if let Ok(ms) = value.parse::<u64>() {
                            self.current_retry = Some(ms);
                        }
                    }
                    _ => {}
                }
            }
        }

        events
    }

    /// Checks if a string contains the standard SSE end-of-stream marker.
    pub fn is_done_marker(data: &str) -> bool {
        data.trim() == "[DONE]"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_line_parsing() {
        let mut parser = SseParser::new();
        let events = parser.feed("event: update\ndata: value\nid: 1\nretry: 1000\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            SseEvent {
                event: "update".to_string(),
                data: "value".to_string(),
                id: Some("1".to_string()),
                retry: Some(1000),
            }
        );
    }

    #[test]
    fn test_sse_done_marker_detection() {
        assert!(SseParser::is_done_marker("[DONE]"));
        assert!(SseParser::is_done_marker(" [DONE] \n"));
        assert!(!SseParser::is_done_marker("data: [DONE]"));
    }

    #[test]
    fn test_sse_incomplete_chunk_handling() {
        let mut parser = SseParser::new();
        let events1 = parser.feed("data: start");
        assert!(events1.is_empty());

        let events2 = parser.feed("ing\n\n");
        assert_eq!(events2.len(), 1);
        assert_eq!(events2[0].data, "starting");
    }

    #[test]
    fn test_sse_empty_lines_ignored() {
        let mut parser = SseParser::new();
        let events = parser.feed("\n\n\n:comment\n\n");
        assert!(events.is_empty());
    }

    #[test]
    fn test_sse_multi_line_data() {
        let mut parser = SseParser::new();
        let events = parser.feed("data: first\ndata: second\n\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].data, "first\nsecond");
    }
}
