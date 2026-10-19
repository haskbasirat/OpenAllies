//!KKC_ResourceCacheManager at ResourceManager+9C: per-class LRU lists of
//!released, still-loaded Resources (flags+8 bit5) and their accounting.
//!
//!Native layout (research/development10/resource-release): cache+20/+24 class
//!hash keyed by the Resource vf0 class id, +30/+3C total RAM/VRAM in the cache,
//!+34/+40 budgets (59B6B0 SetSize: ResourceCachePercentOfTotalRAM/VRAM of the
//!machine), +38/+44 weight sums. Class entry (59C360): +C/+10 RAM used/peak,
//!+1C/+20 VRAM used/peak, +8/+18 caps (59C040), +28 cache, +2C/+30 LRU head/
//!tail; a Resource links through +18 prev/+1C next.
//!
//!Removed cap (no-limits policy): the 59B6B0/59C040 budgets and the eviction
//!they drive (59BC70 EnsureEnoughAvailableMemory and 59BB70 ReduceCacheToSize
//!"Decaching", 59DA00 out-of-memory refusal) are not imposed: admission always
//!succeeds, as on a machine with enough memory. Those budgets are 25% of the
//!host's physical RAM / device VRAM, so nothing deterministic in the game can
//!depend on them; eviction only ever unloads a released (count 0) cached
//!Resource, whose next request reloads the same bytes. Accounting, LRU order
//!and bit5 are kept exactly. Class rows start with 59C360's zeroed fields and
//!are materialised on first use (their weights only feed the removed caps).
use crate::resource_objects::{Entry, Handle, Kind};
type Result<T> = std::result::Result<T, String>;

#[derive(Default)]
pub(crate) struct Class {
    pub(crate) ram_used0c: u32,
    pub(crate) ram_peak10: u32,
    pub(crate) vram_used1c: u32,
    pub(crate) vram_peak20: u32,
    ///+2C head first; +30 tail last.
    pub(crate) lru: Vec<Handle>,
}
#[derive(Default)]
pub(crate) struct State {
    classes: [Option<Class>; 5],
    pub(crate) ram_used30: u32,
    pub(crate) vram_used3c: u32,
}
fn slot(kind: Kind) -> usize {
    match kind {
        Kind::Texture => 0,
        Kind::Model => 1,
        Kind::Sequence => 2,
        Kind::Sound => 3,
        Kind::TerrainBlock => 4,
    }
}
impl State {
    #[cfg(test)]
    pub(crate) fn class(&self, kind: Kind) -> Option<&Class> {
        self.classes[slot(kind)].as_ref()
    }
    #[cfg(test)]
    pub(crate) fn class_mut_for_test(&mut self, kind: Kind) -> &mut Class {
        self.classes[slot(kind)].get_or_insert_with(Class::default)
    }
    ///59BC70 with the removed budget: the `(cap-used)<need` loop never runs
    ///and the renderer-thread shortcut returns the same 1.
    fn ensure_available_59bc70(&self, _ram: u32, _vram: u32) -> bool {
        true
    }
    ///59BF30: class lookup, 59BC70, then 59B9F0 tail append, 59B8C0
    ///accounting (+C/+10 RAM, +1C/+20 VRAM peaks, cache totals) and bit5.
    pub(crate) fn admit_59bf30(&mut self, entry: &mut Entry) -> Result<bool> {
        if entry.flags8 & 0x20 != 0 {
            return Err("59B9F0 Resource is already linked in a ResourceCache list".into());
        }
        let (ram, vram) = (entry.memory0c, entry.memory10);
        if !self.ensure_available_59bc70(ram, vram) {
            return Ok(false);
        }
        let class = self.classes[slot(entry.handle.kind())].get_or_insert_with(Class::default);
        class.lru.try_reserve(1).map_err(|e| e.to_string())?;
        class.lru.push(entry.handle.clone());
        class.ram_used0c = class.ram_used0c.wrapping_add(ram);
        class.vram_used1c = class.vram_used1c.wrapping_add(vram);
        if class.ram_used0c > class.ram_peak10 {
            class.ram_peak10 = class.ram_used0c;
        }
        if class.vram_used1c > class.vram_peak20 {
            class.vram_peak20 = class.vram_used1c;
        }
        self.ram_used30 = self.ram_used30.wrapping_add(ram);
        self.vram_used3c = self.vram_used3c.wrapping_add(vram);
        entry.flags8 |= 0x20;
        Ok(true)
    }
    ///59BFA0 then 59BA90: unlink, subtract the CURRENT +C/+10 sizes from the
    ///class and cache totals, clear bit5. Reached from 5B0270 re-request and
    ///5ABFF0 unload.
    pub(crate) fn remove_59bfa0(&mut self, entry: &mut Entry) -> Result<()> {
        let class = self.classes[slot(entry.handle.kind())]
            .as_mut()
            .ok_or("59BFA0 ResourceCache class entry is absent for a cached Resource")?;
        let index = class
            .lru
            .iter()
            .position(|h| h.same_identity(&entry.handle))
            .ok_or("59BA90 cached Resource is not linked in its class list")?;
        class.lru.remove(index);
        let (ram, vram) = (entry.memory0c, entry.memory10);
        class.ram_used0c = class.ram_used0c.wrapping_sub(ram);
        class.vram_used1c = class.vram_used1c.wrapping_sub(vram);
        self.ram_used30 = self.ram_used30.wrapping_sub(ram);
        self.vram_used3c = self.vram_used3c.wrapping_sub(vram);
        entry.flags8 &= !0x20;
        Ok(())
    }
}
