//! Reading a sector storage object's contents (API v131, issue #387).
//!
//! The cockpit could see a detached container sitting in the sector and act on
//! it — recover it, inspect it — but never say what was inside. Requested by
//! the game's own author, who exposes the same view in the reference WebUI.
//!
//! Three properties shape this.
//!
//! **It is read lazily, on the drill.** One request per container, and a pilot
//! who never looks never pays it — the same rule the logbook follows (#254).
//!
//! **It is paginated by an opaque cursor**, and the server revalidates access
//! on *every* page, so the loop cannot assume the first page's permission
//! holds. Pages accumulate into one view rather than being shown one at a time:
//! "what is in this container" is a single answer, not a slideshow.
//!
//! **A 409 restarts it.** The cursor is versioned against the contents, not
//! merely positional, so a container that changed under us invalidates
//! everything read so far. Restarting from an empty accumulator is the only
//! honest response; showing half of one version stitched to half of another
//! would be a number nobody could act on.

use super::*;
use crate::api::types::{SectorObjectType, SectorStorageInventory, SectorStorageItem, SectorStorageResource};

/// How many pages to follow before giving up.
///
/// The server caps a page at 500 entries, so this is far past any real
/// container; it exists because a cursor that never returns `None` would
/// otherwise spend the rate-limit window in a loop (#332).
pub const SECTOR_STORAGE_MAX_PAGES: usize = 20;

/// How many times a 409 may restart the read before we stop and say so.
///
/// The page cap alone does not bound this: a restart resets the page count, so
/// a container being written to continuously would loop forever. Two retries
/// is enough for an ordinary race and short enough that a busy container
/// reports itself instead of quietly spending the quota.
pub const SECTOR_STORAGE_MAX_RESTARTS: usize = 2;

/// Contents of one sector storage object, accumulated across pages.
#[derive(Debug, Clone, Default)]
pub struct SectorStorageView {
    /// The object being read; a late page arriving for a different one is
    /// dropped rather than merged, since the pilot may have drilled elsewhere.
    pub object_id: String,
    pub resources: Vec<SectorStorageResource>,
    pub items: Vec<SectorStorageItem>,
    /// Pages already folded in — what makes the restart-on-409 observable.
    pub pages: usize,
    /// How many times a 409 has sent us back to the first page.
    pub restarts: usize,
    /// True while a further page is on the wire.
    pub loading: bool,
}

impl SectorStorageView {
    /// Total space the listed items occupy, in ECE.
    pub fn items_space(&self) -> f64 {
        self.items.iter().map(|i| i.container_space).sum()
    }

    /// Whether anything at all was found. An empty container is a real answer
    /// and must read differently from "still loading".
    pub fn is_empty(&self) -> bool {
        self.resources.is_empty() && self.items.is_empty()
    }
}

impl AppState {
    /// Begin reading a sector object's contents, discarding anything held for
    /// a previous one.
    pub fn start_sector_storage(&mut self, object_id: String) {
        self.sector_storage = Some(SectorStorageView {
            object_id,
            loading: true,
            ..Default::default()
        });
        self.sector_storage_error = None;
    }

    /// Fold one page in. Returns the cursor to fetch next, if any.
    ///
    /// A page for an object we are no longer reading is dropped: the pilot
    /// drilled out or moved on, and merging it would attribute one container's
    /// contents to another.
    pub fn merge_sector_storage(&mut self, page: SectorStorageInventory) -> Option<String> {
        let expected = self.sector_storage.as_ref().map(|v| v.object_id.clone())?;
        if let Some(id) = &page.object_id {
            if id != &expected {
                return None;
            }
        }
        let view = self.sector_storage.as_mut()?;
        view.resources.extend(page.resources);
        view.items.extend(page.items);
        view.pages += 1;
        let next = page.next_cursor.filter(|_| view.pages < SECTOR_STORAGE_MAX_PAGES);
        view.loading = next.is_some();
        next
    }

    /// The contents changed under the cursor (409). Everything read so far
    /// belongs to a version that no longer exists, so the accumulator is
    /// emptied and the read restarts from the first page.
    ///
    /// Returns `None` once [`SECTOR_STORAGE_MAX_RESTARTS`] is spent: a
    /// container being written to continuously would otherwise loop forever,
    /// since a restart resets the page count the other cap counts.
    pub fn restart_sector_storage(&mut self) -> Option<String> {
        let view = self.sector_storage.as_mut()?;
        if view.restarts >= SECTOR_STORAGE_MAX_RESTARTS {
            view.loading = false;
            self.sector_storage_error = Some("contents kept changing while reading".into());
            return None;
        }
        view.restarts += 1;
        view.resources.clear();
        view.items.clear();
        view.pages = 0;
        view.loading = true;
        Some(view.object_id.clone())
    }

    pub fn fail_sector_storage(&mut self, msg: String) {
        if let Some(v) = self.sector_storage.as_mut() {
            v.loading = false;
        }
        self.sector_storage_error = Some(msg);
    }

    pub fn clear_sector_storage(&mut self) {
        self.sector_storage = None;
        self.sector_storage_error = None;
    }

    /// Whether this scanned object's contents can be read.
    ///
    /// The server serves the endpoint for drifting containers, containers
    /// personally known to be hidden on an asteroid, and "other storage objects
    /// whose access this probe has discovered". The cockpit offers it on the
    /// object kind it can actually name — a detached container — and lets a
    /// 403/404 speak for the rest rather than guessing at a broader rule.
    pub fn sector_storage_readable(&self, entry: &ScannerObjectEntry) -> bool {
        entry.provenance == ObjectProvenance::TopLevel
            && matches!(entry.object_type, SectorObjectType::DetachedContainer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(resources: &str, items: &str, next: &str) -> SectorStorageInventory {
        serde_json::from_str(&format!(
            r#"{{"objectId": "c1", "resources": [{resources}], "items": [{items}],
                 "nextCursor": {next}}}"#
        ))
        .unwrap()
    }

    const METALS: &str = r#"{"type": "metals", "amount": 2.0, "reservedAmount": 0.5, "availableAmount": 1.5}"#;
    const ITEM: &str = r#"{"id": "i1", "type": "steel_plate", "name": "Steel plate", "containerSpace": 0.1,
            "available": true, "metadata": {}}"#;

    #[test]
    fn pages_accumulate_into_one_answer() {
        let mut s = AppState::default();
        s.start_sector_storage("c1".into());
        assert!(s.sector_storage.as_ref().unwrap().loading);

        let next = s.merge_sector_storage(page(METALS, ITEM, "\"cur2\""));
        assert_eq!(next.as_deref(), Some("cur2"), "a cursor means another page");
        assert!(s.sector_storage.as_ref().unwrap().loading);

        let next = s.merge_sector_storage(page(METALS, ITEM, "null"));
        assert!(next.is_none(), "the last page ends the loop");
        let v = s.sector_storage.as_ref().unwrap();
        assert_eq!(v.resources.len(), 2);
        assert_eq!(v.items.len(), 2);
        assert_eq!(v.pages, 2);
        assert!(!v.loading);
        assert!((v.items_space() - 0.2).abs() < 1e-9);
    }

    #[test]
    fn a_change_under_the_cursor_restarts_from_empty() {
        // The cursor is versioned against the contents, so half of one version
        // stitched to half of another would be a number nobody could act on.
        let mut s = AppState::default();
        s.start_sector_storage("c1".into());
        s.merge_sector_storage(page(METALS, ITEM, "\"cur2\""));
        assert_eq!(s.sector_storage.as_ref().unwrap().items.len(), 1);

        let restart = s.restart_sector_storage();
        assert_eq!(restart.as_deref(), Some("c1"), "restarts on the same object");
        let v = s.sector_storage.as_ref().unwrap();
        assert!(v.is_empty(), "everything read so far is discarded");
        assert_eq!(v.pages, 0);
        assert!(v.loading);
    }

    #[test]
    fn a_container_that_keeps_changing_gives_up_instead_of_looping() {
        // A restart resets the page count, so the page cap cannot bound this.
        let mut s = AppState::default();
        s.start_sector_storage("c1".into());
        for _ in 0..SECTOR_STORAGE_MAX_RESTARTS {
            assert!(s.restart_sector_storage().is_some());
        }
        assert!(s.restart_sector_storage().is_none(), "stops rather than looping");
        assert!(s
            .sector_storage_error
            .as_deref()
            .is_some_and(|e| e.contains("changing")));
        assert!(!s.sector_storage.as_ref().unwrap().loading);
    }

    #[test]
    fn a_page_for_another_object_is_dropped() {
        // The pilot drilled elsewhere while it was on the wire; merging would
        // attribute one container's contents to another.
        let mut s = AppState::default();
        s.start_sector_storage("c1".into());
        let stray: SectorStorageInventory =
            serde_json::from_str(r#"{"objectId": "OTHER", "resources": [], "items": [], "nextCursor": null}"#).unwrap();
        assert!(s.merge_sector_storage(stray).is_none());
        assert_eq!(s.sector_storage.as_ref().unwrap().pages, 0, "nothing folded in");
    }

    #[test]
    fn a_runaway_cursor_stops_at_the_page_cap() {
        let mut s = AppState::default();
        s.start_sector_storage("c1".into());
        // A server that always hands back a cursor would otherwise spend the
        // whole rate-limit window in this loop.
        let mut next = Some("x".to_string());
        let mut rounds = 0;
        while next.is_some() && rounds < 100 {
            next = s.merge_sector_storage(page("", "", "\"more\""));
            rounds += 1;
        }
        assert_eq!(rounds, SECTOR_STORAGE_MAX_PAGES, "bounded");
        assert!(!s.sector_storage.as_ref().unwrap().loading);
    }

    #[test]
    fn an_empty_container_is_not_a_loading_one() {
        let mut s = AppState::default();
        s.start_sector_storage("c1".into());
        s.merge_sector_storage(page("", "", "null"));
        let v = s.sector_storage.as_ref().unwrap();
        assert!(v.is_empty() && !v.loading, "an empty container is a real answer");
    }
}
