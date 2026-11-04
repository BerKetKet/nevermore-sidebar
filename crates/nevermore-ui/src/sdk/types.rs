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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeepOnHide(pub bool);

#[derive(Debug, Clone, Copy)]
pub struct PluginSpec {
    pub id: &'static str,
    pub title: &'static str,
    pub icon: &'static str,
    pub order: u32,
    pub description: &'static str,
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
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IconSource {
    Named(String),
    Svg(String),
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
