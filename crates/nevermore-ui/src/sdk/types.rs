#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Primary,
    Secondary,
    Muted,
    Accent,
    Danger,
    Success,
    Warning,
}

impl Tone {
    pub fn css_class(&self) -> &'static str {
        match self {
            Tone::Primary => "nm-tone-primary",
            Tone::Secondary => "nm-tone-secondary",
            Tone::Muted => "nm-tone-muted",
            Tone::Accent => "nm-tone-accent",
            Tone::Danger => "nm-tone-danger",
            Tone::Success => "nm-tone-success",
            Tone::Warning => "nm-tone-warning",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeepOnHide(pub bool);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingKind {
    Text {
        default: &'static str,
    },
    Path {
        default: &'static str,
    },
    Int {
        default: i64,
        min: i64,
        max: i64,
    },
    Bool {
        default: bool,
    },
    Choice {
        options: &'static [&'static str],
        default: &'static str,
    },
    Secret {
        key_name: &'static str,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct SettingSpec {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: SettingKind,
}

impl SettingSpec {
    pub const fn new(key: &'static str, label: &'static str, kind: SettingKind) -> Self {
        Self { key, label, kind }
    }
}

#[derive(Clone, Copy)]
pub struct PluginSpec {
    pub id: &'static str,
    pub title: &'static str,
    pub icon: &'static str,
    pub order: u32,
    pub description: &'static str,
    pub settings: &'static [SettingSpec],
    pub settings_custom: Option<fn(&crate::plugin_api::PluginContext) -> gtk4::Widget>,
}

impl std::fmt::Debug for PluginSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginSpec")
            .field("id", &self.id)
            .field("title", &self.title)
            .field("icon", &self.icon)
            .field("order", &self.order)
            .field("description", &self.description)
            .field("settings", &self.settings)
            .field("has_custom_settings", &self.settings_custom.is_some())
            .finish()
    }
}

impl PluginSpec {
    pub const fn new(
        id: &'static str,
        title: &'static str,
        icon: &'static str,
        order: u32,
        description: &'static str,
    ) -> Self {
        Self {
            id,
            title,
            icon,
            order,
            description,
            settings: &[],
            settings_custom: None,
        }
    }

    pub const fn with_settings(
        id: &'static str,
        title: &'static str,
        icon: &'static str,
        order: u32,
        description: &'static str,
        settings: &'static [SettingSpec],
        settings_custom: Option<fn(&crate::plugin_api::PluginContext) -> gtk4::Widget>,
    ) -> Self {
        Self {
            id,
            title,
            icon,
            order,
            description,
            settings,
            settings_custom,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IconSource {
    Named(String),
    Svg(String),
}

pub type Icon = IconSource;

impl IconSource {
    pub fn svg(s: &'static str) -> Self {
        IconSource::Svg(s.to_string())
    }

    pub fn named(name: impl Into<String>) -> Self {
        IconSource::Named(name.into())
    }
}

impl From<&str> for IconSource {
    fn from(s: &str) -> Self {
        let trimmed = s.trim_start();
        if trimmed.starts_with('<') {
            IconSource::Svg(s.to_string())
        } else {
            IconSource::Named(s.to_string())
        }
    }
}

impl From<String> for IconSource {
    fn from(s: String) -> Self {
        let trimmed = s.trim_start();
        if trimmed.starts_with('<') {
            IconSource::Svg(s)
        } else {
            IconSource::Named(s)
        }
    }
}

pub struct Tx<Msg> {
    pub(crate) sender: async_channel::Sender<Msg>,
}

impl<Msg> Clone for Tx<Msg> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

impl<Msg> Tx<Msg> {
    pub fn new(sender: async_channel::Sender<Msg>) -> Self {
        Self { sender }
    }

    pub fn send(&self, msg: Msg) {
        let _ = self.sender.try_send(msg);
    }
}
