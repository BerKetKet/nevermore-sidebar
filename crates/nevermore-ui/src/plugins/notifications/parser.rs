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
/// Immediately emits on the `int32` line closing the Notify call after hints depth returns to 0.

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
    pending_string: Option<(usize, String)>,
    in_actions: bool,
    in_hints: bool,
    depth: usize,
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

fn is_structural_dbus_token(line: &str) -> bool {
    let t = line.trim();
    t.starts_with("uint32 ")
        || t.starts_with("int32 ")
        || t.starts_with("int64 ")
        || t.starts_with("uint64 ")
        || t.starts_with("double ")
        || t.starts_with("byte ")
        || t.starts_with("boolean ")
        || t.starts_with("string \"")
        || t.starts_with("array [")
        || t.starts_with("array of ")
        || t.starts_with("dict entry(")
        || t.starts_with("variant ")
        || t.starts_with("struct {")
        || t == "]"
        || t == ")"
        || t == "}"
        || t.starts_with("method call ")
        || t.starts_with("signal ")
        || t.starts_with("method return ")
        || t.starts_with("error ")
}

impl DbusNotificationParser {
    pub fn new() -> Self {
        Self::default()
    }

    fn commit_pending_string(&mut self, target_idx: usize, mut content: String) {
        if content.ends_with('"') && !content.ends_with("\\\"") {
            content.pop();
        }

        match target_idx {
            0 => {
                self.current_app_name = content;
                self.state = ParserState::InNotify { arg_index: 1 };
            }
            2 => {
                // app_icon
                self.state = ParserState::InNotify { arg_index: 3 };
            }
            3 => {
                self.current_summary = content;
                self.state = ParserState::InNotify { arg_index: 4 };
            }
            4 => {
                self.current_body = content;
                self.state = ParserState::InNotify { arg_index: 5 };
            }
            _ => {}
        }
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
            self.pending_string = None;
            self.in_actions = false;
            self.in_hints = false;
            self.depth = 0;
            self.waiting_desktop_entry_variant = false;
            return None;
        }

        // If not in a Notify method call, ignore other signals/calls
        let mut arg_index = match self.state {
            ParserState::InNotify { arg_index } => arg_index,
            ParserState::Idle => return None,
        };

        // 2. If we have a pending string, check if this line is a structural token or a continuation
        if let Some((target_idx, mut text)) = self.pending_string.take() {
            if is_structural_dbus_token(line) {
                // The previous string line WAS the end of the string! Commit it now.
                self.commit_pending_string(target_idx, text);
                // Update local arg_index from state
                if let ParserState::InNotify { arg_index: new_idx } = self.state {
                    arg_index = new_idx;
                }
                // Fall through to process `line` below as the new structural element!
            } else {
                // Not a structural token: it is a continuation of the multi-line string
                text.push('\n');
                text.push_str(line.trim_end());
                self.pending_string = Some((target_idx, text));
                return None;
            }
        }

        // 3. Hints parsing and nesting depth tracking
        if self.in_hints {
            let opens = trimmed
                .chars()
                .filter(|&c| c == '[' || c == '{' || c == '(')
                .count();
            let closes = trimmed
                .chars()
                .filter(|&c| c == ']' || c == '}' || c == ')')
                .count();
            self.depth = (self.depth + opens).saturating_sub(closes);

            if self.waiting_desktop_entry_variant {
                if let Some(start) = trimmed.find("string \"") {
                    let rem = &trimmed[start + 8..];
                    if let Some(end) = rem.rfind('"') {
                        self.current_desktop_entry = Some(rem[..end].to_string());
                    }
                    self.waiting_desktop_entry_variant = false;
                }
            } else if trimmed.contains("string \"desktop-entry\"") {
                self.waiting_desktop_entry_variant = true;
            }

            if self.depth == 0 {
                self.in_hints = false;
                self.state = ParserState::InNotify { arg_index: 7 };
            }
            return None;
        }

        // 4. Actions parsing and nesting depth tracking
        if self.in_actions {
            let opens = trimmed
                .chars()
                .filter(|&c| c == '[' || c == '{' || c == '(')
                .count();
            let closes = trimmed
                .chars()
                .filter(|&c| c == ']' || c == '}' || c == ')')
                .count();
            self.depth = (self.depth + opens).saturating_sub(closes);

            if self.depth == 0 {
                self.in_actions = false;
                self.state = ParserState::InNotify { arg_index: 6 };
            }
            return None;
        }

        // 5. Normal argument processing
        if let Some(content_part) = trimmed.strip_prefix("string \"") {
            self.pending_string = Some((arg_index, content_part.trim_end().to_string()));
        } else if trimmed.starts_with("uint32 ") {
            if arg_index == 1 {
                // replaces_id
                self.state = ParserState::InNotify { arg_index: 2 };
            }
        } else if trimmed.starts_with("array [") {
            if arg_index == 5 {
                self.in_actions = true;
                self.depth = 1;
                if trimmed.ends_with(']') {
                    self.depth = 0;
                    self.in_actions = false;
                    self.state = ParserState::InNotify { arg_index: 6 };
                }
            } else if arg_index == 6 {
                self.in_hints = true;
                self.depth = 1;
                if trimmed.ends_with(']') {
                    self.depth = 0;
                    self.in_hints = false;
                    self.state = ParserState::InNotify { arg_index: 7 };
                }
            }
        } else if trimmed.starts_with("int32 ") && arg_index == 7 && self.depth == 0 {
            // Expire timeout encountered ONLY when hints are closed (arg_index == 7 and depth == 0)
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
            self.depth = 0;
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
    fn test_image_data_struct_before_desktop_entry() {
        let sample = r#"
method call time=1793753550.426000 sender=:1.10675 -> destination=org.freedesktop.Notifications serial=9 path=/org/freedesktop/Notifications; interface=org.freedesktop.Notifications; member=Notify
   string "discord"
   uint32 0
   string ""
   string "Alice"
   string "hey, are you online?"
   array [
   ]
   array [
      dict entry(
         string "image-data"
         variant             struct {
               int32 48
               int32 48
               int32 192
               boolean true
               int32 8
               int32 4
               array of bytes [
                  ff 00 00 ff ff 00 00 ff ff 00 00 ff
               ]
            }
      )
      dict entry(
         string "desktop-entry"
         variant             string "com.discordapp.Discord"
      )
   ]
   int32 -1
"#;

        let mut parser = DbusNotificationParser::new();
        let mut emitted = Vec::new();

        for line in sample.lines() {
            if let Some(n) = parser.feed_line(line) {
                emitted.push(n);
            }
        }

        assert_eq!(
            emitted.len(),
            1,
            "Should emit exactly 1 notification on the final int32 -1"
        );
        let n = &emitted[0];
        assert_eq!(n.app_name, "discord");
        assert_eq!(n.summary, "Alice");
        assert_eq!(n.body, "hey, are you online?");
        assert_eq!(n.desktop_entry, "com.discordapp.Discord");
    }

    #[test]
    fn test_image_data_struct_after_desktop_entry() {
        let sample = r#"
method call time=1793753550.426000 sender=:1.10675 -> destination=org.freedesktop.Notifications serial=9 path=/org/freedesktop/Notifications; interface=org.freedesktop.Notifications; member=Notify
   string "discord"
   uint32 0
   string ""
   string "Alice"
   string "hey, are you online?"
   array [
   ]
   array [
      dict entry(
         string "desktop-entry"
         variant             string "com.discordapp.Discord"
      )
      dict entry(
         string "image-data"
         variant             struct {
               int32 48
               int32 48
               int32 192
               boolean true
               int32 8
               int32 4
               array of bytes [
                  ff 00 00 ff ff 00 00 ff ff 00 00 ff
               ]
            }
      )
   ]
   int32 -1
"#;

        let mut parser = DbusNotificationParser::new();
        let mut emitted = Vec::new();

        for line in sample.lines() {
            if let Some(n) = parser.feed_line(line) {
                emitted.push(n);
            }
        }

        assert_eq!(emitted.len(), 1);
        let n = &emitted[0];
        assert_eq!(n.desktop_entry, "com.discordapp.Discord");
    }

    #[test]
    fn test_nested_variant_array_before_desktop_entry() {
        let sample = r#"
method call time=1793753550.426000 sender=:1.10675 -> destination=org.freedesktop.Notifications serial=9 path=/org/freedesktop/Notifications; interface=org.freedesktop.Notifications; member=Notify
   string "discord"
   uint32 0
   string ""
   string "Alice"
   string "hey, are you online?"
   array [
   ]
   array [
      dict entry(
         string "tags"
         variant             array [
               string "a"
               string "b"
            ]
      )
      dict entry(
         string "desktop-entry"
         variant             string "com.discordapp.Discord"
      )
   ]
   int32 -1
"#;

        let mut parser = DbusNotificationParser::new();
        let mut emitted = Vec::new();

        for line in sample.lines() {
            if let Some(n) = parser.feed_line(line) {
                emitted.push(n);
            }
        }

        assert_eq!(emitted.len(), 1);
        let n = &emitted[0];
        assert_eq!(n.desktop_entry, "com.discordapp.Discord");
    }

    #[test]
    fn test_multiline_body_ending_with_quote() {
        let sample = r#"
method call time=1793753550.426000 sender=:1.10675 -> destination=org.freedesktop.Notifications serial=9 path=/org/freedesktop/Notifications; interface=org.freedesktop.Notifications; member=Notify
   string "discord"
   uint32 0
   string ""
   string "Alice"
   string "Line 1 "quoted"
Line 2"
   array [
   ]
   array [
      dict entry(
         string "desktop-entry"
         variant             string "com.discordapp.Discord"
      )
   ]
   int32 -1
"#;

        let mut parser = DbusNotificationParser::new();
        let mut emitted = Vec::new();

        for line in sample.lines() {
            if let Some(n) = parser.feed_line(line) {
                emitted.push(n);
            }
        }

        assert_eq!(emitted.len(), 1);
        let n = &emitted[0];
        assert_eq!(n.summary, "Alice");
        assert_eq!(n.body, "Line 1 \"quoted\"\nLine 2");
        assert_eq!(n.desktop_entry, "com.discordapp.Discord");
    }

    #[test]
    fn test_existing_flat_hints() {
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
            if let Some(n) = parser.feed_line(line) {
                emitted.push(n);
            }
        }

        assert_eq!(emitted.len(), 1);
        let n = &emitted[0];
        assert_eq!(n.app_name, "TestApp");
        assert_eq!(n.summary, "Hello Summary");
        assert_eq!(n.body, "Line1\nLine2");
        assert_eq!(n.desktop_entry, "firefox");
    }
}
