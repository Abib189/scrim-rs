//! A document's file as it is at one moment, encoded (the slow part of saving) and then written
//! (moved out of `cmd/file.rs`).

use super::*;

/// Thumbnail PNG of the first page's content (for file browsers / the home screen).
pub fn thumbnail(doc: &Document) -> Option<Vec<u8>> {
    let page = doc.first_page()?;
    let b = doc.page_content_bounds(page)?;
    let scale = (400.0 / b.width().max(b.height()).max(1.0)).min(2.0);
    let view = figcraft_render::View {
        origin: (b.x0, b.y0),
        scale,
        width: (b.width() * scale).ceil().max(1.0) as u32,
        height: (b.height() * scale).ceil().max(1.0) as u32,
    };
    let pix = figcraft_render::render_view(doc, page, &view, &Default::default())?;
    figcraft_render::encode_png(&pix).ok()
}

/// The `.figcraft` file for a document: content, thumbnail and version history.
pub fn file_bytes(ds: &DocState) -> Result<Vec<u8>> {
    FileSnapshot::of(ds).encode()
}

/// A document's file as it is at one moment, so the slow part of saving it (the thumbnail and the
/// encoding) can run on another thread while editing goes on: [`encode`](Self::encode) there, then
/// [`save`](Self::save) the bytes back on the session's thread.
pub struct FileSnapshot {
    uid: u64,
    path: Option<String>,
    revision: u64,
    doc: Arc<Document>,
    /// The document as last saved when the snapshot was taken: another save since makes it stale.
    saved: Arc<Document>,
    page: NodeId,
    versions: Vec<Version>,
    branches: Vec<figcraft_format::Branch>,
    /// Images the file holds that weren't loaded: copied across (Fix AI LW-3).
    left: Option<figcraft_format::carry::LeftInFile>,
}

impl FileSnapshot {
    pub fn of(ds: &DocState) -> Self {
        Self {
            uid: ds.uid,
            path: ds.path.clone(),
            revision: ds.revision,
            doc: ds.doc.clone(),
            saved: ds.saved_doc.clone(),
            page: ds.page,
            versions: ds.versions.clone(),
            branches: ds.branches.clone(),
            left: ds.images_left_in_file.clone(),
        }
    }

    /// The file's bytes: content, thumbnail and version history.
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut doc = (*self.doc).clone();
        doc.current_page = Some(self.page);
        let thumb = thumbnail(&doc);
        figcraft_format::save_bytes_keeping(&doc, thumb.as_deref(), &self.versions, &self.branches, self.left.as_ref())
            .map_err(|e| EngineError::Other(e.to_string()))
    }

    /// Write `bytes` (this snapshot's [`encode`](Self::encode)) to the document's file: it is saved
    /// as of the snapshot, and edits since stay unsaved. False (nothing written) when the document
    /// was closed, saved, reverted or given another path since: the bytes are out of date.
    pub fn save(self, s: &mut Session, bytes: &[u8]) -> Result<bool> {
        // Auto-save writes .scrim files only (Fix AG: a .figcraft it opened is never written).
        if !self.path.as_deref().is_some_and(saves_in_place) {
            return Ok(false);
        }
        let Some(ds) = s.docs.iter_mut().find(|d| d.uid == self.uid && d.path == self.path && Arc::ptr_eq(&d.saved_doc, &self.saved)) else {
            return Ok(false);
        };
        write(self.path.as_deref().ok_or(EngineError::NoDocument)?, bytes)?;
        // The history or branches may have changed since: then they still count as unsaved.
        if ds.revision == self.revision {
            ds.versions_changed = false;
        }
        ds.saved_doc = self.doc;
        ds.revision += 1;
        branch_tabs_saved(s, self.uid);
        Ok(true)
    }
}

impl DocState {
    /// With the images its file `path` holds that weren't loaded (past what Scrim reads from one
    /// file): every save copies them from the file it last read or wrote (Fix AI LW-3).
    pub fn with_images_left_in_file(mut self, path: &str, hashes: std::collections::BTreeSet<String>) -> Self {
        self.images_left_in_file = (!hashes.is_empty()).then(|| figcraft_format::carry::LeftInFile { file: path.into(), hashes });
        self
    }

    /// The images its file holds that weren't loaded are now in the file at `path` too; how many.
    pub(crate) fn images_left_now_in(&mut self, path: &str) -> usize {
        self.images_left_in_file.as_mut().map_or(0, |l| {
            l.file = path.into();
            l.hashes.len()
        })
    }
}
