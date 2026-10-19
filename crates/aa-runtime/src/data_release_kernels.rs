//!Native leaf kernels of the Data vf18 release / vf1C ready family. Every
//!Resource vf14 (5AA550) is a caller callback; readiness reads only the
//!Resource flags+8 bit0 the caller reports. Proof (original code under
//!Unicorn): research/skirmish-20261019-data-release-ready.
type Result<T> = std::result::Result<T, String>;

///Cells of one 428900/428950 list in native order ({node, next}).
pub(crate) trait FxNodes {
    type Node;
    ///node vf24 (CreateVFX class) and vf2C (PlaySFX class).
    fn creates_vfx(&mut self, node: &Self::Node) -> Result<bool>;
    fn plays_sfx(&mut self, node: &Self::Node) -> Result<bool>;
    ///(+3C VFX)->vf18 / vf1C; (+44 AudioData)->vf18 / vf1C.
    fn release_vfx(&mut self, node: &Self::Node, delta: u32) -> Result<()>;
    fn vfx_ready(&mut self, node: &Self::Node) -> Result<bool>;
    fn release_sfx(&mut self, node: &Self::Node, delta: u32) -> Result<()>;
    fn sfx_ready(&mut self, node: &Self::Node) -> Result<bool>;
}
///428900
pub(crate) fn fx_list_release<W: FxNodes>(w: &mut W, nodes: &[W::Node], delta: u32) -> Result<()> {
    for node in nodes {
        if w.creates_vfx(node)? {
            w.release_vfx(node, delta)?;
        }
        if w.plays_sfx(node)? {
            w.release_sfx(node, delta)?;
        }
    }
    Ok(())
}
///428950: the first false member ends the walk.
pub(crate) fn fx_list_ready<W: FxNodes>(w: &mut W, nodes: &[W::Node]) -> Result<bool> {
    for node in nodes {
        if w.creates_vfx(node)? && !w.vfx_ready(node)? {
            return Ok(false);
        }
        if w.plays_sfx(node)? && !w.sfx_ready(node)? {
            return Ok(false);
        }
    }
    Ok(true)
}
///4BE450: (AbilityData+0)->Definition lists +70+8i, i<5.
pub(crate) fn ability_release(delta: u32, mut list: impl FnMut(u32, u32) -> Result<()>) -> Result<()> {
    for i in 0..5 {
        list(0x70 + 8 * i, delta)?;
    }
    Ok(())
}
///4BE480
pub(crate) fn ability_ready(mut list: impl FnMut(u32) -> Result<bool>) -> Result<bool> {
    for i in 0..5 {
        if !list(0x70 + 8 * i)? {
            return Ok(false);
        }
    }
    Ok(true)
}
///One VFX ThingData: 4BB050/4BB060 on Data+20 and the optional +40 Resource.
pub(crate) trait VfxRelease {
    type Resource;
    fn release_base(&mut self, delta: u32) -> Result<()>;
    fn requested(&mut self) -> Result<bool>;
    fn resource40(&mut self) -> Result<Option<Self::Resource>>;
    fn release(&mut self, resource: &Self::Resource, delta: u32) -> Result<()>;
    fn loaded(&mut self, resource: &Self::Resource) -> Result<bool>;
}
///vf18: 494390 (4BB050 only) or, for the Resource40 classes, 494450
///(4BB050 then a nonnull +40 vf14).
pub(crate) fn vfx_release<W: VfxRelease>(w: &mut W, resource_class: bool, delta: u32) -> Result<()> {
    w.release_base(delta)?;
    if resource_class && let Some(r) = w.resource40()? {
        w.release(&r, delta)?;
    }
    Ok(())
}
///vf1C: 4943A0 (4BB060) or 494810 (4BB060, then +40 null or loaded).
pub(crate) fn vfx_ready<W: VfxRelease>(w: &mut W, resource_class: bool) -> Result<bool> {
    if !w.requested()? {
        return Ok(false);
    }
    if !resource_class {
        return Ok(true);
    }
    match w.resource40()? {
        None => Ok(true),
        Some(r) => w.loaded(&r),
    }
}
///The VFX vtable class split proven for vf14 (494380/494480) holds for
///vf18 (494390/494450) and vf1C (4943A0/494810) of the same vtables.
pub(crate) fn vfx_resource_class(kind: u32) -> Result<bool> {
    match kind {
        5 | 6 | 7 | 8 | 9 | 12 | 13 | 15 | 16 | 22 | 23 => Ok(false),
        10 | 11 | 14 | 17 => Ok(true),
        _ => Err("VFX Data vf18/vf1C constructor dispatch is unbound".into()),
    }
}
///5AA550 Resource vf14 and the Resource flags+8 bit0 read.
pub(crate) trait ResourceRelease {
    fn release_14(&mut self, handle: &crate::resource_objects::Handle, delta: u32) -> Result<()>;
    fn loaded(&mut self, handle: &crate::resource_objects::Handle) -> Result<bool>;
}
///46B260: lists +34/+40/+4C, each count reread (immutable here).
pub(crate) fn audio_release<R>(lists: [&[R]; 3], delta: u32, mut release: impl FnMut(&R, u32) -> Result<()>) -> Result<()> {
    for list in lists {
        for sound in list {
            release(sound, delta)?;
        }
    }
    Ok(())
}
///46B060: each count captured; every Sound must be loaded.
pub(crate) fn audio_ready<R>(lists: [&[R]; 3], mut loaded: impl FnMut(&R) -> Result<bool>) -> Result<bool> {
    for list in lists {
        for sound in list {
            if !loaded(sound)? {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
///49B820 / 49B860 over ObjectAnimation entries' +1C Resource (null admitted).
pub(crate) fn set_release<R>(entries: &[Option<R>], delta: u32, mut release: impl FnMut(&R, u32) -> Result<()>) -> Result<()> {
    for r in entries.iter().flatten() {
        release(r, delta)?;
    }
    Ok(())
}
pub(crate) fn set_ready<R>(entries: &[Option<R>], mut loaded: impl FnMut(&R) -> Result<bool>) -> Result<bool> {
    for r in entries.iter().flatten() {
        if !loaded(r)? {
            return Ok(false);
        }
    }
    Ok(true)
}
///One Aspect allocation for 469BB0/469C50.
pub(crate) trait AspectRelease {
    type Resource;
    ///+18
    fn count(&mut self) -> Result<u32>;
    fn store_count(&mut self, value: u32) -> Result<()>;
    ///+28, dereferenced unconditionally.
    fn model(&mut self) -> Result<Self::Resource>;
    fn release(&mut self, resource: &Self::Resource, delta: u32) -> Result<()>;
    fn loaded(&mut self, resource: &Self::Resource) -> Result<bool>;
    ///+38 ObjectAnimationSet: 49B820 / 49B860 when nonnull.
    fn release_animations(&mut self, delta: u32) -> Result<()>;
    fn animations_ready(&mut self) -> Result<bool>;
    ///+30/+34
    fn skin(&mut self, index: usize) -> Result<Option<Self::Resource>>;
    ///+54
    fn extra_requested(&mut self) -> Result<bool>;
    fn clear_extra_requested(&mut self) -> Result<()>;
    ///5A4ED0 Model+7C, then +48 8-byte elements' +0 Resource.
    fn extra_count(&mut self, model: &Self::Resource) -> Result<i32>;
    fn extra(&mut self, index: usize) -> Result<Self::Resource>;
}
///469BB0. The signed subtraction is clamped to zero only when negative.
pub(crate) fn aspect_release<W: AspectRelease>(w: &mut W, delta: u32) -> Result<()> {
    let count = w.count()?.wrapping_sub(delta);
    w.store_count(count)?;
    if (count as i32) < 0 {
        w.store_count(0)?;
    }
    let model = w.model()?;
    w.release(&model, delta)?;
    w.release_animations(delta)?;
    for index in 0..2 {
        if let Some(skin) = w.skin(index)? {
            w.release(&skin, delta)?;
        }
    }
    if w.extra_requested()? {
        let model = w.model()?;
        let count = w.extra_count(&model)?;
        for index in 0..count.max(0) as usize {
            let extra = w.extra(index)?;
            w.release(&extra, delta)?;
        }
        if w.count()? == 0 {
            w.clear_extra_requested()?;
        }
    }
    Ok(())
}
///469C50
pub(crate) fn aspect_ready<W: AspectRelease>(w: &mut W) -> Result<bool> {
    let model = w.model()?;
    if !w.loaded(&model)? || !w.animations_ready()? {
        return Ok(false);
    }
    for index in 0..2 {
        if let Some(skin) = w.skin(index)?
            && !w.loaded(&skin)?
        {
            return Ok(false);
        }
    }
    if w.extra_requested()? {
        let model = w.model()?;
        let count = w.extra_count(&model)?;
        for index in 0..count.max(0) as usize {
            let extra = w.extra(index)?;
            if !w.loaded(&extra)? {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
///46C7D0 -> Plane vf4 46C860 -> Decal vf4 46C920 -> Texture vf14; every
///count captured. 46C800/46C890/46C950 are the early-false ready walk.
pub(crate) fn banner_release<R>(planes: &[Vec<Vec<R>>], delta: u32, mut release: impl FnMut(&R, u32) -> Result<()>) -> Result<()> {
    for plane in planes {
        for decal in plane {
            for texture in decal {
                release(texture, delta)?;
            }
        }
    }
    Ok(())
}
pub(crate) fn banner_ready<R>(planes: &[Vec<Vec<R>>], mut loaded: impl FnMut(&R) -> Result<bool>) -> Result<bool> {
    for plane in planes {
        for decal in plane {
            for texture in decal {
                if !loaded(texture)? {
                    return Ok(false);
                }
            }
        }
    }
    Ok(true)
}
///One Actor Animation for 45EAC0 (vf4) / 45EB10.
pub(crate) trait AnimationRelease {
    type Resource;
    type List;
    ///+34 Sequence
    fn resource(&mut self) -> Result<Option<Self::Resource>>;
    fn release(&mut self, resource: &Self::Resource, delta: u32) -> Result<()>;
    fn loaded(&mut self, resource: &Self::Resource) -> Result<bool>;
    ///+38/+40/+48 then the +50 timed-cue chain (node+4), in that order.
    fn lists(&mut self) -> Result<Vec<Self::List>>;
    fn release_list(&mut self, list: &Self::List, delta: u32) -> Result<()>;
    fn list_ready(&mut self, list: &Self::List) -> Result<bool>;
}
pub(crate) fn animation_release<W: AnimationRelease>(w: &mut W, delta: u32) -> Result<()> {
    if let Some(r) = w.resource()? {
        w.release(&r, delta)?;
    }
    for list in w.lists()? {
        w.release_list(&list, delta)?;
    }
    Ok(())
}
pub(crate) fn animation_ready<W: AnimationRelease>(w: &mut W) -> Result<bool> {
    if let Some(r) = w.resource()?
        && !w.loaded(&r)?
    {
        return Ok(false);
    }
    for list in w.lists()? {
        if !w.list_ready(&list)? {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(test)]
#[path = "data_release_kernels_tests.rs"]
mod tests;
