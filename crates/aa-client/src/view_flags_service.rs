//! Client view part of `aa_runtime::view_flags_service` (native 0x4CFCC0
//! branch 3 and 0x50C8E0): view+0x54 of the view published at an actor's
//! Thing+0xC, read from the SAME `live_scene::view_directory::ViewDirectory`
//! entry (its `native_fields.flags()`, the word `ViewDirectory::flags_54`
//! returns). Nothing is cached: every call reads the entry.
//!
//! The simulation callback holds the ActorStore split, so the directory
//! validates the handle against the actor's GTID and ObjectKey instead of
//! an `&ActorStore` (`ViewDirectory::flags_for_current` needs the store).
//! The other parts (`SessionPublication`, `UiPublication`) and their
//! composition (`aa_runtime::view_flags_service::Composed`) are lent by the
//! netcode Session and battle-orders UIWorldState owners.
// Constructed by the frame owner once ViewDirectory implements
// DirectoryFlags (research/phase8-orders-input/action-mask-publication).
#![allow(dead_code)]
use aa_actors::ActorId;
use aa_runtime::{actor_world::ObjectKey, view_flags_service::{Result, ViewFlags}};

/// The ViewDirectory read: the positive Thing+0xC handle must name this
/// actor's current published view (same GTID and ObjectKey, publication
/// receipt, completed derived Init), as `flags_for_current` checks.
pub trait DirectoryFlags {
    fn flags_54_for_thing_c(&self, actor: ActorId, key: ObjectKey, thing_c: usize) -> anyhow::Result<u32>;
}

/// [`ViewFlags`] over the SAME directory, lent for one callback.
pub struct DirectoryViewFlags<'a, D: ?Sized>(pub &'a D);

impl<D: DirectoryFlags + ?Sized> ViewFlags for DirectoryViewFlags<'_, D> {
    fn view_flags_54(&self, actor: ActorId, key: ObjectKey, thing_c: usize) -> Result<u32> {
        if thing_c == 0 {
            return Err("view+0x54 read for a NULL Thing+0xC (0x4CFD0C tests it first)".into());
        }
        self.0.flags_54_for_thing_c(actor, key, thing_c).map_err(|e| format!("{e:#}"))
    }
}
