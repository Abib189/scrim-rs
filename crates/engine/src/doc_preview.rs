//! Showing a saved version in place of the document (split out of `lib.rs`).

use std::sync::Arc;

use figcraft_doc::Document;

use crate::{DocState, VersionPreview};

impl DocState {
    /// Show `version` in place of the document (or switch to it from another preview).
    pub fn start_preview(&mut self, id: &str, name: &str, time: u64, doc: Arc<Document>) {
        self.end_preview();
        self.preview = Some(VersionPreview {
            id: id.to_string(),
            name: name.to_string(),
            time,
            doc: std::mem::replace(&mut self.doc, doc),
            selection: std::mem::take(&mut self.selection),
            page: self.page,
        });
        self.page = Some(self.page)
            .filter(|pg| self.doc.pages.contains(pg))
            .or_else(|| self.doc.current_page.filter(|pg| self.doc.pages.contains(pg)))
            .or_else(|| self.doc.first_page())
            .unwrap_or(self.page);
        self.text_edit = None;
        self.vector_edit = None;
        self.interaction = None;
        self.revision += 1;
    }

    /// End a version preview, putting the document, selection and page back. False when none.
    pub fn end_preview(&mut self) -> bool {
        let Some(p) = self.preview.take() else { return false };
        self.doc = p.doc;
        self.selection = p.selection;
        self.page = p.page;
        self.text_edit = None;
        self.vector_edit = None;
        self.interaction = None;
        self.clean_selection();
        self.revision += 1;
        true
    }
}
