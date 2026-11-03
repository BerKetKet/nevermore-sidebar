use crate::traits::{AdapterError, WindowRaiser};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// KWin Wayland window raiser implementing external application activation via KWin scripting DBus.
#[derive(Debug, Default, Clone)]
pub struct KWinAdapter;

impl KWinAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl WindowRaiser for KWinAdapter {
    fn raise_application(&self, desktop_entry: &str) -> Result<(), AdapterError> {
        // Validate desktop_entry against ^[A-Za-z0-9_.-]+$
        if desktop_entry.is_empty()
            || !desktop_entry
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-')
        {
            return Err(AdapterError::Compositor(format!(
                "Invalid desktop entry identifier: '{}'",
                desktop_entry
            )));
        }

        let runtime_dir = env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/tmp"));
        let nevermore_dir = runtime_dir.join("nevermore");
        fs::create_dir_all(&nevermore_dir)?;

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let plugin_name = format!("nevermore_raise_{}", ts);
        let script_path = nevermore_dir.join(format!("{}.js", plugin_name));

        let query = desktop_entry.to_lowercase();
        let js_code = format!(
            r#"
const clients = workspace.windowList();
let target = null;
const query = "{query}";
for (let i = 0; i < clients.length; i++) {{
    const c = clients[i];
    const df = (c.desktopFileName || "").toLowerCase();
    const rc = (c.resourceClass || "").toLowerCase();
    const an = (c.applicationName || "").toLowerCase();
    if (df === query || df === query + ".desktop" || rc === query || an === query) {{
        target = c;
        break;
    }}
}}
if (target) {{
    workspace.activeWindow = target;
    console.log("NEVERMORE_RAISE_MATCH_FOUND");
}} else {{
    console.log("NEVERMORE_RAISE_NO_MATCH");
}}
"#
        );

        fs::write(&script_path, js_code)?;

        // Execute via qdbus6 / dbus-send on org.kde.KWin /Scripting
        let load_out = Command::new("qdbus6")
            .arg("org.kde.KWin")
            .arg("/Scripting")
            .arg("org.kde.kwin.Scripting.loadScript")
            .arg(script_path.to_string_lossy().to_string())
            .arg(&plugin_name)
            .output();

        let _ = fs::remove_file(&script_path);

        match load_out {
            Ok(out) if out.status.success() => {
                let script_id = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if let Ok(id_num) = script_id.parse::<i32>() {
                    if id_num >= 0 {
                        let script_obj = format!("/Scripting/Script{}", id_num);
                        let _ = Command::new("qdbus6")
                            .arg("org.kde.KWin")
                            .arg(&script_obj)
                            .arg("org.kde.kwin.Scripting.run")
                            .output();
                        let _ = Command::new("qdbus6")
                            .arg("org.kde.KWin")
                            .arg(&script_obj)
                            .arg("org.kde.kwin.Scripting.stop")
                            .output();
                        let _ = Command::new("qdbus6")
                            .arg("org.kde.KWin")
                            .arg("/Scripting")
                            .arg("org.kde.kwin.Scripting.unloadScript")
                            .arg(&plugin_name)
                            .output();
                        return Ok(());
                    }
                }
                Err(AdapterError::Compositor(format!(
                    "KWin scripting load returned invalid id: '{}'",
                    script_id
                )))
            }
            Ok(out) => Err(AdapterError::Compositor(format!(
                "Failed to load KWin raise script: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ))),
            Err(e) => Err(AdapterError::Io(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_desktop_entry_validation() {
        let adapter = KWinAdapter::new();

        // Invalid characters
        assert!(adapter
            .raise_application("org.kde.konsole; rm -rf /")
            .is_err());
        assert!(adapter.raise_application("bad name with spaces").is_err());
        assert!(adapter.raise_application("").is_err());

        // Valid syntax
        let res = adapter.raise_application("org.kde.nonexistent_window_12345");
        // May fail if KWin is not reachable during headless test, but must not fail on argument validation
        if let Err(AdapterError::Compositor(msg)) = res {
            assert!(!msg.contains("Invalid desktop entry identifier"));
        }
    }
}
