use std::collections::HashSet;

/// Decodes a Base64URL string (RFC 4648 §5) into bytes without external crates.
pub fn base64url_decode(input: &str) -> Option<Vec<u8>> {
    let clean = input.trim();
    if clean.is_empty() {
        return Some(Vec::new());
    }

    // Convert Base64URL characters to standard Base64 characters
    let mut standard = clean.replace('-', "+").replace('_', "/");
    while !standard.len().is_multiple_of(4) {
        standard.push('=');
    }

    let b64_bytes = standard.as_bytes();
    let mut out = Vec::with_capacity((b64_bytes.len() * 3) / 4);

    let decode_char = |c: u8| -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            b'=' => Some(0),
            _ => None,
        }
    };

    let mut i = 0;
    while i < b64_bytes.len() {
        let b0 = decode_char(b64_bytes[i])?;
        let b1 = decode_char(b64_bytes[i + 1])?;
        let b2 = decode_char(b64_bytes[i + 2])?;
        let b3 = decode_char(b64_bytes[i + 3])?;

        out.push((b0 << 2) | (b1 >> 4));
        if b64_bytes[i + 2] != b'=' {
            out.push((b1 << 4) | (b2 >> 2));
        }
        if b64_bytes[i + 3] != b'=' {
            out.push((b2 << 6) | b3);
        }

        i += 4;
    }

    Some(out)
}

/// Encodes bytes into a Base64URL string (RFC 4648 §5) without padding.
pub fn base64url_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity((data.len() * 4).div_ceil(3));

    let mut i = 0;
    while i < data.len() {
        let b0 = data[i];
        let b1 = if i + 1 < data.len() { data[i + 1] } else { 0 };
        let b2 = if i + 2 < data.len() { data[i + 2] } else { 0 };

        let rem = data.len() - i;
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);

        if rem > 1 {
            out.push(TABLE[(((b1 & 0x0F) << 2) | (b2 >> 6)) as usize] as char);
        }
        if rem > 2 {
            out.push(TABLE[(b2 & 0x3F) as usize] as char);
        }

        i += 3;
    }

    out
}

/// Strips HTML tags, styles, scripts, and translates basic entities to plain text.
pub fn strip_html_tags(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_style = false;
    let mut in_script = false;

    let chars: Vec<char> = html.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if !in_tag && i + 7 <= chars.len() {
            let slice: String = chars[i..i + 7].iter().collect();
            if slice.eq_ignore_ascii_case("<style>") || slice.eq_ignore_ascii_case("<style ") {
                in_style = true;
                in_tag = true;
                i += 7;
                continue;
            }
        }
        if in_style && i + 8 <= chars.len() {
            let slice: String = chars[i..i + 8].iter().collect();
            if slice.eq_ignore_ascii_case("</style>") {
                in_style = false;
                in_tag = false;
                i += 8;
                continue;
            }
        }
        if !in_tag && i + 8 <= chars.len() {
            let slice: String = chars[i..i + 8].iter().collect();
            if slice.eq_ignore_ascii_case("<script>") || slice.eq_ignore_ascii_case("<script ") {
                in_script = true;
                in_tag = true;
                i += 8;
                continue;
            }
        }
        if in_script && i + 9 <= chars.len() {
            let slice: String = chars[i..i + 9].iter().collect();
            if slice.eq_ignore_ascii_case("</script>") {
                in_script = false;
                in_tag = false;
                i += 9;
                continue;
            }
        }

        if in_style || in_script {
            i += 1;
            continue;
        }

        let c = chars[i];
        if c == '<' {
            in_tag = true;
            // Check for line-breaking tags
            let rem: String = chars[i..chars.len().min(i + 6)].iter().collect();
            let rem_lower = rem.to_ascii_lowercase();
            if rem_lower.starts_with("<br")
                || rem_lower.starts_with("</p")
                || rem_lower.starts_with("</div")
                || rem_lower.starts_with("</tr")
                || rem_lower.starts_with("</li")
            {
                text.push('\n');
            }
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            text.push(c);
        }
        i += 1;
    }

    // Decode entities
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Extracts all unique http/https URLs from text.
pub fn extract_urls(text: &str) -> Vec<String> {
    let mut urls = Vec::new();
    let mut seen = HashSet::new();

    for word in text.split_whitespace() {
        let clean = word
            .trim_matches(|c: char| c == '(' || c == ')' || c == '[' || c == ']' || c == '<' || c == '>' || c == ',' || c == '"' || c == '\'');
        if (clean.starts_with("http://") || clean.starts_with("https://"))
            && clean.len() > 10
            && seen.insert(clean.to_string())
        {
            urls.push(clean.to_string());
        }
    }

    urls
}

#[derive(Clone, Debug, Default)]
pub struct ParsedMessage {
    pub id: String,
    pub thread_id: String,
    pub message_id: String,
    pub subject: String,
    pub from: String,
    pub to: String,
    pub reply_to: String,
    pub date: String,
    pub references: String,
    pub body_text: String,
    pub urls: Vec<String>,
}

fn extract_body_from_payload(payload: &serde_json::Value) -> (Option<String>, Option<String>) {
    let mut plain_text: Option<String> = None;
    let mut html_text: Option<String> = None;

    let mime_type = payload["mimeType"].as_str().unwrap_or("");
    if let Some(data) = payload["body"]["data"].as_str() {
        if let Some(bytes) = base64url_decode(data) {
            let decoded = String::from_utf8_lossy(&bytes).to_string();
            if mime_type.eq_ignore_ascii_case("text/plain") {
                plain_text = Some(decoded);
            } else if mime_type.eq_ignore_ascii_case("text/html") {
                html_text = Some(decoded);
            }
        }
    }

    if let Some(parts) = payload["parts"].as_array() {
        for part in parts {
            let (sub_plain, sub_html) = extract_body_from_payload(part);
            if plain_text.is_none() && sub_plain.is_some() {
                plain_text = sub_plain;
            }
            if html_text.is_none() && sub_html.is_some() {
                html_text = sub_html;
            }
            if plain_text.is_some() {
                break;
            }
        }
    }

    (plain_text, html_text)
}

pub fn parse_full_message(json: &serde_json::Value) -> ParsedMessage {
    let mut msg = ParsedMessage::default();
    msg.id = json["id"].as_str().unwrap_or("").to_string();
    msg.thread_id = json["threadId"].as_str().unwrap_or(&msg.id).to_string();

    let payload = &json["payload"];
    if let Some(headers) = payload["headers"].as_array() {
        for h in headers {
            let name = h["name"].as_str().unwrap_or("");
            let val = h["value"].as_str().unwrap_or("");
            if name.eq_ignore_ascii_case("Subject") {
                msg.subject = val.to_string();
            } else if name.eq_ignore_ascii_case("From") {
                msg.from = val.to_string();
            } else if name.eq_ignore_ascii_case("To") {
                msg.to = val.to_string();
            } else if name.eq_ignore_ascii_case("Reply-To") {
                msg.reply_to = val.to_string();
            } else if name.eq_ignore_ascii_case("Date") {
                msg.date = val.to_string();
            } else if name.eq_ignore_ascii_case("Message-ID") {
                msg.message_id = val.to_string();
            } else if name.eq_ignore_ascii_case("References") {
                msg.references = val.to_string();
            }
        }
    }

    let (plain, html) = extract_body_from_payload(payload);
    let raw_body = if let Some(p) = plain {
        p
    } else if let Some(h) = html {
        strip_html_tags(&h)
    } else {
        json["snippet"].as_str().unwrap_or("").to_string()
    };

    msg.urls = extract_urls(&raw_body);
    msg.body_text = raw_body;

    msg
}

/// Constructs a valid RFC 2822 email string and Base64URL-encodes it.
pub fn build_rfc2822_reply(
    to: &str,
    subject: &str,
    in_reply_to: &str,
    references: &str,
    body: &str,
) -> String {
    let reply_subject = if subject.to_ascii_lowercase().starts_with("re:") {
        subject.to_string()
    } else {
        format!("Re: {}", subject)
    };

    let refs = if references.is_empty() {
        in_reply_to.to_string()
    } else if in_reply_to.is_empty() {
        references.to_string()
    } else {
        format!("{} {}", references, in_reply_to)
    };

    let mut rfc = format!("To: {}\r\nSubject: {}\r\n", to, reply_subject);
    if !in_reply_to.is_empty() {
        rfc.push_str(&format!("In-Reply-To: {}\r\n", in_reply_to));
    }
    if !refs.is_empty() {
        rfc.push_str(&format!("References: {}\r\n", refs));
    }
    rfc.push_str("Content-Type: text/plain; charset=\"UTF-8\"\r\n\r\n");
    rfc.push_str(body);

    base64url_encode(rfc.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64url_roundtrip() {
        let text = "Hello Nevermore Sidebar! Base64URL safe testing ~!@#$%^&*()_+";
        let enc = base64url_encode(text.as_bytes());
        assert!(!enc.contains('+'));
        assert!(!enc.contains('/'));
        assert!(!enc.contains('='));

        let dec = base64url_decode(&enc).expect("decode failed");
        assert_eq!(String::from_utf8(dec).unwrap(), text);
    }

    #[test]
    fn test_strip_html_tags() {
        let html = "<html><head><style>body { color: red; }</style></head><body><h1>Title</h1><p>Hello world!<br/>Second line.</p><script>alert('xss');</script></body></html>";
        let text = strip_html_tags(html);
        assert!(!text.contains("color: red"));
        assert!(!text.contains("alert"));
        assert!(text.contains("Title"));
        assert!(text.contains("Hello world!"));
        assert!(text.contains("Second line."));
    }

    #[test]
    fn test_extract_urls() {
        let text = "Visit our site at https://nevermore.dev/docs or http://example.com/test?a=1&b=2 for info.";
        let urls = extract_urls(text);
        assert_eq!(urls.len(), 2);
        assert!(urls.contains(&"https://nevermore.dev/docs".to_string()));
        assert!(urls.contains(&"http://example.com/test?a=1&b=2".to_string()));
    }
}
