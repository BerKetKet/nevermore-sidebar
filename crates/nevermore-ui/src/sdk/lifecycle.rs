use crate::plugin_api::{PluginContext, SidebarPlugin};
use crate::sdk::types::PluginSpec;
use gtk4::glib;
use nevermore_core::plugin::PluginManifest;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

pub fn safe_source_remove(id: glib::SourceId) {
    if glib::MainContext::default()
        .find_source_by_id(&id)
        .is_some()
    {
        id.remove();
    }
}

pub struct PluginTimer {
    pub interval: Duration,
    pub callback: Rc<dyn Fn()>,
    pub keep_on_hide: bool,
    pub source_id: Option<glib::SourceId>,
}

pub struct ChildProc {
    pub pid: i32,
    pub keep_on_hide: bool,
}

#[derive(Default)]
pub struct PluginLifecycle {
    pub timers: Vec<PluginTimer>,
    pub processes: Vec<ChildProc>,
    pub on_show_hooks: Vec<Rc<dyn Fn()>>,
    pub on_hide_hooks: Vec<Rc<dyn Fn()>>,
}

impl PluginLifecycle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_timer(
        &mut self,
        interval: Duration,
        keep_on_hide: bool,
        callback: impl Fn() + 'static,
    ) {
        self.timers.push(PluginTimer {
            interval,
            callback: Rc::new(callback),
            keep_on_hide,
            source_id: None,
        });
    }

    pub fn register_process(&mut self, pid: i32, keep_on_hide: bool) {
        self.processes.push(ChildProc { pid, keep_on_hide });
    }

    pub fn register_on_show(&mut self, callback: impl Fn() + 'static) {
        self.on_show_hooks.push(Rc::new(callback));
    }

    pub fn register_on_hide(&mut self, callback: impl Fn() + 'static) {
        self.on_hide_hooks.push(Rc::new(callback));
    }

    pub fn active_timer_count(&self) -> usize {
        self.timers.iter().filter(|t| t.source_id.is_some()).count()
    }

    pub fn on_show(&mut self) {
        for hook in &self.on_show_hooks {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe({
                let h = hook.clone();
                move || h()
            }));
        }

        for timer in &mut self.timers {
            if timer.source_id.is_none() {
                // Fire immediately once
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe({
                    let cb = timer.callback.clone();
                    move || cb()
                }));

                let cb = timer.callback.clone();
                let source = glib::timeout_add_local(timer.interval, move || {
                    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe({
                        let cb = cb.clone();
                        move || cb()
                    }));
                    if res.is_ok() {
                        glib::ControlFlow::Continue
                    } else {
                        glib::ControlFlow::Break
                    }
                });
                timer.source_id = Some(source);
            }
        }
    }

    pub fn on_hide(&mut self) {
        for hook in &self.on_hide_hooks {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe({
                let h = hook.clone();
                move || h()
            }));
        }

        for timer in &mut self.timers {
            if !timer.keep_on_hide {
                if let Some(source) = timer.source_id.take() {
                    safe_source_remove(source);
                }
            }
        }

        // Kill non-persistent child processes
        self.processes.retain(|proc| {
            if !proc.keep_on_hide {
                unsafe {
                    libc::kill(-proc.pid, libc::SIGKILL);
                }
                false
            } else {
                true
            }
        });
    }
}

impl Drop for PluginLifecycle {
    fn drop(&mut self) {
        for timer in &mut self.timers {
            if let Some(source) = timer.source_id.take() {
                safe_source_remove(source);
            }
        }
        for proc in &self.processes {
            unsafe {
                libc::kill(-proc.pid, libc::SIGKILL);
            }
        }
    }
}

pub struct SpecPluginAdapter {
    pub spec: PluginSpec,
    pub build_fn: fn(&PluginContext) -> gtk4::Widget,
    pub widget: Option<gtk4::Widget>,
    pub lifecycle: Rc<RefCell<PluginLifecycle>>,
    pub manifest: PluginManifest,
}

impl SpecPluginAdapter {
    pub fn new(
        spec: PluginSpec,
        build_fn: fn(&PluginContext) -> gtk4::Widget,
        ctx: &PluginContext,
    ) -> Self {
        let manifest = PluginManifest {
            id: spec.id.to_string(),
            title: spec.title.to_string(),
            icon: spec.icon.to_string(),
            order: spec.order,
            description: spec.description.to_string(),
        };

        Self {
            spec,
            build_fn,
            widget: None,
            lifecycle: ctx.lifecycle.clone(),
            manifest,
        }
    }
}

impl SidebarPlugin for SpecPluginAdapter {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn build_widget(&mut self, ctx: &PluginContext) -> gtk4::Widget {
        if let Some(ref w) = self.widget {
            return w.clone();
        }
        let w = (self.build_fn)(ctx);
        self.widget = Some(w.clone());
        w
    }

    fn on_show(&mut self) {
        self.lifecycle.borrow_mut().on_show();
    }

    fn on_hide(&mut self) {
        self.lifecycle.borrow_mut().on_hide();
    }
}
