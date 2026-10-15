//! Incremental Server-Sent Events parser (WHATWG `text/event-stream`): feed it response
//! chunks as they arrive, split anywhere (even inside a line or a UTF-8 character).

/// One dispatched event. `event` is `None` for the default "message" type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseEvent {
    pub event: Option<String>,
    pub data: String,
}

/// A single line longer than this is treated as a broken stream.
pub const MAX_LINE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Default)]
pub struct SseParser {
    line: Vec<u8>,
    /// The previous chunk ended in `\r`; a leading `\n` in the next one belongs to it.
    after_cr: bool,
    event: Option<String>,
    data: String,
    has_data: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct LineTooLong;

impl SseParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses `chunk` and returns the events it completed.
    pub fn push(&mut self, chunk: &[u8]) -> Result<Vec<SseEvent>, LineTooLong> {
        let mut events = Vec::new();
        for &byte in chunk {
            if self.after_cr {
                self.after_cr = false;
                if byte == b'\n' {
                    continue;
                }
            }
            match byte {
                b'\n' | b'\r' => {
                    self.after_cr = byte == b'\r';
                    let line = std::mem::take(&mut self.line);
                    if let Some(event) = self.line_done(&line) {
                        events.push(event);
                    }
                }
                _ => {
                    if self.line.len() >= MAX_LINE_BYTES {
                        return Err(LineTooLong);
                    }
                    self.line.push(byte);
                }
            }
        }
        Ok(events)
    }

    /// End of the stream: a last event without its blank line is still delivered.
    pub fn finish(&mut self) -> Option<SseEvent> {
        let line = std::mem::take(&mut self.line);
        let pending = if line.is_empty() {
            None
        } else {
            self.line_done(&line)
        };
        pending.or_else(|| self.dispatch())
    }

    fn line_done(&mut self, line: &[u8]) -> Option<SseEvent> {
        if line.is_empty() {
            return self.dispatch();
        }
        if line[0] == b':' {
            return None; // comment / keep-alive
        }
        let text = String::from_utf8_lossy(line);
        let (field, value) = match text.find(':') {
            Some(at) => {
                let value = &text[at + 1..];
                (&text[..at], value.strip_prefix(' ').unwrap_or(value))
            }
            None => (text.as_ref(), ""),
        };
        match field {
            "data" => {
                if self.has_data {
                    self.data.push('\n');
                }
                self.data.push_str(value);
                self.has_data = true;
            }
            "event" => self.event = Some(value.to_owned()).filter(|e| !e.is_empty()),
            _ => {} // id, retry and unknown fields do not matter here
        }
        None
    }

    fn dispatch(&mut self) -> Option<SseEvent> {
        let event = self.event.take();
        if !self.has_data {
            return None;
        }
        self.has_data = false;
        Some(SseEvent {
            event,
            data: std::mem::take(&mut self.data),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_all(input: &[u8]) -> Vec<SseEvent> {
        let mut parser = SseParser::new();
        let mut events = parser.push(input).expect("push");
        events.extend(parser.finish());
        events
    }

    fn ev(event: Option<&str>, data: &str) -> SseEvent {
        SseEvent {
            event: event.map(str::to_owned),
            data: data.into(),
        }
    }

    #[test]
    fn parses_events_data_and_comments() {
        let input =
            b": keep-alive\n\nevent: ping\ndata: {}\n\ndata: a\ndata: b\nid: 7\n\ndata:x\n\n";
        assert_eq!(
            parse_all(input),
            vec![ev(Some("ping"), "{}"), ev(None, "a\nb"), ev(None, "x")]
        );
    }

    #[test]
    fn handles_every_line_ending() {
        let expected = vec![ev(Some("e"), "1"), ev(None, "2")];
        assert_eq!(
            parse_all(b"event: e\r\ndata: 1\r\n\r\ndata: 2\r\n\r\n"),
            expected
        );
        assert_eq!(parse_all(b"event: e\rdata: 1\r\rdata: 2\r\r"), expected);
        assert_eq!(parse_all(b"event: e\ndata: 1\n\ndata: 2\n\n"), expected);
    }

    #[test]
    fn same_result_however_the_stream_is_split() {
        let input = "event: message_delta\r\ndata: {\"text\":\"héllo 🤖\"}\r\n\r\n: ping\r\ndata: [DONE]\r\n\r\n";
        let whole = parse_all(input.as_bytes());
        assert_eq!(whole.len(), 2);
        assert_eq!(whole[0].data, "{\"text\":\"héllo 🤖\"}");
        let bytes = input.as_bytes();
        for split in 0..bytes.len() {
            let mut parser = SseParser::new();
            let mut events = parser.push(&bytes[..split]).expect("push");
            events.extend(parser.push(&bytes[split..]).expect("push"));
            events.extend(parser.finish());
            assert_eq!(events, whole, "split at {split}");
        }
        let mut parser = SseParser::new();
        let mut events = Vec::new();
        for byte in bytes {
            events.extend(parser.push(std::slice::from_ref(byte)).expect("push"));
        }
        assert_eq!(events, whole, "byte by byte");
    }

    #[test]
    fn a_trailing_event_without_blank_line_is_kept() {
        assert_eq!(parse_all(b"data: last"), vec![ev(None, "last")]);
        assert!(parse_all(b"event: only-a-name\n\n").is_empty());
    }

    #[test]
    fn rejects_runaway_lines() {
        let mut parser = SseParser::new();
        let big = vec![b'a'; MAX_LINE_BYTES + 1];
        assert_eq!(parser.push(&big), Err(LineTooLong));
    }
}
