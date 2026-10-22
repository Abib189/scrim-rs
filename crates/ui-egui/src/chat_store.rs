//! The AI panel's conversations (Fix T4, `docs/design/ai-panel.md`): kept on this computer, in the
//! app's settings folder (`conversations.json`, through `Services`), keyed by document, never in the
//! document file. No keys, and nothing of them reaches Settings ▸ Diagnostics.
//! - **A new chat is made lazily:** on its first send, titled from that prompt.
//! - **Keyed by document:** the file's path, or `unsaved:{uid}` for a document never saved (it lasts
//!   as long as that document is open).
//! - **Another document's conversation** opens read-only: a run needs its own document.
//! - **Search** matches the title and every line's text, ignoring case, here only.
//! - The newest [`KEEP`] conversations are kept, each with its last [`LINES`] lines.

use serde::{Deserialize, Serialize};

use super::Line;
use crate::FigApp;

/// How many conversations are kept.
pub const KEEP: usize = 200;
/// How many lines a conversation keeps.
pub const LINES: usize = 500;
/// A title's length at most, in characters.
pub const TITLE: usize = 48;

/// One conversation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    /// The document's key: its file's path, or `unsaved:{uid}`.
    pub doc: String,
    /// The document's name when the conversation began.
    pub doc_name: String,
    /// When it last changed (Unix seconds).
    pub updated: u64,
    pub lines: Vec<Line>,
}

/// The conversations and which is shown.
#[derive(Default)]
pub struct Store {
    pub list: Vec<Conversation>,
    /// The conversation shown, once it has been saved (`None`: a new chat, not made yet).
    pub current: Option<String>,
    /// The list shows every document's conversations, not only this one's.
    pub all: bool,
    /// The list's search.
    pub query: String,
    /// The popover is open.
    pub open: bool,
    loaded: bool,
}

fn unix_now() -> u64 {
    web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or_default()
}

/// The document shown: its key and name.
pub fn doc_key(app: &FigApp) -> Option<(String, String)> {
    let ds = app.session.ds().ok()?;
    let name = ds.doc.name.clone();
    Some(match ds.path.as_ref() {
        Some(p) => (p.clone(), name),
        None => (format!("unsaved:{}", ds.uid), name),
    })
}

/// The conversations as last kept, read once.
pub fn load(app: &mut FigApp) {
    if app.ui.agent_view.chat.store.loaded {
        return;
    }
    app.ui.agent_view.chat.store.loaded = true;
    if let Some(json) = app.services.load_conversations() {
        // A file this version can't read is set aside, not lost: it is only replaced on the next save.
        app.ui.agent_view.chat.store.list = serde_json::from_str(&json).unwrap_or_default();
    }
}

fn save(app: &mut FigApp) {
    let list = &app.ui.agent_view.chat.store.list;
    if let Ok(json) = serde_json::to_string(list) {
        app.services.save_conversations(&json);
    }
}

/// `line` joins the conversation shown, which is made now if it is a new chat's first send.
pub fn append(app: &mut FigApp, line: &Line) {
    load(app);
    let key = doc_key(app);
    let st = &mut app.ui.agent_view.chat.store;
    let at = st.current.as_ref().and_then(|c| st.list.iter().position(|x| &x.id == c));
    let i = match (at, line, key) {
        (Some(i), _, _) => i,
        (None, Line::You(prompt), Some((doc, doc_name))) => {
            let n = st.list.len();
            let id = format!("c{}-{n}", unix_now());
            let title: String = prompt.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(TITLE).collect();
            st.list.push(Conversation { id: id.clone(), title, doc, doc_name, updated: unix_now(), lines: Vec::new() });
            st.current = Some(id);
            st.list.len() - 1
        }
        // Lines before any send (a note) belong to no conversation.
        _ => return,
    };
    let c = &mut st.list[i];
    c.lines.push(line.clone());
    if c.lines.len() > LINES {
        let cut = c.lines.len() - LINES;
        c.lines.drain(..cut);
    }
    c.updated = unix_now();
    // The newest KEEP.
    if st.list.len() > KEEP {
        st.list.sort_by_key(|c| std::cmp::Reverse(c.updated));
        st.list.truncate(KEEP);
    }
    save(app);
}

/// The conversation shown, if it is saved.
pub fn current(app: &FigApp) -> Option<&Conversation> {
    let st = &app.ui.agent_view.chat.store;
    st.current.as_ref().and_then(|c| st.list.iter().find(|x| &x.id == c))
}

/// Whether the conversation shown belongs to another document (it is read-only here).
pub fn read_only(app: &FigApp) -> bool {
    let key = doc_key(app).map(|k| k.0);
    current(app).is_some_and(|c| Some(&c.doc) != key.as_ref())
}

/// The title shown in the panel's header.
pub fn title(app: &FigApp) -> String {
    current(app).map_or_else(|| crate::i18n::tr("New chat").to_string(), |c| c.title.clone())
}

/// A line's text, for search.
fn text(l: &Line) -> &str {
    match l {
        Line::You(s) | Line::Work(s) | Line::Note(s) | Line::Failure(s) | Line::Cost(s) => s,
        Line::Reply { text, .. } => text,
    }
}

/// The conversations the popover lists, newest first: this document's (or every one's with `all`)
/// matching `query` in their title or any line, ignoring case.
pub fn listed(app: &FigApp) -> Vec<&Conversation> {
    let st = &app.ui.agent_view.chat.store;
    let key = doc_key(app).map(|k| k.0);
    let q = st.query.trim().to_lowercase();
    let mut out: Vec<&Conversation> = st
        .list
        .iter()
        .filter(|c| st.all || Some(&c.doc) == key.as_ref())
        .filter(|c| q.is_empty() || c.title.to_lowercase().contains(&q) || c.lines.iter().any(|l| text(l).to_lowercase().contains(&q)))
        .collect();
    out.sort_by_key(|c| std::cmp::Reverse(c.updated));
    out
}

/// A new chat: nothing shown, made on its first send.
pub fn new_chat(app: &mut FigApp) {
    let chat = &mut app.ui.agent_view.chat;
    chat.store.current = None;
    chat.store.open = false;
    chat.lines.clear();
}

/// Show conversation `id`.
pub fn open(app: &mut FigApp, id: &str) -> bool {
    load(app);
    let chat = &mut app.ui.agent_view.chat;
    let Some(c) = chat.store.list.iter().find(|c| c.id == id) else { return false };
    chat.lines = c.lines.clone();
    chat.store.current = Some(c.id.clone());
    chat.store.open = false;
    true
}
