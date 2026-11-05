use nevermore_ui::sdk::cmd::Cmd;
use serde::Deserialize;
use std::path::Path;

#[derive(Clone, Debug, PartialEq)]
pub struct LrcLine {
    pub timestamp_sec: Option<f64>,
    pub text: String,
}

#[derive(Deserialize)]
struct LrclibResponse {
    #[serde(rename = "syncedLyrics")]
    synced_lyrics: Option<String>,
    #[serde(rename = "plainLyrics")]
    plain_lyrics: Option<String>,
}

pub fn url_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    for b in input.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => {
                use std::fmt::Write;
                let _ = write!(out, "%{:02X}", b);
            }
        }
    }
    out
}

pub fn parse_lrc(content: &str) -> (Vec<LrcLine>, bool) {
    let mut lines = Vec::new();
    let mut has_synced = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with('[') {
            if let Some(close_idx) = trimmed.find(']') {
                let tag = &trimmed[1..close_idx];
                let text = trimmed[close_idx + 1..].trim().to_string();

                if let Some((min_str, sec_str)) = tag.split_once(':') {
                    if let (Ok(min), Ok(sec)) = (min_str.parse::<f64>(), sec_str.parse::<f64>()) {
                        let timestamp = min * 60.0 + sec;
                        has_synced = true;
                        lines.push(LrcLine {
                            timestamp_sec: Some(timestamp),
                            text,
                        });
                        continue;
                    }
                }
            }
        }

        lines.push(LrcLine {
            timestamp_sec: None,
            text: trimmed.to_string(),
        });
    }

    (lines, has_synced)
}

pub fn find_local_lyrics(music_dir: &Path, song_uri: &str) -> Option<String> {
    let full_path = music_dir.join(song_uri);
    let music_dir_canon = music_dir.canonicalize().ok()?;

    let lrc_path = full_path.with_extension("lrc");
    if lrc_path.exists() {
        if let Ok(canon) = lrc_path.canonicalize() {
            if canon.starts_with(&music_dir_canon) {
                if let Ok(content) = std::fs::read_to_string(canon) {
                    return Some(content);
                }
            }
        }
    }

    let txt_path = full_path.with_extension("txt");
    if txt_path.exists() {
        if let Ok(canon) = txt_path.canonicalize() {
            if canon.starts_with(&music_dir_canon) {
                if let Ok(content) = std::fs::read_to_string(canon) {
                    return Some(content);
                }
            }
        }
    }

    None
}

pub fn fetch_lrclib_lyrics(
    cache_dir: &Path,
    artist: &str,
    title: &str,
    album: &str,
    duration_sec: f64,
) -> Option<String> {
    let _ = std::fs::create_dir_all(cache_dir);
    let key = format!("{}:{}:{}", artist, title, duration_sec as u64);
    let mut hasher = gtk4::glib::Checksum::new(gtk4::glib::ChecksumType::Sha256)?;
    hasher.update(key.as_bytes());
    let hash = hasher.string()?;
    let cache_file = cache_dir.join(format!("{}.json", hash));

    if cache_file.exists() {
        if let Ok(cached) = std::fs::read_to_string(&cache_file) {
            if let Ok(resp) = serde_json::from_str::<LrclibResponse>(&cached) {
                if let Some(synced) = resp.synced_lyrics {
                    if !synced.trim().is_empty() {
                        return Some(synced);
                    }
                }
                if let Some(plain) = resp.plain_lyrics {
                    if !plain.trim().is_empty() {
                        return Some(plain);
                    }
                }
            }
        }
    }

    let url = format!(
        "https://lrclib.net/api/get?artist_name={}&track_name={}&album_name={}&duration={}",
        url_encode(artist),
        url_encode(title),
        url_encode(album),
        duration_sec as u64
    );

    let res = Cmd::new("curl")
        .arg("-s")
        .arg("--connect-timeout")
        .arg("4")
        .arg("-m")
        .arg("8")
        .arg(&url)
        .run_in_worker_thread();

    if res.code == Some(0) && !res.stdout.trim().is_empty() {
        let _ = std::fs::write(&cache_file, &res.stdout);
        if let Ok(resp) = serde_json::from_str::<LrclibResponse>(&res.stdout) {
            if let Some(synced) = resp.synced_lyrics {
                if !synced.trim().is_empty() {
                    return Some(synced);
                }
            }
            if let Some(plain) = resp.plain_lyrics {
                if !plain.trim().is_empty() {
                    return Some(plain);
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_encode() {
        assert_eq!(url_encode("hello world"), "hello+world");
        assert_eq!(url_encode("AC/DC"), "AC%2FDC");
    }

    #[test]
    fn test_parse_lrc() {
        let content = "[00:10.50] Line 1\n[00:20.00] Line 2\nPlain line\n";
        let (lines, has_synced) = parse_lrc(content);
        assert!(has_synced);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].timestamp_sec, Some(10.5));
        assert_eq!(lines[0].text, "Line 1");
        assert_eq!(lines[1].timestamp_sec, Some(20.0));
        assert_eq!(lines[1].text, "Line 2");
        assert_eq!(lines[2].timestamp_sec, None);
        assert_eq!(lines[2].text, "Plain line");
    }
}
