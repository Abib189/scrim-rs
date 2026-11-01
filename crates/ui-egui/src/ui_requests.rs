//! Acting on the UI requests commands queue: dialogs, panels, windows (moved out of `lib.rs`).

use super::*;

impl FigApp {
    /// Act on UI requests queued by commands (dialogs, panels).
    pub fn handle_ui_requests(&mut self) {
        layers_restore::check(self);
        let reqs = std::mem::take(&mut self.session.ui_requests);
        for r in reqs {
            match r.kind.as_str() {
                "dialog.about" => {
                    self.ui.about = true;
                    self.ui.about_tab = dialogs::AboutTab::About;
                }
                "dialog.preferences" => {
                    self.ui.preferences = true;
                    self.ui.settings_section = "general";
                }
                "dialog.missingFonts" => missing_fonts::request(self),
                "panel.shortcuts" => self.ui.shortcuts_panel = !self.ui.shortcuts_panel,
                "palette.actions" => menus::toggle_palette(&mut self.ui),
                kind if panel_view::request(self, kind, &r.data) => {}
                "panel.leftSidebar" => self.ui.left_collapsed = !self.ui.left_collapsed,
                "panel.widths" => {
                    let width = |k: &str| r.data.get(k).and_then(Value::as_f64).map(|w| w as f32);
                    let (left, right) = (width("left"), width("right"));
                    let reset = left.is_none() && right.is_none();
                    self.set_panel_widths(
                        left.unwrap_or(if reset { PANEL_DEFAULT } else { self.ui.left_width }),
                        right.unwrap_or(if reset { PANEL_DEFAULT } else { self.ui.right_width }),
                    );
                    self.services.save_prefs(&self.session.prefs);
                }
                "focus.region" => self.ui.nav.step(r.data.get("step").and_then(Value::as_i64).unwrap_or(1).clamp(-4, 4) as i32),
                "panel.layers" => self.ui.left_tab = LeftTab::File,
                "panel.assets" => {
                    self.ui.left_tab = LeftTab::Assets;
                    self.ui.left_collapsed = false;
                    self.ui.versions = Default::default();
                }
                "panel.versionHistory" => versions::open(self),
                "view.devCompare" => {
                    self.ui.dev.compare = r.data.get("id").and_then(figcraft_engine::params::id_of).map(dev_compare::CompareView::new);
                }
                "panel.libraries" => {
                    self.ui.left_tab = LeftTab::Assets;
                    self.ui.left_collapsed = false;
                    self.ui.libraries = true;
                }
                "home" => self.ui.home = true,
                "home.templates" => {
                    self.ui.home = true;
                    self.ui.home_view.section = home::Section::Templates;
                }
                "home.gettingStarted" => {
                    self.ui.home = true;
                    self.ui.home_view.section = home::Section::Recents;
                    self.services.save_prefs(&self.session.prefs);
                }
                "dialog.saveAs" => self.save_as_dialog(),
                "dialog.colorProfile" => self.ui.color_profile = self.session.doc().ok().map(|d| d.color_profile),
                "present" => {
                    if let Some(start) = r.data.get("start").and_then(figcraft_engine::params::id_of) {
                        let mut p = present::Presenter::new(start);
                        p.view = self.ui.present_view;
                        self.ui.present = Some(p);
                        self.ui.paused_present = None;
                        self.ui.present_uid = self.session.docs.get(self.session.active).map(|d| d.uid);
                        self.ui.home = false;
                    }
                }
                "present.options" => {
                    let v = &mut self.ui.present_view;
                    if let Some(s) = r.data.get("scale").and_then(|s| s.as_str()).and_then(present::PresentScale::parse) {
                        v.scale = s;
                    }
                    if let Some(f) = r.data.get("deviceFrame").and_then(|f| f.as_bool()) {
                        v.device_frame = f;
                    }
                    if let Some(h) = r.data.get("hotspotHints").and_then(|h| h.as_bool()) {
                        v.hotspot_hints = h;
                    }
                    if let Some(i) = r.data.get("inspect").and_then(|i| i.as_bool()) {
                        v.inspect = i;
                    }
                    if let Some(p) = self.ui.present.as_mut() {
                        p.view.device_frame = v.device_frame;
                        p.view.hotspot_hints = v.hotspot_hints;
                        p.view.inspect = v.inspect;
                        p.set_scale(v.scale);
                    }
                }
                "preview" => {
                    if let Some(start) = r.data.get("start").and_then(figcraft_engine::params::id_of) {
                        self.ui.preview = Some(present::Pane::new(start, &self.session));
                    }
                }
                "preview.close" => self.ui.preview = None,
                "open.help" => self.services.open_url("https://github.com/Abib189/scrim-rs#readme"),
                "layers.collapse-all" => layers_restore::collapse(self),
                "layers.restore-expansion" => layers_restore::restore(self),
                figcraft_engine::cmd::camera::FLY_REQUEST => camera_flight::request(self, &r.data),
                "contextMenu" => self.ui.key_menu = true,
                "selectArea" => self.start_key_box(),
                "canvas.renameLabel" => {
                    self.ui.rename_label = r.data.get("id").and_then(figcraft_engine::params::id_of).map(|id| layers::Rename { id, start: true });
                }
                "layers.rename" => {
                    let ids: Vec<NodeId> =
                        r.data.get("ids").and_then(Value::as_array).into_iter().flatten().filter_map(figcraft_engine::params::id_of).collect();
                    if ids.len() > 1 {
                        dialogs::open_batch_rename(self, &ids);
                    } else {
                        layers::start_rename(self, &ids);
                    }
                }
                "comment.open" => {
                    if let Some(id) = r.data.get("id").and_then(Value::as_u64) {
                        self.ui.comments.show(id);
                    }
                }
                "comment.new" => {
                    let x = r.data.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let y = r.data.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    comments::start_draft(self, x, y);
                }
                "dialog.unsaved" => self.toast(i18n::tr("This file has unsaved changes")),
                "dialog.plugins" => self.ui.plugins_window = true,
                "dialog.cursorChat" => live::open_chat(self),
                "window.new" | "window.focus" | "window.fullscreen" => self.window_requests.push(r),
                other => windows::request(self, other),
            }
        }
        if let Some(m) = self.session.message.take() {
            self.toast(m);
        }
    }
}
