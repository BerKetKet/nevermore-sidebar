/// Deterministic parser for `dbus-monitor` output of `org.freedesktop.Notifications.Notify`.
///
/// D-Bus Signature: `susssasa{sv}i`
/// 1: string app_name
/// 2: uint32 replaces_id
/// 3: string app_icon
/// 4: string summary
/// 5: string body
/// 6: array [ actions ]
/// 7: array [ dict entry( string key, variant val ) hints ]
/// 8: int32 expire_timeout
///
/// Immediately emits on the `int32` line closing the Notify call.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedNotification {
    pub app_name: String,
    pub summary: String,
    pub body: String,
    pub desktop_entry: String,
    pub timestamp_unix: u64,
}

#[derive(Debug, Default)]
pub struct DbusNotificationParser {
    state: ParserState,
    current_app_name: String,
    current_summary: String,
    current_body: String,
    current_desktop_entry: Option<String>,
    multiline_buf: Option<(MultilineTarget, String)>,
    in_actions: bool,
    in_hints: bool,
    waiting_desktop_entry_variant: bool,
}

#[derive(Debug, Default, PartialEq, Eq)]
enum ParserState {
    #[default]
    Idle,
    InNotify {
        arg_index: usize,
    },
}

#[derive(Debug, PartialEq, Eq)]
enum MultilineTarget {
    AppName,
    Summary,
    Body,
}

impl DbusNotificationParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds a single line of dbus-monitor output. Returns Some(ParsedNotification) immediately
    /// when the closing `int32` line of a Notify call is processed.
    pub fn feed_line(&mut self, line: &str) -> Option<ParsedNotification> {
        let trimmed = line.trim();

        // 1. Check for method call header
        if trimmed.starts_with("method call ") && trimmed.contains("member=Notify") {
            self.state = ParserState::InNotify { arg_index: 0 };
            self.current_app_name.clear();
            self.current_summary.clear();
            self.current_body.clear();
            self.current_desktop_entry = None;
            self.multiline_buf = None;
            self.in_actions = false;
            self.in_hints = false;
            self.waiting_desktop_entry_variant = false;
            return None;
        }

        // 2. Handle active multi-line string accumulation
        if let Some((target, mut text)) = self.multiline_buf.take() {
            let line_trimmed = line.trim_end();
            if line_trimmed.ends_with('"') && !line_trimmed.ends_with("\\\"") {
                // Final line of multi-line string
                let content = &line_trimmed[..line_trimmed.len() - 1];
                text.push('\n');
                text.push_str(content);
                match target {
                    MultilineTarget::AppName => self.current_app_name = text,
                    MultilineTarget::Summary => self.current_summary = text,
                    MultilineTarget::Body => self.current_body = text,
                }
            } else {
                text.push('\n');
                text.push_str(line);
                self.multiline_buf = Some((target, text));
            }
            return None;
        }

        // If not in a Notify method call, ignore other signals/calls
        let arg_index = match self.state {
            ParserState::InNotify { arg_index } => arg_index,
            ParserState::Idle => return None,
        };

        // 3. Hints parsing
        if self.in_hints {
            if self.waiting_desktop_entry_variant {
                if trimmed.contains("string \"") {
                    if let Some(start) = trimmed.find("string \"") {
                        let rem = &trimmed[start + 8..];
                        if let Some(end) = rem.rfind('"') {
                            self.current_desktop_entry = Some(rem[..end].to_string());
                        }
                    }
                    self.waiting_desktop_entry_variant = false;
                }
            } else if trimmed.contains("string \"desktop-entry\"") {
                self.waiting_desktop_entry_variant = true;
            }
        }

        // 4. Structural elements
        if let Some(content_part) = trimmed.strip_prefix("string \"") {
            // Check if single line: ends with '"' and not the opening quote itself
            if content_part.ends_with('"') && !content_part.ends_with("\\\"") {
                let s = &content_part[..content_part.len() - 1];
                match arg_index {
                    0 => {
                        self.current_app_name = s.to_string();
                        self.state = ParserState::InNotify { arg_index: 1 };
                    }
                    2 => {
                        // app_icon
                        self.state = ParserState::InNotify { arg_index: 3 };
                    }
                    3 => {
                        self.current_summary = s.to_string();
                        self.state = ParserState::InNotify { arg_index: 4 };
                    }
                    4 => {
                        self.current_body = s.to_string();
                        self.state = ParserState::InNotify { arg_index: 5 };
                    }
                    _ => {}
                }
            } else {
                // Multi-line string begins
                let target = match arg_index {
                    0 => {
                        self.state = ParserState::InNotify { arg_index: 1 };
                        MultilineTarget::AppName
                    }
                    3 => {
                        self.state = ParserState::InNotify { arg_index: 4 };
                        MultilineTarget::Summary
                    }
                    4 => {
                        self.state = ParserState::InNotify { arg_index: 5 };
                        MultilineTarget::Body
                    }
                    _ => return None,
                };
                self.multiline_buf = Some((target, content_part.to_string()));
            }
        } else if trimmed.starts_with("uint32 ") {
            if arg_index == 1 {
                // replaces_id
                self.state = ParserState::InNotify { arg_index: 2 };
            }
        } else if trimmed.starts_with("array [") {
            if arg_index == 5 {
                self.in_actions = true;
            } else if arg_index == 6 {
                self.in_hints = true;
            }
        } else if trimmed == "]" {
            if self.in_actions {
                self.in_actions = false;
                self.state = ParserState::InNotify { arg_index: 6 };
            } else if self.in_hints {
                self.in_hints = false;
                self.state = ParserState::InNotify { arg_index: 7 };
            }
        } else if trimmed.starts_with("int32 ") {
            // Expire timeout encountered -> Notification is complete!
            let desktop_entry = self
                .current_desktop_entry
                .clone()
                .unwrap_or_else(|| self.current_app_name.to_lowercase().replace(' ', "-"));
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let notification = ParsedNotification {
                app_name: std::mem::take(&mut self.current_app_name),
                summary: std::mem::take(&mut self.current_summary),
                body: std::mem::take(&mut self.current_body),
                desktop_entry,
                timestamp_unix: now,
            };

            self.state = ParserState::Idle;
            self.in_actions = false;
            self.in_hints = false;
            self.waiting_desktop_entry_variant = false;

            return Some(notification);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_notification_immediate_emission_and_hint() {
        let sample = r#"
method call time=1793753550.426000 sender=:1.10675 -> destination=:1.8142 serial=9 path=/org/freedesktop/Notifications; interface=org.freedesktop.Notifications; member=Notify
   string "TestApp"
   uint32 0
   string "dialog-information"
   string "Hello Summary"
   string "Line1
Line2"
   array [
   ]
   array [
      dict entry(
         string "image-path"
         variant             string "dialog-information"
      )
      dict entry(
         string "desktop-entry"
         variant             string "firefox"
      )
      dict entry(
         string "urgency"
         variant             byte 1
      )
   ]
   int32 -1
"#;

        let mut parser = DbusNotificationParser::new();
        let mut emitted = Vec::new();

        for line in sample.lines() {
            if let Some(item) = parser.feed_line(line) {
                emitted.push(item);
            }
        }

        // Must emit EXACTLY 1 notification immediately upon seeing int32 -1!
        assert_eq!(
            emitted.len(),
            1,
            "Must emit single notification immediately"
        );
        let notif = &emitted[0];
        assert_eq!(notif.app_name, "TestApp");
        assert_eq!(notif.summary, "Hello Summary");
        assert_eq!(notif.body, "Line1\nLine2");
        assert_eq!(notif.desktop_entry, "firefox");
    }

    #[test]
    fn test_fallback_desktop_entry_when_hint_absent() {
        let sample = r#"
method call time=1793753550.426000 sender=:1.10675 -> destination=:1.8142 serial=9 path=/org/freedesktop/Notifications; interface=org.freedesktop.Notifications; member=Notify
   string "VLC Media Player"
   uint32 0
   string ""
   string "Now Playing"
   string "Artist - Song"
   array [
   ]
   array [
   ]
   int32 5000
"#;

        let mut parser = DbusNotificationParser::new();
        let mut emitted = Vec::new();

        for line in sample.lines() {
            if let Some(item) = parser.feed_line(line) {
                emitted.push(item);
            }
        }

        assert_eq!(emitted.len(), 1);
        let notif = &emitted[0];
        assert_eq!(notif.app_name, "VLC Media Player");
        assert_eq!(notif.summary, "Now Playing");
        assert_eq!(notif.body, "Artist - Song");
        assert_eq!(notif.desktop_entry, "vlc-media-player");
    }
}
