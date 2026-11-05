use crate::sdk::cmd::{Cmd, CmdResult};
use crate::sdk::lifecycle::PluginLifecycle;
use crate::sdk::types::{KeepOnHide, SettingKind, Tx};
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
        && name.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_' || c == '-'
        })
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
    /// Runs sequentially on the main loop. Every message invocation is wrapped in catch_unwind.
    pub fn mailbox<Msg: Send + 'static>(
        &self,
        mut handler: impl FnMut(Msg, &Tx<Msg>) + 'static,
    ) -> Tx<Msg> {
        let (sender, receiver) = async_channel::unbounded::<Msg>();
        let tx = Tx::new(sender);
        let tx_for_handler = tx.clone();
        glib::spawn_future_local(async move {
            while let Ok(msg) = receiver.recv().await {
                let tx_ref = tx_for_handler.clone();
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    handler(msg, &tx_ref);
                }));
            }
        });
        tx
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

    /// Registers a callback invoked whenever the plugin becomes visible.
    pub fn on_show(&self, callback: impl Fn() + 'static) {
        self.lifecycle.borrow_mut().register_on_show(callback);
    }

    /// Registers a callback invoked whenever the plugin becomes hidden.
    pub fn on_hide(&self, callback: impl Fn() + 'static) {
        self.lifecycle.borrow_mut().register_on_hide(callback);
    }

    /// Executes a callback after a specified duration on the local GLib context.
    pub fn after_delay<F: FnOnce() + 'static>(
        &self,
        delay: Duration,
        callback: F,
    ) -> glib::SourceId {
        glib::timeout_add_local_once(delay, callback)
    }

    /// Spawns a background command and posts the mapped result to the mailbox.
    pub fn run<Msg: Send + 'static>(
        &self,
        cmd: Cmd,
        tx: &Tx<Msg>,
        map_res: impl FnOnce(CmdResult) -> Msg + Send + 'static,
    ) {
        let tx = tx.clone();
        std::thread::spawn(move || {
            let res = cmd.execute_blocking();
            tx.send(map_res(res));
        });
    }

    /// Runs a command, streaming its output line-by-line to the mailbox.
    /// Child process is spawned from the calling thread with PR_SET_PDEATHSIG.
    pub fn stream<Msg: Send + 'static>(
        &self,
        cmd: Cmd,
        tx: &Tx<Msg>,
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
        unsafe {
            command.pre_exec(|| {
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL);
                Ok(())
            });
        }
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

        let tx_exit = tx.clone();
        std::thread::spawn(move || {
            let _ = h1.join();
            let _ = h2.join();
            let status = child.wait().ok().and_then(|s| s.code());
            tx_exit.send(on_exit(status));
        });
    }

    /// Spawns a long-running daemon command tracked by the lifecycle.
    /// Spawned from the calling thread with PR_SET_PDEATHSIG.
    pub fn daemon(&self, cmd: Cmd, keep_on_hide: KeepOnHide) {
        use std::os::unix::process::CommandExt;
        use std::process::{Command, Stdio};

        let mut command = Command::new(&cmd.program);
        command.args(&cmd.args);
        for (k, v) in &cmd.envs {
            command.env(k, v);
        }
        command.process_group(0);
        unsafe {
            command.pre_exec(|| {
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL);
                Ok(())
            });
        }
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
    pub fn load_json<T: Default + serde::de::DeserializeOwned>(&self, name: &str) -> T {
        if !is_valid_name(name) {
            eprintln!("[nevermore-plugin] invalid json storage name: '{}'", name);
            return T::default();
        }
        let path = self.data_dir.join(format!("{}.json", name));
        if !path.exists() {
            return T::default();
        }
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[nevermore-plugin] error reading {:?}: {}", path, e);
                return T::default();
            }
        };
        match serde_json::from_str::<T>(&content) {
            Ok(data) => data,
            Err(e) => {
                eprintln!("[nevermore-plugin] corrupted json in {:?}: {}", path, e);
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let bad_path = self.data_dir.join(format!("{}.bad.{}", name, ts));
                let _ = fs::rename(&path, &bad_path);
                T::default()
            }
        }
    }

    /// Saves JSON data atomically via tempfile + fsync + rename.
    pub fn save_json<T: serde::Serialize>(&self, name: &str, data: &T) -> std::io::Result<()> {
        if !is_valid_name(name) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid storage name '{}'", name),
            ));
        }
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let json = serde_json::to_string_pretty(data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let target_path = self.data_dir.join(format!("{}.json", name));
        let tmp_path = self
            .data_dir
            .join(format!(".{}.tmp.{}.{}", name, std::process::id(), seq));

        use std::io::Write;
        let mut file = std::fs::File::create(&tmp_path)?;
        file.write_all(json.as_bytes())?;
        file.sync_all()?;
        drop(file);
        fs::rename(&tmp_path, &target_path)?;
        Ok(())
    }

    /// Asynchronously retrieves a secret from Secret Service via secret-tool over stdin.
    pub fn secret_get<Msg: Send + 'static>(
        &self,
        key: &str,
        tx: &Tx<Msg>,
        map_res: impl FnOnce(Option<String>) -> Msg + Send + 'static,
    ) {
        let plugin_id = self.plugin_id.clone();
        let key_str = key.to_string();
        let tx = tx.clone();
        std::thread::spawn(move || {
            let val = secret_lookup(&plugin_id, &key_str);
            tx.send(map_res(val));
        });
    }

    /// Asynchronously stores a secret in Secret Service via secret-tool strictly over stdin.
    pub fn secret_set<Msg: Send + 'static>(
        &self,
        key: &str,
        val: &str,
        tx: &Tx<Msg>,
        map_res: impl FnOnce(bool) -> Msg + Send + 'static,
    ) {
        let plugin_id = self.plugin_id.clone();
        let key_str = key.to_string();
        let val_str = val.to_string();
        let tx = tx.clone();
        std::thread::spawn(move || {
            let success = secret_store(&plugin_id, &key_str, &val_str).is_ok();
            tx.send(map_res(success));
        });
    }

    /// Synchronous secret lookup helper.
    pub fn secret_get_blocking(&self, key: &str) -> Option<String> {
        secret_lookup(&self.plugin_id, key)
    }

    /// Synchronous secret store helper.
    pub fn secret_set_blocking(&self, key: &str, val: &str) -> std::io::Result<()> {
        secret_store(&self.plugin_id, key, val)
    }

    pub fn plugin_config_path(&self) -> PathBuf {
        if let Some(home) = env::var_os("HOME") {
            PathBuf::from(home)
                .join(".config")
                .join("nevermore")
                .join("plugins")
                .join(format!("{}.toml", self.plugin_id))
        } else {
            self.config_dir.join(format!("{}.toml", self.plugin_id))
        }
    }

    fn read_plugin_toml(&self) -> Option<toml::Table> {
        let p = self.plugin_config_path();
        if p.exists() {
            let s = fs::read_to_string(p).ok()?;
            toml::from_str::<toml::Table>(&s).ok()
        } else {
            None
        }
    }

    pub fn setting_str(&self, key: &str) -> Option<String> {
        if let Some(val) = self
            .read_plugin_toml()
            .and_then(|t| t.get(key).and_then(|v| v.as_str()).map(|s| s.to_string()))
        {
            return Some(val);
        }
        for spec in crate::plugins::ALL_DISCOVERED_SPECS {
            if spec.id == self.plugin_id {
                for setting in spec.settings {
                    if setting.key == key {
                        match setting.kind {
                            SettingKind::Text { default } | SettingKind::Path { default } => {
                                return Some(default.to_string());
                            }
                            SettingKind::Choice { default, .. } => {
                                return Some(default.to_string());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        None
    }

    pub fn setting_bool(&self, key: &str) -> Option<bool> {
        if let Some(val) = self
            .read_plugin_toml()
            .and_then(|t| t.get(key).and_then(|v| v.as_bool()))
        {
            return Some(val);
        }
        for spec in crate::plugins::ALL_DISCOVERED_SPECS {
            if spec.id == self.plugin_id {
                for setting in spec.settings {
                    if setting.key == key {
                        if let SettingKind::Bool { default } = setting.kind {
                            return Some(default);
                        }
                    }
                }
            }
        }
        None
    }

    pub fn setting_int(&self, key: &str) -> Option<i64> {
        if let Some(val) = self
            .read_plugin_toml()
            .and_then(|t| t.get(key).and_then(|v| v.as_integer()))
        {
            return Some(val);
        }
        for spec in crate::plugins::ALL_DISCOVERED_SPECS {
            if spec.id == self.plugin_id {
                for setting in spec.settings {
                    if setting.key == key {
                        if let SettingKind::Int { default, .. } = setting.kind {
                            return Some(default);
                        }
                    }
                }
            }
        }
        None
    }

    pub fn set_setting(&self, key: &str, val: toml::Value) -> std::io::Result<()> {
        let p = self.plugin_config_path();
        if let Some(parent) = p.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut table = self.read_plugin_toml().unwrap_or_default();
        table.insert(key.to_string(), val);
        let s = toml::to_string_pretty(&table)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(p, s)
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

/// Standalone thread-safe secret lookup function using `secret-tool`.
pub fn secret_lookup(plugin_id: &str, key: &str) -> Option<String> {
    let output = std::process::Command::new("secret-tool")
        .args([
            "lookup",
            "service",
            "nevermore",
            "plugin",
            plugin_id,
            "key",
            key,
        ])
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

/// Standalone thread-safe secret store function using `secret-tool`.
pub fn secret_store(plugin_id: &str, key: &str, val: &str) -> std::io::Result<()> {
    use std::io::Write;
    let mut child = std::process::Command::new("secret-tool")
        .args([
            "store",
            &format!("--label=Nevermore {} {}", plugin_id, key),
            "service",
            "nevermore",
            "plugin",
            plugin_id,
            "key",
            key,
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(val.as_bytes())?;
    }

    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other("secret-tool store failed"))
    }
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

        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Default)]
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
        let loaded: TestData = ctx.load_json("test_file");
        assert_eq!(loaded, data);

        // Corrupt file
        let path = ctx.data_dir.join("test_file.json");
        fs::write(&path, b"invalid json content {[").unwrap();

        // Load corrupt -> returns Default and renames to .bad.<ts>
        let corrupt_loaded: TestData = ctx.load_json("test_file");
        assert_eq!(corrupt_loaded, TestData::default());
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
