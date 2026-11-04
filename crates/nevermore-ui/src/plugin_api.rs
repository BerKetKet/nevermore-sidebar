use crate::sdk::cmd::{Cmd, CmdResult};
use crate::sdk::lifecycle::PluginLifecycle;
use crate::sdk::types::{KeepOnHide, Tx};
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Overflow, PolicyType, ScrolledWindow, Widget};
use nevermore_core::kwin::KWinAdapter;
use nevermore_core::plugin::PluginManifest;
use nevermore_core::theme::ThemePalette;
use nevermore_core::tmux::TmuxBackend;
use nevermore_core::SystemdTaskBackend;
use std::cell::RefCell;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

pub const PLUGIN_CONTENT_WIDTH: i32 = 340;

fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
}

/// PluginContext: Safe capability struct passed to each plugin.
#[derive(Clone)]
pub struct PluginContext {
    pub plugin_id: String,
    pub data_dir: PathBuf,
    pub config_dir: PathBuf,
    pub lifecycle: Rc<RefCell<PluginLifecycle>>,
    theme: Rc<RefCell<ThemePalette>>,
    pub tmux: TmuxBackend,
    pub systemd: SystemdTaskBackend,
    pub kwin: KWinAdapter,
}

impl PluginContext {
    pub fn new(
        plugin_id: &str,
        theme: Rc<RefCell<ThemePalette>>,
        tmux: TmuxBackend,
        systemd: SystemdTaskBackend,
        kwin: KWinAdapter,
    ) -> Self {
        let data_dir = if let Some(xdg) = env::var_os("XDG_DATA_HOME") {
            PathBuf::from(xdg)
                .join("nevermore")
                .join("plugins")
                .join(plugin_id)
        } else if let Some(home) = env::var_os("HOME") {
            PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("nevermore")
                .join("plugins")
                .join(plugin_id)
        } else {
            PathBuf::from(format!("/tmp/nevermore-data/plugins/{}", plugin_id))
        };

        let config_dir = if let Some(xdg) = env::var_os("XDG_CONFIG_HOME") {
            PathBuf::from(xdg)
                .join("nevermore")
                .join("plugins")
                .join(plugin_id)
        } else if let Some(home) = env::var_os("HOME") {
            PathBuf::from(home)
                .join(".config")
                .join("nevermore")
                .join("plugins")
                .join(plugin_id)
        } else {
            PathBuf::from(format!("/tmp/nevermore-config/plugins/{}", plugin_id))
        };

        let _ = fs::create_dir_all(&data_dir);
        let _ = fs::create_dir_all(&config_dir);

        Self {
            plugin_id: plugin_id.to_string(),
            data_dir,
            config_dir,
            lifecycle: Rc::new(RefCell::new(PluginLifecycle::new())),
            theme,
            tmux,
            systemd,
            kwin,
        }
    }

    /// Read-only getter for the current theme palette.
    pub fn theme(&self) -> ThemePalette {
        self.theme.borrow().clone()
    }

    /// Helper for headless test execution.
    pub fn for_test(temp_dir: &std::path::Path) -> Self {
        let data_dir = temp_dir.join("data");
        let config_dir = temp_dir.join("config");
        let _ = fs::create_dir_all(&data_dir);
        let _ = fs::create_dir_all(&config_dir);
        Self {
            plugin_id: "test".to_string(),
            data_dir,
            config_dir,
            lifecycle: Rc::new(RefCell::new(PluginLifecycle::new())),
            theme: Rc::new(RefCell::new(ThemePalette::default())),
            tmux: TmuxBackend::new("nevermore"),
            systemd: SystemdTaskBackend::new("nevermore"),
            kwin: KWinAdapter::new(),
        }
    }

    /// Typed Mailbox: connects message handler on the UI thread and returns Tx<Msg>.
    pub fn mailbox<Msg: Send + 'static>(&self, handler: impl Fn(Msg) + 'static) -> Tx<Msg> {
        let (sender, receiver) = async_channel::unbounded::<Msg>();
        glib::spawn_future_local(async move {
            while let Ok(msg) = receiver.recv().await {
                handler(msg);
            }
        });
        Tx::new(sender)
    }

    /// Registers a periodic timer that is paused when the plugin tab is hidden.
    pub fn every(&self, interval: Duration, callback: impl Fn() + 'static) {
        self.lifecycle
            .borrow_mut()
            .register_timer(interval, false, callback);
    }

    /// Registers a periodic timer with an explicit KeepOnHide policy.
    pub fn every_with_policy(
        &self,
        interval: Duration,
        keep_on_hide: KeepOnHide,
        callback: impl Fn() + 'static,
    ) {
        self.lifecycle
            .borrow_mut()
            .register_timer(interval, keep_on_hide.0, callback);
    }

    /// Spawns a background command and posts the mapped result to the mailbox.
    pub fn run<Msg: Send + 'static>(
        &self,
        cmd: Cmd,
        tx: Tx<Msg>,
        map_res: impl FnOnce(CmdResult) -> Msg + Send + 'static,
    ) {
        std::thread::spawn(move || {
            let res = cmd.execute_blocking();
            tx.send(map_res(res));
        });
    }

    /// Runs a command, streaming its output line-by-line to the mailbox.
    pub fn stream<Msg: Send + 'static>(
        &self,
        cmd: Cmd,
        tx: Tx<Msg>,
        on_line: impl Fn(String) -> Msg + Send + Sync + 'static,
        on_exit: impl FnOnce(Option<i32>) -> Msg + Send + 'static,
    ) {
        use std::io::{BufRead, BufReader};
        use std::os::unix::process::CommandExt;
        use std::process::{Command, Stdio};

        let mut command = Command::new(&cmd.program);
        command.args(&cmd.args);
        for (k, v) in &cmd.envs {
            command.env(k, v);
        }
        command.process_group(0);
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let mut child = match command.spawn() {
            Ok(c) => c,
            Err(_) => {
                tx.send(on_exit(None));
                return;
            }
        };

        let pid = child.id() as i32;
        self.lifecycle.borrow_mut().register_process(pid, false);

        let on_line = std::sync::Arc::new(on_line);
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let tx_out = tx.clone();
        let on_line_out = on_line.clone();
        let h1 = std::thread::spawn(move || {
            if let Some(out) = stdout {
                let reader = BufReader::new(out);
                for line in reader.lines().map_while(Result::ok) {
                    tx_out.send(on_line_out(line));
                }
            }
        });

        let tx_err = tx.clone();
        let on_line_err = on_line;
        let h2 = std::thread::spawn(move || {
            if let Some(err) = stderr {
                let reader = BufReader::new(err);
                for line in reader.lines().map_while(Result::ok) {
                    tx_err.send(on_line_err(line));
                }
            }
        });

        std::thread::spawn(move || {
            let _ = h1.join();
            let _ = h2.join();
            let status = child.wait().ok().and_then(|s| s.code());
            tx.send(on_exit(status));
        });
    }

    /// Spawns a long-running daemon command tracked by the lifecycle.
    pub fn daemon(&self, cmd: Cmd, keep_on_hide: KeepOnHide) {
        use std::os::unix::process::CommandExt;
        use std::process::{Command, Stdio};

        let mut command = Command::new(&cmd.program);
        command.args(&cmd.args);
        for (k, v) in &cmd.envs {
            command.env(k, v);
        }
        command.process_group(0);
        command.stdout(Stdio::null());
        command.stderr(Stdio::null());

        if let Ok(child) = command.spawn() {
            let pid = child.id() as i32;
            self.lifecycle
                .borrow_mut()
                .register_process(pid, keep_on_hide.0);
        }
    }

    /// Loads JSON data from plugin data directory with corruption recovery.
    pub fn load_json<T: serde::de::DeserializeOwned>(&self, name: &str) -> Option<T> {
        if !is_valid_name(name) {
            eprintln!("[nevermore-plugin] invalid json storage name: '{}'", name);
            return None;
        }
        let path = self.data_dir.join(format!("{}.json", name));
        if !path.exists() {
            return None;
        }
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[nevermore-plugin] error reading {:?}: {}", path, e);
                return None;
            }
        };
        match serde_json::from_str::<T>(&content) {
            Ok(data) => Some(data),
            Err(e) => {
                eprintln!("[nevermore-plugin] corrupted json in {:?}: {}", path, e);
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let bad_path = self.data_dir.join(format!("{}.bad.{}", name, ts));
                let _ = fs::rename(&path, &bad_path);
                None
            }
        }
    }

    /// Saves JSON data atomically via tempfile + rename.
    pub fn save_json<T: serde::Serialize>(&self, name: &str, data: &T) -> std::io::Result<()> {
        if !is_valid_name(name) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid storage name '{}'", name),
            ));
        }
        let json = serde_json::to_string_pretty(data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let target_path = self.data_dir.join(format!("{}.json", name));
        let tmp_path = self
            .data_dir
            .join(format!(".{}.tmp.{}", name, std::process::id()));
        fs::write(&tmp_path, json.as_bytes())?;
        fs::rename(&tmp_path, &target_path)?;
        Ok(())
    }

    /// Retrieves a secret securely from the system Secret Service.
    pub fn secret_get(&self, key: &str) -> Option<String> {
        let output = std::process::Command::new("secret-tool")
            .args(["lookup", "nevermore", &self.plugin_id, "key", key])
            .output()
            .ok()?;
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !s.is_empty() {
                return Some(s);
            }
        }
        None
    }

    /// Stores a secret securely in the system Secret Service.
    pub fn secret_set(&self, key: &str, val: &str) -> std::io::Result<()> {
        let mut child = std::process::Command::new("secret-tool")
            .args([
                "store",
                &format!("--label=Nevermore {} {}", self.plugin_id, key),
                "nevermore",
                &self.plugin_id,
                "key",
                key,
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;

        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin.write_all(val.as_bytes())?;
        }

        let status = child.wait()?;
        if status.success() {
            Ok(())
        } else {
            Err(std::io::Error::other(
                "secret-tool returned non-zero exit status",
            ))
        }
    }

    /// Injects scoped CSS with validation against forbidden literal colors.
    pub fn css(&self, scoped_css: &str) -> Result<(), String> {
        let lower = scoped_css.to_ascii_lowercase();
        let bytes = lower.as_bytes();
        let len = bytes.len();
        let mut i = 0;
        while i < len {
            if bytes[i] == b'#' {
                let mut hex_len = 0;
                let mut j = i + 1;
                while j < len && (bytes[j] as char).is_ascii_hexdigit() {
                    hex_len += 1;
                    j += 1;
                }
                if hex_len == 3 || hex_len == 4 || hex_len == 6 || hex_len == 8 {
                    return Err(
                        "Plugin CSS must not use literal hex colors. Use @theme_* or var(--nm-*) instead."
                            .to_string(),
                    );
                }
            } else if lower[i..].starts_with("rgb(")
                || lower[i..].starts_with("rgba(")
                || lower[i..].starts_with("hsl(")
                || lower[i..].starts_with("hsla(")
            {
                return Err(
                    "Plugin CSS must not use literal rgb/hsl colors. Use @theme_* or var(--nm-*) instead."
                        .to_string(),
                );
            }
            i += 1;
        }

        if gtk4::is_initialized() {
            let scoped = format!(".nm-plugin-{} {{\n{}\n}}", self.plugin_id, scoped_css);
            let provider = gtk4::CssProvider::new();
            provider.load_from_string(&scoped);
            if let Some(display) = gtk4::gdk::Display::default() {
                gtk4::style_context_add_provider_for_display(
                    &display,
                    &provider,
                    gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
                );
            }
        }
        Ok(())
    }

    /// Number of active timers currently running.
    pub fn active_timer_count(&self) -> usize {
        self.lifecycle.borrow().active_timer_count()
    }
}

pub trait SidebarPlugin {
    fn manifest(&self) -> &PluginManifest;
    fn build_widget(&mut self, ctx: &PluginContext) -> Widget;
    fn manages_own_scroll(&self) -> bool {
        false
    }
    fn on_show(&mut self) {}
    fn on_hide(&mut self) {}
    fn on_theme_changed(&mut self, _theme: &ThemePalette) {}
}

/// Helper to wrap every plugin widget in a true 340px-clamped ScrolledWindow container.
pub fn wrap_in_plugin_container(widget: &Widget, manages_own_scroll: bool) -> ScrolledWindow {
    let scrolled = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(if manages_own_scroll {
            PolicyType::Never
        } else {
            PolicyType::Automatic
        })
        .propagate_natural_width(false)
        .min_content_width(PLUGIN_CONTENT_WIDTH)
        .max_content_width(PLUGIN_CONTENT_WIDTH)
        .hexpand(false)
        .vexpand(true)
        .build();
    scrolled.set_size_request(PLUGIN_CONTENT_WIDTH, -1);
    scrolled.set_overflow(Overflow::Hidden);
    scrolled.set_child(Some(widget));
    scrolled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_in_plugin_container_clamp() {
        if gtk4::init().is_err() {
            eprintln!("Skipping container clamp test: no display available");
            return;
        }
        let box_widget = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        box_widget.set_size_request(1000, 50);
        let container = wrap_in_plugin_container(&box_widget.upcast::<Widget>(), false);
        assert_eq!(container.min_content_width(), PLUGIN_CONTENT_WIDTH);
        assert_eq!(container.max_content_width(), PLUGIN_CONTENT_WIDTH);
        assert!(!container.propagates_natural_width());
    }

    #[test]
    fn test_load_save_json_and_corruption() {
        let temp = std::env::temp_dir().join(format!("nevermore_test_json_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let ctx = PluginContext::for_test(&temp);

        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct TestData {
            key: String,
            count: u32,
        }

        let data = TestData {
            key: "hello".to_string(),
            count: 42,
        };

        // Save
        assert!(ctx.save_json("test_file", &data).is_ok());

        // Load
        let loaded: Option<TestData> = ctx.load_json("test_file");
        assert_eq!(loaded, Some(data));

        // Corrupt file
        let path = ctx.data_dir.join("test_file.json");
        fs::write(&path, b"invalid json content {[").unwrap();

        // Load corrupt -> returns None and renames to .bad.<ts>
        let corrupt_loaded: Option<TestData> = ctx.load_json("test_file");
        assert_eq!(corrupt_loaded, None);
        assert!(!path.exists());

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_css_validation() {
        let temp = std::env::temp_dir().join(format!("nevermore_test_css_{}", std::process::id()));
        let ctx = PluginContext::for_test(&temp);

        assert!(ctx.css("label { font-size: 14px; }").is_ok());
        assert!(ctx.css("label { color: var(--nm-text-primary); }").is_ok());
        assert!(ctx.css("label { color: #ffffff; }").is_err());
        assert!(ctx.css("label { color: #123; }").is_err());
        assert!(ctx.css("label { color: rgb(255, 0, 0); }").is_err());
        assert!(ctx.css("label { color: rgba(0, 0, 0, 0.5); }").is_err());
        assert!(ctx.css("label { color: hsl(120, 100%, 50%); }").is_err());

        let _ = fs::remove_dir_all(&temp);
    }
}
