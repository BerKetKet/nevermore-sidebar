use std::fmt;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(2);
pub const MAX_LINE_LEN: usize = 65536; // 64 KB cap
pub const MAX_BINARY_LEN: usize = 8 * 1024 * 1024; // 8 MB cap

#[derive(Debug)]
pub enum MpdError {
    Io(std::io::Error),
    ConnectionFailed(String),
    Protocol(String),
    Ack(String),
    LineTooLong,
    BinaryTooLarge,
    InvalidArgument(String),
}

impl fmt::Display for MpdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MpdError::Io(e) => write!(f, "I/O error: {}", e),
            MpdError::ConnectionFailed(msg) => write!(f, "MPD connection failed: {}", msg),
            MpdError::Protocol(msg) => write!(f, "MPD protocol error: {}", msg),
            MpdError::Ack(msg) => write!(f, "MPD ACK error: {}", msg),
            MpdError::LineTooLong => write!(f, "MPD response line exceeded 64KB"),
            MpdError::BinaryTooLarge => write!(f, "MPD binary payload exceeded 8MB"),
            MpdError::InvalidArgument(msg) => write!(f, "Invalid argument: {}", msg),
        }
    }
}

impl std::error::Error for MpdError {}

impl From<std::io::Error> for MpdError {
    fn from(e: std::io::Error) -> Self {
        MpdError::Io(e)
    }
}

#[derive(Clone, Debug, Default)]
pub enum MpdEndpoint {
    Unix(PathBuf),
    Tcp(SocketAddr),
    #[default]
    Auto,
}

pub enum MpdStream {
    Unix(UnixStream),
    Tcp(TcpStream),
}

impl Read for MpdStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            MpdStream::Unix(s) => s.read(buf),
            MpdStream::Tcp(s) => s.read(buf),
        }
    }
}

impl Write for MpdStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            MpdStream::Unix(s) => s.write(buf),
            MpdStream::Tcp(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            MpdStream::Unix(s) => s.flush(),
            MpdStream::Tcp(s) => s.flush(),
        }
    }
}

impl MpdStream {
    pub fn shutdown(&self, how: Shutdown) -> std::io::Result<()> {
        match self {
            MpdStream::Unix(s) => s.shutdown(how),
            MpdStream::Tcp(s) => s.shutdown(how),
        }
    }

    pub fn set_read_timeout(&self, dur: Option<Duration>) -> std::io::Result<()> {
        match self {
            MpdStream::Unix(s) => s.set_read_timeout(dur),
            MpdStream::Tcp(s) => s.set_read_timeout(dur),
        }
    }

    pub fn set_write_timeout(&self, dur: Option<Duration>) -> std::io::Result<()> {
        match self {
            MpdStream::Unix(s) => s.set_write_timeout(dur),
            MpdStream::Tcp(s) => s.set_write_timeout(dur),
        }
    }

    pub fn try_clone(&self) -> std::io::Result<Self> {
        match self {
            MpdStream::Unix(s) => s.try_clone().map(MpdStream::Unix),
            MpdStream::Tcp(s) => s.try_clone().map(MpdStream::Tcp),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PlayState {
    #[default]
    Stopped,
    Playing,
    Paused,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Status {
    pub state: PlayState,
    pub elapsed_sec: f64,
    pub duration_sec: f64,
    pub volume: i32, // 0..100 or -1 if unavailable
    pub song_id: Option<u32>,
    pub song_pos: Option<u32>,
    pub playlist_version: u32,
    pub repeat: bool,
    pub random: bool,
    pub single: bool,
    pub consume: bool,
}

impl Status {
    pub fn from_kvs(kvs: &[(String, String)]) -> Self {
        let mut status = Status {
            volume: -1,
            ..Status::default()
        };

        for (k, v) in kvs {
            match k.as_str() {
                "state" => {
                    status.state = match v.as_str() {
                        "play" => PlayState::Playing,
                        "pause" => PlayState::Paused,
                        _ => PlayState::Stopped,
                    };
                }
                "elapsed" => {
                    status.elapsed_sec = v.parse().unwrap_or(0.0);
                }
                "duration" => {
                    status.duration_sec = v.parse().unwrap_or(0.0);
                }
                "time" => {
                    // time format: "elapsed:duration"
                    if status.duration_sec == 0.0 {
                        let parts: Vec<&str> = v.split(':').collect();
                        if parts.len() == 2 {
                            status.elapsed_sec = parts[0].parse().unwrap_or(0.0);
                            status.duration_sec = parts[1].parse().unwrap_or(0.0);
                        }
                    }
                }
                "volume" => {
                    status.volume = v.parse().unwrap_or(-1);
                }
                "songid" => {
                    status.song_id = v.parse().ok();
                }
                "song" => {
                    status.song_pos = v.parse().ok();
                }
                "playlist" => {
                    status.playlist_version = v.parse().unwrap_or(0);
                }
                "repeat" => {
                    status.repeat = v == "1";
                }
                "random" => {
                    status.random = v == "1";
                }
                "single" => {
                    status.single = v == "1";
                }
                "consume" => {
                    status.consume = v == "1";
                }
                _ => {}
            }
        }
        status
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Song {
    pub uri: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub track: Option<String>,
    pub date: Option<String>,
    pub duration_sec: f64,
}

impl Song {
    pub fn from_kvs(kvs: &[(String, String)]) -> Self {
        let mut song = Song::default();
        for (k, v) in kvs {
            match k.to_lowercase().as_str() {
                "file" => song.uri = v.clone(),
                "title" => song.title = Some(v.clone()),
                "artist" => song.artist = Some(v.clone()),
                "album" => song.album = Some(v.clone()),
                "albumartist" => song.album_artist = Some(v.clone()),
                "track" => song.track = Some(v.clone()),
                "date" => song.date = Some(v.clone()),
                "duration" | "time" if song.duration_sec == 0.0 => {
                    song.duration_sec = v.parse().unwrap_or(0.0);
                }
                _ => {}
            }
        }
        song
    }
}

pub fn quote_arg(arg: &str) -> Result<String, MpdError> {
    if arg.contains('\n') || arg.contains('\r') {
        return Err(MpdError::InvalidArgument(
            "MPD argument must not contain newlines".into(),
        ));
    }
    let mut out = String::with_capacity(arg.len() + 2);
    out.push('"');
    for ch in arg.chars() {
        if ch == '\\' || ch == '"' {
            out.push('\\');
        }
        out.push(ch);
    }
    out.push('"');
    Ok(out)
}

pub struct MpdClient {
    reader: BufReader<MpdStream>,
    endpoint: MpdEndpoint,
    version: String,
}

impl MpdClient {
    pub fn connect(endpoint: &MpdEndpoint) -> Result<Self, MpdError> {
        let stream = match endpoint {
            MpdEndpoint::Unix(path) => {
                let s = UnixStream::connect(path).map_err(|e| {
                    MpdError::ConnectionFailed(format!("Failed to connect to {:?}: {}", path, e))
                })?;
                MpdStream::Unix(s)
            }
            MpdEndpoint::Tcp(addr) => {
                let s = TcpStream::connect_timeout(addr, DEFAULT_TIMEOUT).map_err(|e| {
                    MpdError::ConnectionFailed(format!("Failed to connect to {}: {}", addr, e))
                })?;
                MpdStream::Tcp(s)
            }
            MpdEndpoint::Auto => {
                // Try ~/.config/mpd/socket
                let user_socket = dirs_home_socket();
                if let Some(path) = &user_socket {
                    if path.exists() {
                        if let Ok(s) = UnixStream::connect(path) {
                            MpdStream::Unix(s)
                        } else {
                            Self::connect_auto_fallback()?
                        }
                    } else {
                        Self::connect_auto_fallback()?
                    }
                } else {
                    Self::connect_auto_fallback()?
                }
            }
        };

        stream.set_read_timeout(Some(DEFAULT_TIMEOUT))?;
        stream.set_write_timeout(Some(DEFAULT_TIMEOUT))?;

        let mut reader = BufReader::new(stream);
        let mut greeting = String::new();
        reader.read_line(&mut greeting)?;

        if !greeting.starts_with("OK MPD ") {
            return Err(MpdError::Protocol(format!(
                "Invalid MPD greeting: {}",
                greeting.trim()
            )));
        }

        let version = greeting
            .trim_start_matches("OK MPD ")
            .trim()
            .to_string();

        Ok(Self {
            reader,
            endpoint: endpoint.clone(),
            version,
        })
    }

    fn connect_auto_fallback() -> Result<MpdStream, MpdError> {
        // Try $XDG_RUNTIME_DIR/mpd/socket
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            let path = PathBuf::from(runtime_dir).join("mpd").join("socket");
            if path.exists() {
                if let Ok(s) = UnixStream::connect(&path) {
                    return Ok(MpdStream::Unix(s));
                }
            }
        }

        // Try TCP 127.0.0.1:6600
        let addr: SocketAddr = "127.0.0.1:6600".parse().unwrap();
        TcpStream::connect_timeout(&addr, DEFAULT_TIMEOUT)
            .map(MpdStream::Tcp)
            .map_err(|e| {
                MpdError::ConnectionFailed(format!(
                    "Failed to connect to MPD via unix socket or 127.0.0.1:6600: {}",
                    e
                ))
            })
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn endpoint(&self) -> &MpdEndpoint {
        &self.endpoint
    }

    pub fn send_cmd(&mut self, cmd: &str, args: &[&str]) -> Result<(), MpdError> {
        let mut line = cmd.to_string();
        for arg in args {
            line.push(' ');
            line.push_str(&quote_arg(arg)?);
        }
        line.push('\n');

        self.reader.get_mut().write_all(line.as_bytes())?;
        self.reader.get_mut().flush()?;
        Ok(())
    }

    pub fn read_response(&mut self) -> Result<Vec<(String, String)>, MpdError> {
        let mut kvs = Vec::new();
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = self.reader.read_line(&mut line)?;
            if bytes_read == 0 {
                return Err(MpdError::Protocol("Unexpected EOF from MPD".into()));
            }
            if line.len() > MAX_LINE_LEN {
                return Err(MpdError::LineTooLong);
            }

            let trimmed = line.trim_end_matches(['\r', '\n']);
            if trimmed == "OK" {
                break;
            }
            if trimmed.starts_with("ACK ") {
                return Err(MpdError::Ack(trimmed.to_string()));
            }

            if let Some((k, v)) = trimmed.split_once(": ") {
                kvs.push((k.to_string(), v.to_string()));
            }
        }

        Ok(kvs)
    }

    pub fn status(&mut self) -> Result<Status, MpdError> {
        self.send_cmd("status", &[])?;
        let kvs = self.read_response()?;
        Ok(Status::from_kvs(&kvs))
    }

    pub fn current_song(&mut self) -> Result<Option<Song>, MpdError> {
        self.send_cmd("currentsong", &[])?;
        let kvs = self.read_response()?;
        if kvs.is_empty() {
            Ok(None)
        } else {
            let song = Song::from_kvs(&kvs);
            if song.uri.is_empty() {
                Ok(None)
            } else {
                Ok(Some(song))
            }
        }
    }

    pub fn play(&mut self) -> Result<(), MpdError> {
        self.send_cmd("play", &[])?;
        self.read_response()?;
        Ok(())
    }

    pub fn pause(&mut self, pause: bool) -> Result<(), MpdError> {
        self.send_cmd("pause", &[if pause { "1" } else { "0" }])?;
        self.read_response()?;
        Ok(())
    }

    pub fn play_pause(&mut self) -> Result<(), MpdError> {
        let status = self.status()?;
        match status.state {
            PlayState::Playing => self.pause(true),
            PlayState::Paused => self.pause(false),
            PlayState::Stopped => self.play(),
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<(), MpdError> {
        self.send_cmd("next", &[])?;
        self.read_response()?;
        Ok(())
    }

    pub fn prev(&mut self) -> Result<(), MpdError> {
        self.send_cmd("previous", &[])?;
        self.read_response()?;
        Ok(())
    }

    pub fn seek_cur(&mut self, secs: f64) -> Result<(), MpdError> {
        let arg = format!("{:.2}", secs.max(0.0));
        self.send_cmd("seekcur", &[&arg])?;
        self.read_response()?;
        Ok(())
    }

    pub fn set_volume(&mut self, vol: i32) -> Result<(), MpdError> {
        let clamped = vol.clamp(0, 100);
        let arg = clamped.to_string();
        self.send_cmd("setvol", &[&arg])?;
        self.read_response()?;
        Ok(())
    }

    fn read_picture_chunk(
        &mut self,
        cmd: &str,
        uri: &str,
        offset: usize,
    ) -> Result<Option<(Vec<u8>, usize, String)>, MpdError> {
        let off_str = offset.to_string();
        self.send_cmd(cmd, &[uri, &off_str])?;

        let mut size = 0usize;
        let binary;
        let mut mime = "image/jpeg".to_string();
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = self.reader.read_line(&mut line)?;
            if bytes_read == 0 {
                return Err(MpdError::Protocol("Unexpected EOF from MPD during picture".into()));
            }

            let trimmed = line.trim_end_matches(['\r', '\n']);
            if trimmed == "OK" {
                // If OK appears before binary: 0 bytes available
                return Ok(None);
            }
            if trimmed.starts_with("ACK ") {
                return Ok(None);
            }

            if let Some((k, v)) = trimmed.split_once(": ") {
                match k {
                    "size" => size = v.parse().unwrap_or(0),
                    "type" => mime = v.to_string(),
                    "binary" => {
                        binary = v.parse().unwrap_or(0);
                        break;
                    }
                    _ => {}
                }
            }
        }

        if binary == 0 {
            // Read trailing OK
            line.clear();
            self.reader.read_line(&mut line)?;
            return Ok(None);
        }

        if binary > MAX_BINARY_LEN {
            return Err(MpdError::BinaryTooLarge);
        }

        let mut buf = vec![0u8; binary];
        self.reader.read_exact(&mut buf)?;

        // MPD sends a single trailing newline after binary data, then OK
        let mut trailing = [0u8; 1];
        self.reader.read_exact(&mut trailing)?;

        line.clear();
        self.reader.read_line(&mut line)?;
        if line.trim() != "OK" {
            return Err(MpdError::Protocol(format!(
                "Expected OK after binary chunk, got: {}",
                line.trim()
            )));
        }

        Ok(Some((buf, size, mime)))
    }

    /// Fetches picture data (trying readpicture first, then albumart).
    pub fn get_picture(&mut self, uri: &str) -> Result<Option<(Vec<u8>, String)>, MpdError> {
        if uri.is_empty() {
            return Ok(None);
        }

        // 1. Try readpicture (embedded tag art)
        if let Some(res) = self.fetch_all_picture_chunks("readpicture", uri)? {
            return Ok(Some(res));
        }

        // 2. Try albumart (cover file in directory)
        if let Some(res) = self.fetch_all_picture_chunks("albumart", uri)? {
            return Ok(Some(res));
        }

        Ok(None)
    }

    fn fetch_all_picture_chunks(
        &mut self,
        cmd: &str,
        uri: &str,
    ) -> Result<Option<(Vec<u8>, String)>, MpdError> {
        let first = self.read_picture_chunk(cmd, uri, 0)?;
        let (first_bytes, total_size, mime_type) = match first {
            Some(data) => data,
            None => return Ok(None),
        };

        if total_size > MAX_BINARY_LEN {
            return Err(MpdError::BinaryTooLarge);
        }

        let mut total_bytes = Vec::with_capacity(total_size);
        total_bytes.extend(first_bytes);

        while total_bytes.len() < total_size {
            let offset = total_bytes.len();
            match self.read_picture_chunk(cmd, uri, offset)? {
                Some((chunk, _, _)) => {
                    if chunk.is_empty() {
                        break;
                    }
                    total_bytes.extend(chunk);
                }
                None => break,
            }
        }

        if total_bytes.is_empty() {
            Ok(None)
        } else {
            Ok(Some((total_bytes, mime_type)))
        }
    }
}

fn dirs_home_socket() -> Option<PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".config").join("mpd").join("socket"))
}

pub struct IdleHandle {
    stream: Option<MpdStream>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl IdleHandle {
    pub fn stop(&mut self) {
        if let Some(stream) = self.stream.take() {
            let _ = stream.shutdown(Shutdown::Both);
        }
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for IdleHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn start_idle(
    endpoint: MpdEndpoint,
    subsystems: &[&str],
) -> (async_channel::Receiver<Vec<String>>, IdleHandle) {
    let (tx, rx) = async_channel::unbounded();

    let client_res = MpdClient::connect(&endpoint);
    let mut client = match client_res {
        Ok(c) => c,
        Err(_) => {
            return (
                rx,
                IdleHandle {
                    stream: None,
                    thread: None,
                },
            );
        }
    };

    // Remove timeout on idle connection so it can block indefinitely
    let _ = client.reader.get_mut().set_read_timeout(None);

    let stream_clone = client.reader.get_mut().try_clone().ok();
    let sub_cmd = if subsystems.is_empty() {
        "idle\n".to_string()
    } else {
        format!("idle {}\n", subsystems.join(" "))
    };

    let thread = std::thread::spawn(move || {
        loop {
            if client.reader.get_mut().write_all(sub_cmd.as_bytes()).is_err() {
                break;
            }
            if client.reader.get_mut().flush().is_err() {
                break;
            }

            let mut events = Vec::new();
            let mut line = String::new();
            let ok_received;

            loop {
                line.clear();
                if client.reader.read_line(&mut line).unwrap_or(0) == 0 {
                    return;
                }

                let trimmed = line.trim_end_matches(['\r', '\n']);
                if trimmed == "OK" {
                    ok_received = true;
                    break;
                }
                if trimmed.starts_with("ACK ") {
                    return;
                }

                if let Some((k, v)) = trimmed.split_once(": ") {
                    if k == "changed" {
                        events.push(v.to_string());
                    }
                }
            }

            if ok_received && !events.is_empty() && tx.send_blocking(events).is_err() {
                break;
            }
        }
    });

    (
        rx,
        IdleHandle {
            stream: stream_clone,
            thread: Some(thread),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::os::unix::net::UnixListener;

    #[test]
    fn test_quote_arg() {
        assert_eq!(quote_arg("simple").unwrap(), "\"simple\"");
        assert_eq!(
            quote_arg("file with \"quotes\" and \\slash").unwrap(),
            "\"file with \\\"quotes\\\" and \\\\slash\""
        );
        assert!(quote_arg("bad\nnewline").is_err());
        assert!(quote_arg("bad\rcarriage").is_err());
    }

    #[test]
    fn test_parse_status() {
        let kvs = vec![
            ("volume".to_string(), "85".to_string()),
            ("repeat".to_string(), "1".to_string()),
            ("random".to_string(), "0".to_string()),
            ("state".to_string(), "play".to_string()),
            ("songid".to_string(), "42".to_string()),
            ("elapsed".to_string(), "123.45".to_string()),
            ("duration".to_string(), "300.0".to_string()),
        ];
        let status = Status::from_kvs(&kvs);
        assert_eq!(status.volume, 85);
        assert!(status.repeat);
        assert!(!status.random);
        assert_eq!(status.state, PlayState::Playing);
        assert_eq!(status.song_id, Some(42));
        assert_eq!(status.elapsed_sec, 123.45);
        assert_eq!(status.duration_sec, 300.0);
    }

    #[test]
    fn test_parse_song() {
        let kvs = vec![
            ("file".to_string(), "Artist/Album/01 - Track.flac".to_string()),
            ("Title".to_string(), "Test Title".to_string()),
            ("Artist".to_string(), "Test Artist".to_string()),
            ("Album".to_string(), "Test Album".to_string()),
            ("Duration".to_string(), "215.5".to_string()),
        ];
        let song = Song::from_kvs(&kvs);
        assert_eq!(song.uri, "Artist/Album/01 - Track.flac");
        assert_eq!(song.title.as_deref(), Some("Test Title"));
        assert_eq!(song.artist.as_deref(), Some("Test Artist"));
        assert_eq!(song.album.as_deref(), Some("Test Album"));
        assert_eq!(song.duration_sec, 215.5);
    }

    #[test]
    fn test_fake_mpd_server_roundtrip() {
        let temp_dir = std::env::temp_dir();
        let sock_path = temp_dir.join(format!("test_mpd_{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&sock_path);

        let listener = UnixListener::bind(&sock_path).expect("Failed to bind test socket");

        let server_thread = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("Accept failed");
            // 1. Send greeting
            stream.write_all(b"OK MPD 0.23.5\n").unwrap();

            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();

            // 2. Expect status command
            reader.read_line(&mut line).unwrap();
            assert_eq!(line.trim(), "status");

            // 3. Send status response
            stream
                .write_all(b"volume: 70\nstate: play\nelapsed: 50.0\nduration: 200.0\nOK\n")
                .unwrap();

            // 4. Expect currentsong command
            line.clear();
            reader.read_line(&mut line).unwrap();
            assert_eq!(line.trim(), "currentsong");

            // 5. Send currentsong response
            stream
                .write_all(b"file: Songs/track.mp3\nTitle: Mock Song\nArtist: Mock Band\nOK\n")
                .unwrap();
        });

        let mut client = MpdClient::connect(&MpdEndpoint::Unix(sock_path.clone()))
            .expect("Failed to connect to fake MPD");
        assert_eq!(client.version(), "0.23.5");

        let status = client.status().expect("Status failed");
        assert_eq!(status.volume, 70);
        assert_eq!(status.state, PlayState::Playing);
        assert_eq!(status.elapsed_sec, 50.0);
        assert_eq!(status.duration_sec, 200.0);

        let song = client.current_song().expect("Current song failed").unwrap();
        assert_eq!(song.uri, "Songs/track.mp3");
        assert_eq!(song.title.as_deref(), Some("Mock Song"));
        assert_eq!(song.artist.as_deref(), Some("Mock Band"));

        server_thread.join().unwrap();
        let _ = std::fs::remove_file(&sock_path);
    }
}
