//! Files upstream FigCraft saved as boards (Fix AI C-1). Upstream 8167f13a made a board an ordinary
//! document with `kind: "board"`, whose objects are layers carrying a `board` object (a sticky's
//! author, what a connector joins). Scrim has no boards: it reads the file as a design and a save
//! leaves both marks out, so the reader reports them and the window can say so before that happens.

use serde::de::{Deserializer, IgnoredAny, MapAccess, Visitor};
use serde::{Deserialize, Serialize};

/// The board marks a file carries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct BoardMarks {
    /// The document says `kind: "board"`.
    pub kind: bool,
    /// Layers carrying a `board` object.
    pub items: usize,
}

/// The marks in `document.json` text `text`, whose `kind` the header read (`kind_board`). Layers
/// are looked at only when the text holds a `"board"` key at all: a design without one is read
/// once, as before.
pub(crate) fn find(text: &str, kind_board: bool) -> Option<BoardMarks> {
    let items = if text.contains("\"board\"") { serde_json::from_str::<Probe>(text).map_or(0, |p| p.nodes.0) } else { 0 };
    (kind_board || items > 0).then_some(BoardMarks { kind: kind_board, items })
}

/// Just the layers of a document, counted.
#[derive(Deserialize)]
struct Probe {
    #[serde(default)]
    nodes: Counted,
}

/// A layer: whether it carries a `board` object (`null` is none).
#[derive(Deserialize)]
struct Layer {
    #[serde(default)]
    board: Option<IgnoredAny>,
}

/// How many of a map's layers carry a `board` object, counted as they are read (nothing kept).
#[derive(Default)]
struct Counted(usize);

impl<'de> Deserialize<'de> for Counted {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Counted;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("the document's layers by id")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Counted, A::Error> {
                let mut n = 0;
                while map.next_key::<IgnoredAny>()?.is_some() {
                    n += usize::from(map.next_value::<Layer>()?.board.is_some());
                }
                Ok(Counted(n))
            }
        }
        d.deserialize_map(V)
    }
}
