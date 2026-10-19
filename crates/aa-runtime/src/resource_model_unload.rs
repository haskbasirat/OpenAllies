//!5ACA50, the ResourceManager pass that 5AE0A0 runs after a CHANGED Biome
//!texture subfolder was copied to Manager+80 (43F0F1, VWorld init prefix).
//!
//!Native evidence (research/development10/resource-model-unload):
//!- 5ACA50 walks every Manager+60 hash node (4245A0 iterator, bucket order,
//!  chain order). For each Resource it calls vf4 and walks the class
//!  descriptor's +8 parent chain looking for B3D990 KKC_NiAVObjResource
//!  (CRT 9D06E0). Texture B3D97C->B38E24, NiSequence B3D9A4->B3D968->B38E24,
//!  Audio B3D76C->B38E24, Map B3D7DC->B38E24 and Film B3D7F0->B3D7DC->B38E24
//!  never reach it, so those entries are skipped BEFORE +4/+8 are read.
//!- A KKC_NiAVObjResource (the actual Model class, vtable A45C98 from 5A7DB0)
//!  is unloaded through 5ABFF0 only when flags+8 bit5 (0x20, "in the
//!  ResourceCache LRU list") is set AND the signed request count +4 is <= 0.
//!  Requested Models and Models never admitted to the cache are untouched.
//!- Bit5 has exactly one writer, 59B9F0, reached only by
//!  KKC_ResourceManager::CacheOrUnloadResource 5AC3D0 <- ReleaseResource
//!  5AEE70 <- Resource::Release 5AA550 when the request count reaches zero,
//!  ResourceCaching (UVar54) is true, flags bit3 is clear and 59BF30/59BC70
//!  EnsureEnoughAvailableMemory can make room. 59BA90 (decache) and 59BFA0
//!  (re-request / unload) clear it again.
//!
//!The port has no Resource release/cache admission yet, so no Entry can carry
//!bit5 and the native skip is the whole observable effect here. A reached
//!eligible entry stays an explicit error: 5ABFF0's body (lock +20/+3C, clear
//!bit0 then vf40/vf54 and the "Unloaded" log when loaded, 59BFA0 cache removal
//!when bit5) is recorded by the oracle but its vf40/vf54/cache owners are not
//!ported.
use crate::resource_objects::{Entry, Kind};
type Result<T> = std::result::Result<T, String>;

///B3D990, the descriptor 5ACA90 compares every class-chain step against.
pub(crate) const NI_AVOBJ_RESOURCE_B3D990: u32 = 0xb3d990;

///vf4 descriptor followed by its 409750 +8 parents, as installed by the CRT
///registrations 9D06B0, 9D06E0, 9D0710, 9D02F0 and 9D0680/9C7FE0.
pub(crate) fn native_class_chain(kind: Kind) -> &'static [u32] {
    match kind {
        Kind::Texture => &[0xb3d97c, 0xb38e24],
        Kind::Model => &[0xb3d990, 0xb3d968, 0xb38e24],
        Kind::Sequence => &[0xb3d9a4, 0xb3d968, 0xb38e24],
        Kind::Sound => &[0xb3d76c, 0xb38e24],
        Kind::TerrainBlock => &[0xb4040c, 0xb38e24],
    }
}

///5ACA90..5ACAB7: class match, then `shr 5; test cl,1`, then signed `jg`.
///Flags and count are only meaningful (and natively only read) after the
///class test, which this ordering preserves.
pub(crate) fn eligible_5aca50(kind: Kind, flags8: impl FnOnce() -> u32, requests4: impl FnOnce() -> u32) -> bool {
    native_class_chain(kind).contains(&NI_AVOBJ_RESOURCE_B3D990)
        && (flags8() >> 5) & 1 != 0
        && requests4() as i32 <= 0
}

///5ACA50 over the concrete Texture/Model/Sequence/Sound entries of the SAME
///Manager+60 namespace. Every skip is side-effect free, so traversal order is
///unobservable unless an entry is eligible; that case refuses before any
///unload, retaining the caller's earlier +80 assignment.
pub(crate) fn invalidate_5aca50<'a>(entries: impl IntoIterator<Item = &'a Entry>) -> Result<()> {
    for entry in entries {
        if eligible_5aca50(entry.handle.kind(), || entry.flags8, || entry.requests4) {
            return Err(format!(
                "5ACA50 cached unrequested NiAVObjResource {} requires 5ABFF0 unload (Model vf40 5A6560/vf54 5A7CF0, 59BFA0 ResourceCache removal); new+80 retained",
                String::from_utf16_lossy(entry.handle.logical_ids())
            ));
        }
    }
    Ok(())
}

///The entries 5ACA50 passes to 5ABFF0, in this namespace's traversal order.
///Each unload changes only its own Resource and class list, so the final
///state is independent of the native bucket order (only the "Unloaded" log
///lines would be reordered).
pub(crate) fn eligible_handles_5aca50<'a>(entries: impl IntoIterator<Item = &'a Entry>) -> Vec<crate::resource_objects::Handle> {
    entries
        .into_iter()
        .filter(|e| eligible_5aca50(e.handle.kind(), || e.flags8, || e.requests4))
        .map(|e| e.handle.clone())
        .collect()
}

#[cfg(test)]
#[path = "resource_model_unload_tests.rs"]
mod tests;
