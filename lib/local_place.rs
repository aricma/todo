//! Which place the local store lives in — the local *File* layer's decision,
//! kept pure so it is testable without a browser (ADR-0009). Decision:
//! ADR-0020. Design: `docs/plans/2026-09-21-local-place-layer.md`.
//!
//! A source's store bytes may live in three places, most preferred first: the
//! OPFS directory (`Opfs`, the durable VFS), a whole-file mirror in IndexedDB
//! (`IndexedDb`, durable across a reload), and the page-lifetime in-memory store
//! (`Memory`). Which one a boot uses is a pure question — what the browser can
//! provide, and which places already hold bytes for this source — so it lives
//! here, where `cargo test` reaches it, and the bin only carries out the answer.

/// A place the local store's bytes can live in. The variant order is the
/// preference order; [`PREFERENCE`] is the explicit statement of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlaceKind {
    /// The OPFS VFS — durable, synchronous, needs cross-origin isolation and a
    /// usable OPFS root.
    Opfs,
    /// An in-memory SQLite mirrored whole-file into IndexedDB — durable across a
    /// reload, cleared when the browser clears site data or a private session
    /// ends.
    IndexedDb,
    /// An in-memory SQLite — works anywhere, survives nothing.
    Memory,
}

impl PlaceKind {
    /// A stable name, for logs and messages.
    pub fn as_str(self) -> &'static str {
        match self {
            PlaceKind::Opfs => "opfs",
            PlaceKind::IndexedDb => "indexeddb",
            PlaceKind::Memory => "memory",
        }
    }
}

/// The places in preference order: OPFS, then IndexedDB, then memory.
pub const PREFERENCE: [PlaceKind; 3] = [PlaceKind::Opfs, PlaceKind::IndexedDb, PlaceKind::Memory];

/// What the browser can provide, as far as the resolver is concerned. A place
/// whose capability is false is never chosen and never probed; `Memory` is
/// always available and so has no flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Capabilities {
    pub opfs: bool,
    pub indexed_db: bool,
}

impl Capabilities {
    fn provides(self, kind: PlaceKind) -> bool {
        match kind {
            PlaceKind::Opfs => self.opfs,
            PlaceKind::IndexedDb => self.indexed_db,
            PlaceKind::Memory => true,
        }
    }
}

/// The resolver's answer: which place to open, and — when the target is empty
/// and a lower place holds bytes — which copy to adopt into it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resolution {
    /// The place to open.
    pub target: PlaceKind,
    /// The place to read the store's bytes from first, when the target itself
    /// holds none. `None` means the target is opened as it is (it holds bytes)
    /// or created empty.
    pub adopt_from: Option<PlaceKind>,
}

/// Choose where this source's store opens.
///
/// `holds` is the places that already hold bytes for this source's key — probed
/// by the caller, since only it can do I/O. The rule is **look before you
/// create**:
///
/// * the most-preferred place the browser can provide is the target;
/// * if the target already holds bytes, it opens as it is and lower copies are
///   **left alone** — a lower copy may be newer, and that divergence is not the
///   resolver's to guess about (it is the parked out-of-sync question);
/// * if the target is empty and a lower place holds bytes, those are adopted
///   into the target; the caller deletes the adopted copy only once the target
///   is open and healthy;
/// * otherwise the target is created empty.
pub fn resolve(caps: &Capabilities, holds: &[PlaceKind]) -> Resolution {
    let target = PREFERENCE
        .into_iter()
        .find(|kind| caps.provides(*kind))
        // `Memory` always provides, so the iterator never runs out.
        .unwrap_or(PlaceKind::Memory);
    let adopt_from = if holds.contains(&target) {
        None
    } else {
        PREFERENCE
            .into_iter()
            .skip_while(|kind| *kind != target)
            .skip(1)
            .find(|kind| holds.contains(kind))
    };
    Resolution { target, adopt_from }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caps(opfs: bool, indexed_db: bool) -> Capabilities {
        Capabilities { opfs, indexed_db }
    }

    #[test]
    fn the_most_preferred_available_place_is_the_target() {
        assert_eq!(resolve(&caps(true, true), &[]).target, PlaceKind::Opfs);
        assert_eq!(
            resolve(&caps(false, true), &[]).target,
            PlaceKind::IndexedDb
        );
        assert_eq!(resolve(&caps(false, false), &[]).target, PlaceKind::Memory);
        // A place that is not provided is never chosen, even when it holds bytes.
        assert_eq!(
            resolve(&caps(false, false), &[PlaceKind::Opfs]).target,
            PlaceKind::Memory
        );
    }

    #[test]
    fn a_target_that_holds_bytes_opens_as_is_and_leaves_lower_copies_alone() {
        let r = resolve(&caps(true, true), &[PlaceKind::Opfs, PlaceKind::IndexedDb]);
        assert_eq!(r.target, PlaceKind::Opfs);
        assert_eq!(r.adopt_from, None);
    }

    #[test]
    fn an_empty_target_adopts_the_most_preferred_lower_copy() {
        let r = resolve(
            &caps(true, true),
            &[PlaceKind::IndexedDb, PlaceKind::Memory],
        );
        assert_eq!(r.target, PlaceKind::Opfs);
        assert_eq!(r.adopt_from, Some(PlaceKind::IndexedDb));
    }

    #[test]
    fn a_target_that_holds_bytes_is_not_beaten_by_a_lower_copy() {
        let r = resolve(
            &caps(false, true),
            &[PlaceKind::IndexedDb, PlaceKind::Memory],
        );
        assert_eq!(r.target, PlaceKind::IndexedDb);
        assert_eq!(r.adopt_from, None);
    }

    #[test]
    fn adoption_prefers_indexeddb_over_memory() {
        let r = resolve(&caps(false, true), &[PlaceKind::Memory]);
        assert_eq!(r.target, PlaceKind::IndexedDb);
        assert_eq!(r.adopt_from, Some(PlaceKind::Memory));
    }

    #[test]
    fn nothing_holds_bytes_so_the_target_is_created() {
        let r = resolve(&caps(true, true), &[]);
        assert_eq!(r.target, PlaceKind::Opfs);
        assert_eq!(r.adopt_from, None);
    }

    #[test]
    fn preference_order_is_opfs_then_indexeddb_then_memory() {
        assert_eq!(
            PREFERENCE,
            [PlaceKind::Opfs, PlaceKind::IndexedDb, PlaceKind::Memory]
        );
    }
}
