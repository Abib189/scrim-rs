//! Screenshots without a window (`docs/design/panel-redesign.md`, B5): the app draws a frame headless,
//! egui tessellates it, and these triangles are filled in software as the GPU would (premultiplied
//! colour times the texture, blended over in gamma space). For when no window can draw, such as a locked
//! screen. Not a pixel-exact match for the GPU path (no MSAA, the texture filtered in gamma space): close enough to
//! review a layout, never used as a baseline.
//!
//! - `SCRIM_SHOTS_DIR=<dir> [SCRIM_SHOTS_SET=before|after] cargo test -p figcraft-ui-egui --lib
//!   offscreen -- --ignored` writes screenshots of `examples/ridgeline-app.figcraft` there. `before`
//!   (four selection states, light and dark) uses only commands older than the panel redesign, so
//!   the same file draws a tree from before it; `after` (the default) adds the redesign's states
//!   at three panel widths.
//! - `SCRIM_SHOTS_BEFORE=<dir>` with `SCRIM_SHOTS_DIR=<dir>` compares the shots both hold, outside
//!   the right panel, and prints how many pixels differ.
//!
//! Without the variables the ignored tests do nothing; the one test that always runs checks the
//! rasterizer itself.

use std::collections::HashMap;

use egui::epaint::{ClippedPrimitive, ImageDelta, Primitive, TextureId};
use egui::{Color32, Event, Pos2, RawInput, Rect, vec2};
use figcraft_engine::Session;
use serde_json::json;

use crate::{FigApp, NoServices};

/// A texture egui asked for, as RGBA premultiplied texels.
struct Texture {
    size: [usize; 2],
    pixels: Vec<Color32>,
}

/// An app drawn offscreen at a window size, with its textures kept between frames.
struct Offscreen {
    ctx: egui::Context,
    app: FigApp,
    size: egui::Vec2,
    time: f64,
    textures: HashMap<TextureId, Texture>,
    last: Vec<ClippedPrimitive>,
}

impl Offscreen {
    fn new(s: Session, size: egui::Vec2) -> Self {
        Self { ctx: egui::Context::default(), app: FigApp::new(s, Box::new(NoServices)), size, time: 0.0, textures: HashMap::new(), last: Vec::new() }
    }

    fn frame(&mut self, events: Vec<Event>) {
        self.time += 1.0 / 60.0;
        let input = RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.size)), time: Some(self.time), events, ..Default::default() };
        let app = &mut self.app;
        let mut out = self.ctx.run_ui(input, |ui| app.ui(ui));
        let (set, free) = (std::mem::take(&mut out.textures_delta.set), std::mem::take(&mut out.textures_delta.free));
        out.textures_delta.clear();
        for (id, deltas) in set {
            for delta in &deltas {
                self.apply(id, delta);
            }
        }
        self.last = self.ctx.tessellate(out.shapes, 1.0);
        for id in free {
            self.textures.remove(&id);
        }
    }

    fn apply(&mut self, id: TextureId, delta: &ImageDelta) {
        let egui::ImageData::Color(image) = &delta.image;
        match delta.pos {
            None => {
                self.textures.insert(id, Texture { size: image.size, pixels: image.pixels.clone() });
            }
            Some([x0, y0]) => {
                let Some(t) = self.textures.get_mut(&id) else { return };
                for y in 0..image.size[1] {
                    for x in 0..image.size[0] {
                        if let Some(p) = t.pixels.get_mut((y0 + y) * t.size[0] + x0 + x) {
                            *p = image.pixels[y * image.size[0] + x];
                        }
                    }
                }
            }
        }
    }

    /// Frames until the canvas render thread is done and the UI asks for nothing more.
    fn settle(&mut self) {
        for _ in 0..240 {
            self.frame(vec![]);
            let rendering = crate::idle_tests::wait_for_render(&self.app);
            if !rendering && !self.ctx.has_requested_repaint() {
                break;
            }
        }
        self.frame(vec![]);
    }

    /// The last frame, filled in software.
    fn image(&self) -> image::RgbaImage {
        let (w, h) = (self.size.x as usize, self.size.y as usize);
        let mut px = vec![[0.0_f32; 4]; w * h];
        for cp in &self.last {
            let Primitive::Mesh(mesh) = &cp.primitive else { continue };
            let tex = self.textures.get(&mesh.texture_id);
            let clip = cp.clip_rect.intersect(Rect::from_min_size(Pos2::ZERO, self.size));
            for tri in mesh.indices.chunks_exact(3) {
                let v = [tri[0], tri[1], tri[2]].map(|i| mesh.vertices[i as usize]);
                fill_triangle(&mut px, w, clip, &v, tex);
            }
        }
        let mut img = image::RgbaImage::new(w as u32, h as u32);
        for (i, p) in px.iter().enumerate() {
            let c = [p[0], p[1], p[2], p[3]].map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8);
            img.put_pixel((i % w) as u32, (i / w) as u32, image::Rgba([c[0], c[1], c[2], 255]));
        }
        img
    }
}

/// Fill one triangle: colours and texture coordinates interpolated at each pixel centre, the
/// texel (bilinear) times the vertex colour, blended over (premultiplied alpha).
fn fill_triangle(px: &mut [[f32; 4]], w: usize, clip: Rect, v: &[egui::epaint::Vertex; 3], tex: Option<&Texture>) {
    let (a, b, c) = (v[0].pos, v[1].pos, v[2].pos);
    let area = (b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y);
    if area.abs() < 1e-6 {
        return;
    }
    let bbox = Rect::from_min_max(a.min(b).min(c), a.max(b).max(c)).intersect(clip);
    if !bbox.is_positive() {
        return;
    }
    let colors = v.map(|v| gamma(v.color));
    for y in (bbox.min.y.floor() as usize)..(bbox.max.y.ceil() as usize) {
        for x in (bbox.min.x.floor() as usize)..(bbox.max.x.ceil() as usize) {
            let p = Pos2::new(x as f32 + 0.5, y as f32 + 0.5);
            if !clip.contains(p) {
                continue;
            }
            let w0 = ((b.x - p.x) * (c.y - p.y) - (c.x - p.x) * (b.y - p.y)) / area;
            let w1 = ((c.x - p.x) * (a.y - p.y) - (a.x - p.x) * (c.y - p.y)) / area;
            let w2 = 1.0 - w0 - w1;
            if w0 < -1e-4 || w1 < -1e-4 || w2 < -1e-4 {
                continue;
            }
            let mut col = [0.0_f32; 4];
            for (k, slot) in col.iter_mut().enumerate() {
                *slot = colors[0][k] * w0 + colors[1][k] * w1 + colors[2][k] * w2;
            }
            if let Some(t) = tex {
                let uv = v[0].uv.to_vec2() * w0 + v[1].uv.to_vec2() * w1 + v[2].uv.to_vec2() * w2;
                let s = sample(t, uv);
                for k in 0..4 {
                    col[k] *= s[k];
                }
            }
            let Some(dst) = px.get_mut(y * w + x) else { continue };
            for k in 0..4 {
                dst[k] = col[k] + dst[k] * (1.0 - col[3]);
            }
        }
    }
}

/// A premultiplied colour's components, 0 to 1, in gamma space (where egui's GPU path blends).
fn gamma(c: Color32) -> [f32; 4] {
    c.to_array().map(|v| f32::from(v) / 255.0)
}

/// The texel at `uv`, bilinear, premultiplied RGBA in gamma space.
fn sample(t: &Texture, uv: egui::Vec2) -> [f32; 4] {
    let (tw, th) = (t.size[0] as f32, t.size[1] as f32);
    let (fx, fy) = (uv.x * tw - 0.5, uv.y * th - 0.5);
    let (x0, y0) = (fx.floor(), fy.floor());
    let (dx, dy) = (fx - x0, fy - y0);
    let at = |x: f32, y: f32| {
        let (x, y) = ((x.max(0.0) as usize).min(t.size[0] - 1), (y.max(0.0) as usize).min(t.size[1] - 1));
        gamma(t.pixels[y * t.size[0] + x])
    };
    let (p00, p10, p01, p11) = (at(x0, y0), at(x0 + 1.0, y0), at(x0, y0 + 1.0), at(x0 + 1.0, y0 + 1.0));
    let mut out = [0.0; 4];
    for k in 0..4 {
        out[k] = (p00[k] * (1.0 - dx) + p10[k] * dx) * (1.0 - dy) + (p01[k] * (1.0 - dx) + p11[k] * dx) * dy;
    }
    out
}

/// Ridgeline at `size`, its right panel `panel` wide, in the light or dark theme.
fn ridgeline(dark: bool, panel: f32) -> Offscreen {
    let mut s = Session::new();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/ridgeline-app.figcraft");
    s.execute("file.open", &json!({"path": path})).unwrap_or_default();
    s.prefs.theme = if dark { "dark" } else { "light" }.into();
    s.prefs.panel_widths = (240.0, panel);
    let mut o = Offscreen::new(s, vec2(1440.0, 900.0));
    o.settle();
    o.app.session.execute("view.zoomFit", &json!({})).unwrap_or_default();
    o.settle();
    o
}

fn select(o: &mut Offscreen, ids: &[u64]) {
    o.app.session.execute("select.set", &json!({"ids": ids})).unwrap_or_default();
    o.settle();
}

fn run(o: &mut Offscreen, cmd: &str, p: serde_json::Value) {
    o.app.session.execute(cmd, &p).unwrap_or_default();
    o.app.handle_ui_requests();
    o.settle();
}

fn save(o: &Offscreen, dir: &std::path::Path, name: &str) {
    let path = dir.join(format!("{name}.png"));
    o.image().save(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
}

/// The frame "Home — Light", the text "9:41" and three frames of Ridgeline.
const FRAME: u64 = 68;
const TEXT: u64 = 70;
const FRAMES: [u64; 3] = [68, 239, 178];

#[test]
#[ignore = "writes screenshots when SCRIM_SHOTS_DIR is set"]
fn screenshots() {
    let Some(dir) = std::env::var_os("SCRIM_SHOTS_DIR").map(std::path::PathBuf::from) else { return };
    std::fs::create_dir_all(&dir).unwrap_or_default();
    let after = std::env::var("SCRIM_SHOTS_SET").map_or(true, |s| s != "before");
    for (dark, theme) in [(false, "light"), (true, "dark")] {
        let mut o = ridgeline(dark, 240.0);
        select(&mut o, &[]);
        save(&o, &dir, &format!("none-{theme}"));
        select(&mut o, &[FRAME]);
        save(&o, &dir, &format!("frame-{theme}"));
        select(&mut o, &[TEXT]);
        save(&o, &dir, &format!("text-{theme}"));
        select(&mut o, &FRAMES);
        save(&o, &dir, &format!("multiple-{theme}"));
        // The home screen (Stage C pins Settings at the bottom of its sidebar).
        o.app.ui.home = true;
        o.settle();
        save(&o, &dir, &format!("home-{theme}"));
        o.app.ui.home = false;
        o.settle();
        if !after {
            continue;
        }
        select(&mut o, &[FRAME]);
        for (tab, name) in [("prototype", "prototype-frame"), ("code", "code-frame"), ("ai", "ai")] {
            run(&mut o, &format!("panel.tab.{tab}"), json!({}));
            save(&o, &dir, &format!("{name}-{theme}"));
        }
        run(&mut o, "panel.mode.animate", json!({}));
        save(&o, &dir, &format!("animate-{theme}"));
        run(&mut o, "panel.tab.design", json!({}));
        run(&mut o, "pref.accent", json!({"preset": "teal"}));
        o.app.ui.preferences = true;
        o.settle();
        save(&o, &dir, &format!("preferences-teal-{theme}"));
        o.app.ui.preferences = false;
        run(&mut o, "panel.tab.code", json!({}));
        save(&o, &dir, &format!("code-frame-teal-{theme}"));
        // Three panel widths: the default (also the narrowest), and two wider.
        for panel in [240.0, 360.0, 480.0] {
            let mut o = ridgeline(dark, panel);
            select(&mut o, &[FRAME]);
            save(&o, &dir, &format!("width-{}-frame-{theme}", panel as u32));
            run(&mut o, "panel.tab.ai", json!({}));
            save(&o, &dir, &format!("width-{}-ai-{theme}", panel as u32));
        }
    }
}

/// R5e: "Describe a change" in the AI tab, each state light and dark, on Ridgeline with a scripted
/// agent run (the screen being locked, no window could be captured).
#[test]
#[ignore = "writes screenshots when SCRIM_CHAT_SHOTS_DIR is set"]
fn chat_screenshots() {
    use crate::follow_agent::chat_ui::tests::{CLAUDE, Platform, Script};
    use crate::services::{AgentEnd, AgentRunEvent};
    let Some(dir) = std::env::var_os("SCRIM_CHAT_SHOTS_DIR").map(std::path::PathBuf::from) else { return };
    let _ = std::fs::create_dir_all(&dir);
    for dark in [false, true] {
        let theme = if dark { "dark" } else { "light" };
        let script = std::sync::Arc::new(std::sync::Mutex::new(Script { claude: Some(CLAUDE.into()), ..Default::default() }));
        let mut o = ridgeline(dark, 300.0);
        o.app.services = Box::new(Platform(script.clone()));
        o.app.session.agents.mcp.port = Some(7690);
        run(&mut o, "panel.tab.ai", json!({}));
        // The first use asks.
        o.app.ui.agent_view.chat.draft = "Make the trail cards' titles larger".into();
        run(&mut o, "chat.send", json!({"prompt": "Make the trail cards' titles larger", "agent": "claude-code"}));
        save(&o, &dir, &format!("chat-{theme}-1-first-use-asks"));
        run(&mut o, "chat.allow-agent", json!({"agent": "claude-code", "path": CLAUDE}));
        // At work: two batches land through the app's MCP route.
        let ctx = o.ctx.clone();
        for k in 0..2 {
            let cmds: Vec<serde_json::Value> = (0..3)
                .map(|j| json!({"command": "shape.rectangle", "params": {"x": 2000 + 60 * j, "y": 80 * k, "width": 40, "height": 40}}))
                .collect();
            let _ = crate::control::handle_route(
                &mut o.app,
                &ctx,
                crate::control::ROUTE_MCP_HTTP,
                "engine.execute",
                &json!({"command": "command.batch", "params": {"commands": cmds}}),
            );
            o.settle();
        }
        save(&o, &dir, &format!("chat-{theme}-2-running"));
        script.lock().unwrap().events.extend([
            AgentRunEvent::Line(r#"{"type":"result","subtype":"success","result":"I set the three trail cards' titles to 18 px semibold and left the rest as it was."}"#.into()),
            AgentRunEvent::Ended { end: AgentEnd::Exited(Some(0)), stderr: vec![], secs: 38.0 },
        ]);
        o.settle();
        save(&o, &dir, &format!("chat-{theme}-3-finished"));
        run(&mut o, "settings.open", json!({"section": "ai"}));
        save(&o, &dir, &format!("chat-{theme}-4-settings"));
        o.app.ui.preferences = false;
        script.lock().unwrap().claude = None;
        o.app.ui.agent_view.chat = Default::default();
        o.settle();
        save(&o, &dir, &format!("chat-{theme}-5-no-agent-installed"));
    }
}

/// Fix T4's captures of the AI panel at its default (and minimum) width, 240 px, light and dark:
/// empty, the conversations' popover, a run under way, the grace hint's line, and Settings ▸ Agents.
/// Offscreen: the screen was locked, so no window could be captured.
#[test]
#[ignore = "writes screenshots when SCRIM_AI_SHOTS_DIR is set"]
fn ai_panel_screenshots() {
    use crate::follow_agent::chat_ui::Line;
    use crate::follow_agent::chat_ui::store::Conversation;
    use crate::follow_agent::chat_ui::tests::{CLAUDE, Platform, Script};
    let Some(dir) = std::env::var_os("SCRIM_AI_SHOTS_DIR").map(std::path::PathBuf::from) else { return };
    let _ = std::fs::create_dir_all(&dir);
    for dark in [false, true] {
        let theme = if dark { "dark" } else { "light" };
        let script = std::sync::Arc::new(std::sync::Mutex::new(Script { claude: Some(CLAUDE.into()), ..Default::default() }));
        let mut o = ridgeline(dark, crate::PANEL_DEFAULT);
        o.app.services = Box::new(Platform(script.clone()));
        o.app.session.agents.mcp.port = Some(7690);
        run(&mut o, "panel.tab.ai", json!({}));
        save(&o, &dir, &format!("ai-{theme}-1-empty"));
        // Two conversations of this file and one of another; the popover open.
        let here = crate::follow_agent::chat_ui::store::doc_key(&o.app).map(|k| k.0).unwrap_or_default();
        let conv = |id: &str, title: &str, doc: &str, name: &str, updated: u64| Conversation {
            id: id.into(),
            title: title.into(),
            doc: doc.into(),
            doc_name: name.into(),
            updated,
            lines: vec![Line::You(title.into())],
        };
        o.app.ui.agent_view.chat.store.list = vec![
            conv("c1", "Make the trail cards' titles larger", &here, "Ridgeline", 20),
            conv("c2", "A darker hero image", &here, "Ridgeline", 10),
            conv("c3", "Landing page sections", "other.figcraft", "Site", 30),
        ];
        run(&mut o, "chat.open", json!({"id": "c1"}));
        o.app.ui.agent_view.chat.store.open = true;
        o.settle();
        save(&o, &dir, &format!("ai-{theme}-2-popover"));
        o.app.ui.agent_view.chat.store.open = false;
        // A run under way.
        run(&mut o, "chat.new", json!({}));
        run(&mut o, "chat.allow-agent", json!({"agent": "claude-code", "path": CLAUDE}));
        o.app.ui.agent_view.chat.confirm = None;
        run(&mut o, "chat.send", json!({"prompt": "Make the trail cards' titles larger", "agent": "claude-code"}));
        o.settle();
        save(&o, &dir, &format!("ai-{theme}-3-running"));
        run(&mut o, "chat.cancel", json!({}));
        o.app.ui.agent_view.chat.running = None;
        // The grace hint's line, while it is pending.
        let h = &mut o.app.ui.agent_view.grace_hint;
        (h.shown, h.showing) = (true, true);
        o.settle();
        save(&o, &dir, &format!("ai-{theme}-4-hint"));
        run(&mut o, "settings.open", json!({"section": "agents"}));
        save(&o, &dir, &format!("ai-{theme}-5-settings-agents"));
        o.app.ui.preferences = false;
    }
}

#[test]
#[ignore = "compares screenshots when SCRIM_SHOTS_BEFORE and SCRIM_SHOTS_DIR are set"]
fn compare_outside_the_right_panel() {
    let (Some(before), Some(after)) = (std::env::var_os("SCRIM_SHOTS_BEFORE"), std::env::var_os("SCRIM_SHOTS_DIR")) else { return };
    let (before, after) = (std::path::PathBuf::from(before), std::path::PathBuf::from(after));
    let mut names: Vec<_> = std::fs::read_dir(&before).into_iter().flatten().flatten().map(|e| e.file_name()).collect();
    names.sort();
    for name in names {
        let (Ok(a), Ok(b)) = (image::open(before.join(&name)), image::open(after.join(&name))) else { continue };
        let (a, b) = (a.to_rgba8(), b.to_rgba8());
        // Left of the 240 px right panel and its border.
        let edge = a.width().saturating_sub(241);
        let differ = (0..a.height())
            .flat_map(|y| (0..edge).map(move |x| (x, y)))
            .filter(|&(x, y)| a.get_pixel(x, y).0.iter().zip(b.get_pixel(x, y).0.iter()).any(|(p, q)| p.abs_diff(*q) > 2))
            .count();
        println!("{}: {differ} pixels differ outside the right panel", name.to_string_lossy());
    }
}

/// The rasterizer draws what egui asks: both side panels in their theme's colour.
#[test]
fn the_rasterizer_draws_the_panels_in_their_colours() {
    for (theme, t) in [("light", crate::theme::Tokens::light()), ("dark", crate::theme::Tokens::dark())] {
        let mut s = Session::new();
        s.prefs.theme = theme.into();
        let mut o = Offscreen::new(s, vec2(900.0, 560.0));
        o.settle();
        let img = o.image();
        let at = |x: u32, y: u32| {
            let p = img.get_pixel(x, y);
            Color32::from_rgb(p[0], p[1], p[2])
        };
        assert_eq!(at(150, 545), t.panel, "{theme}: the layers panel");
        assert_eq!(at(899 - 120, 545), t.panel, "{theme}: the right panel");
    }
}

/// Fix X6's captures of the rebuilt AI panel: light and dark; the default orange, a very pale
/// (#FFE066) and a very dark (#1F3BFF) accent; at the design's 336 px and at 240 px; idle, and
/// mid-run (the banner, your message, and the card with three done rows and the active one).
/// Offscreen (`docs/scrim-ports/ai-panel/x/`).
#[test]
#[ignore = "writes screenshots when SCRIM_AI_X_DIR is set"]
fn ai_panel_x_captures() {
    use crate::follow_agent::chat_ui::tests::{CLAUDE, Platform, Script};
    let Some(dir) = std::env::var_os("SCRIM_AI_X_DIR").map(std::path::PathBuf::from) else { return };
    let _ = std::fs::create_dir_all(&dir);
    for dark in [false, true] {
        let orange = figcraft_engine::cmd::appearance::hex(figcraft_engine::cmd::appearance::DEFAULT_ACCENT);
        for (accent, hex) in [("orange", orange.as_str()), ("pale", "#ffe066"), ("blue", "#1f3bff")] {
            for width in [336.0, 240.0] {
                let theme = if dark { "dark" } else { "light" };
                let name = |what: &str| format!("x-{theme}-{accent}-{}-{what}", width as u32);
                let script = std::sync::Arc::new(std::sync::Mutex::new(Script { claude: Some(CLAUDE.into()), ..Default::default() }));
                let mut o = ridgeline(dark, width);
                o.app.session.prefs.accent = hex.into();
                o.app.services = Box::new(Platform(script.clone()));
                o.app.session.agents.mcp.port = Some(7690);
                run(&mut o, "panel.tab.ai", json!({}));
                save(&o, &dir, &name("1-idle"));
                run(&mut o, "chat.allow-agent", json!({"agent": "claude-code", "path": CLAUDE}));
                o.app.ui.agent_view.chat.confirm = None;
                run(
                    &mut o,
                    "chat.send",
                    json!({"prompt": "Build a poster called Welle: twenty bars forming a sine wave, five peaks in orange.", "agent": "claude-code"}),
                );
                // The agent's edits, over the app's MCP route: a frame, twenty bars, a title and a
                // footer, then a property change (the active row).
                let ctx = o.ctx.clone();
                let mut agent = |cmd: &str, p: serde_json::Value| {
                    crate::control::handle_route(&mut o.app, &ctx, "mcp (http)", "engine.execute", &json!({"command": cmd, "params": p}))
                        .unwrap_or_default()
                };
                let f = agent("frame.create", json!({"x": 3000, "y": 0, "width": 640, "height": 480, "name": "Poster — Welle"}));
                let bars: Vec<serde_json::Value> = (0..20)
                    .map(|k| json!({"command": "shape.rectangle", "params": {"x": 20 + k * 30, "y": 200, "width": 20, "height": 60 + (k % 5) * 30, "parent": f["id"]}}))
                    .collect();
                agent("command.batch", json!({"commands": bars}));
                agent("text.create", json!({"text": "Welle", "x": 20, "y": 20, "parent": f["id"]}));
                agent("text.create", json!({"text": "Five peaks", "x": 20, "y": 440, "parent": f["id"]}));
                agent("node.set", json!({"id": f["id"], "props": {"name": "Poster — Welle"}}));
                o.settle();
                save(&o, &dir, &name("2-mid-run"));
                run(&mut o, "chat.cancel", json!({}));
                o.app.ui.agent_view.chat.running = None;
            }
        }
    }
}

/// Fix Z: the AI panel after a run, light and dark, at 336 and 240 px: a 48-character title cut
/// with "…", the card's rows by name, the agent's Markdown reply, and the round Selection chip
/// (`1-done`); the card after Undo run (`2-undone`); the composer with Codex chosen, so the image
/// chip shows too (`3-composer`). Offscreen (`docs/scrim-ports/ai-panel/z/`).
#[test]
#[ignore = "writes screenshots when SCRIM_AI_Z_DIR is set"]
fn ai_panel_z_captures() {
    if let Some(dir) = std::env::var_os("SCRIM_AI_Z_DIR").map(std::path::PathBuf::from) {
        run_captures(&dir, "z");
    }
}

/// Fix AA1: the same captures with the app's real fonts (Inter Regular, SemiBold and Italic), the
/// harness installing them as the app does (`docs/scrim-ports/ai-panel/aa/`).
#[test]
#[ignore = "writes screenshots when SCRIM_AI_AA_DIR is set"]
fn ai_panel_aa_captures() {
    if let Some(dir) = std::env::var_os("SCRIM_AI_AA_DIR").map(std::path::PathBuf::from) {
        run_captures(&dir, "aa");
    }
}

/// The AI panel after a run, its card undone, and the composer with Codex chosen, light and dark,
/// at 336 and 240 px, named `{prefix}-{theme}-{width}-{state}.png`.
fn run_captures(dir: &std::path::Path, prefix: &str) {
    use crate::follow_agent::chat_ui::tests::{CLAUDE, Platform, Script, feed, line_ev};
    use crate::services::{AgentEnd, AgentRunEvent};
    let _ = std::fs::create_dir_all(dir);
    let reply = "Done. **Poster — Welle** is on the canvas:\n\n- 20 bars in a sine wave, *five* peaks in `#FF6A00`\n- the title in Inter Bold, 96 pt\n\n1. Check the footer's spacing\n2. Try a darker ground\n\n```\nbars: 20, peaks: 5\n```";
    for dark in [false, true] {
        for width in [336.0, 240.0] {
            let theme = if dark { "dark" } else { "light" };
            let name = |what: &str| format!("{prefix}-{theme}-{}-{what}", width as u32);
            let script = std::sync::Arc::new(std::sync::Mutex::new(Script {
                claude: Some(CLAUDE.into()),
                codex: Some("/opt/agents/bin/codex".into()),
                ..Default::default()
            }));
            let mut o = ridgeline(dark, width);
            o.app.services = Box::new(Platform(script.clone()));
            o.app.session.agents.mcp.port = Some(7690);
            run(&mut o, "panel.tab.ai", json!({}));
            run(&mut o, "chat.allow-agent", json!({"agent": "claude-code", "path": CLAUDE}));
            o.app.ui.agent_view.chat.confirm = None;
            let prompt = "Build a Swiss poster series for the Welle music festival in Hamburg, twenty bars in a sine wave";
            run(&mut o, "chat.send", json!({"prompt": prompt, "agent": "claude-code"}));
            let ctx = o.ctx.clone();
            let mut agent = |cmd: &str, p: serde_json::Value| {
                crate::control::handle_route(&mut o.app, &ctx, "mcp (http)", "engine.execute", &json!({"command": cmd, "params": p}))
                    .unwrap_or_default()
            };
            let f = agent("frame.create", json!({"x": 3000, "y": 0, "width": 640, "height": 480, "name": "Poster — Welle"}));
            let bars: Vec<serde_json::Value> = (0..20)
                .map(|k| json!({"command": "shape.rectangle", "params": {"x": 20 + k * 30, "y": 200, "width": 20, "height": 60 + (k % 5) * 30, "parent": f["id"]}}))
                .collect();
            agent("command.batch", json!({"label": "Lay out twenty bars in a sine wave", "commands": bars}));
            let t = agent("text.create", json!({"text": "Welle", "x": 20, "y": 20, "parent": f["id"]}));
            agent("text.create", json!({"text": "Five peaks", "x": 20, "y": 440, "parent": f["id"]}));
            agent("node.set", json!({"id": t["id"], "props": {"name": "Title"}}));
            let result = json!({"type": "result", "subtype": "success", "is_error": false, "result": reply}).to_string();
            feed(&script, vec![line_ev(&result), AgentRunEvent::Ended { end: AgentEnd::Exited(Some(0)), stderr: vec![], secs: 42.0 }]);
            o.settle();
            o.settle();
            save(&o, dir, &name("1-done"));
            run(&mut o, "chat.undo-run", json!({}));
            o.settle();
            save(&o, dir, &name("2-undone"));
            run(&mut o, "chat.agent", json!({"agent": "codex"}));
            o.settle();
            save(&o, dir, &name("3-composer"));
        }
    }
}
