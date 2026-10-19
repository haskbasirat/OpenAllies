//!Borrowed native Data vf18 release / vf1C ready dispatch: ObjectData
//!49BB00/49BBB0 and ActorData 45DF40 (A05540+18) / 45EF20 (A05540+1C).
//!Proof: research/skirmish-20261019-data-release-ready (original code under
//!Unicorn; child seams record order, arguments and which counts are reread).
//!Leaves own their native kernels; nothing here manufactures readiness.
type Result<T> = std::result::Result<T, String>;
pub(super) trait ObjectRelease {
    type Data: Clone;
    type Aspect: Clone;
    ///4BB050 Data+20 -= delta (wrapping, no clamp).
    fn release_base(&mut self, delta: u32) -> Result<()>;
    ///4BB060 signed Data+20 > 0.
    fn requested_base(&mut self) -> Result<bool>;
    ///+104
    fn portrait(&mut self) -> Result<Option<Self::Data>>;
    ///Data vf18 / vf1C of another Data allocation.
    fn release_data(&mut self, data: Self::Data, delta: u32) -> Result<()>;
    fn data_ready(&mut self, data: Self::Data) -> Result<bool>;
    ///+134 nonnull -> 46C7D0 / 46C800; null releases nothing and is ready.
    fn release_banner(&mut self, delta: u32) -> Result<()>;
    fn banner_ready(&mut self) -> Result<bool>;
    ///+110 signed count, reread before each element.
    fn aspect_count(&mut self) -> Result<isize>;
    fn aspect(&mut self, index: usize) -> Result<Self::Aspect>;
    ///469BB0 / 469C50
    fn release_aspect(&mut self, aspect: Self::Aspect, delta: u32) -> Result<()>;
    fn aspect_ready(&mut self, aspect: Self::Aspect) -> Result<bool>;
    ///+EC/+F0/+138 Resource: nonnull vf14(delta); ready = null or flags+8 bit0.
    fn release_texture(&mut self, offset: u32, delta: u32) -> Result<()>;
    fn texture_ready(&mut self, offset: u32) -> Result<bool>;
    ///428900 / 428950 on the six View lists +BC+8i.
    fn release_view_fx(&mut self, index: usize, delta: u32) -> Result<()>;
    fn view_fx_ready(&mut self, index: usize) -> Result<bool>;
}
///49BB00
pub(super) fn object_release(w: &mut impl ObjectRelease, delta: u32) -> Result<()> {
    w.release_base(delta)?;
    if let Some(portrait) = w.portrait()? {
        w.release_data(portrait, delta)?;
    }
    w.release_banner(delta)?;
    let mut index = 0usize;
    while (index as isize) < w.aspect_count()? {
        let aspect = w.aspect(index)?;
        w.release_aspect(aspect, delta)?;
        index += 1;
    }
    for offset in [0xec, 0xf0, 0x138] {
        w.release_texture(offset, delta)?;
    }
    for index in 0..6 {
        w.release_view_fx(index, delta)?;
    }
    Ok(())
}
///49BBB0: each false child returns immediately.
pub(super) fn object_ready(w: &mut impl ObjectRelease) -> Result<bool> {
    if !w.requested_base()? {
        return Ok(false);
    }
    if let Some(portrait) = w.portrait()?
        && !w.data_ready(portrait)?
    {
        return Ok(false);
    }
    if !w.banner_ready()? {
        return Ok(false);
    }
    let mut index = 0usize;
    while (index as isize) < w.aspect_count()? {
        let aspect = w.aspect(index)?;
        if !w.aspect_ready(aspect)? {
            return Ok(false);
        }
        index += 1;
    }
    for offset in [0xec, 0xf0, 0x138] {
        if !w.texture_ready(offset)? {
            return Ok(false);
        }
    }
    for index in 0..6 {
        if !w.view_fx_ready(index)? {
            return Ok(false);
        }
    }
    Ok(true)
}
pub(super) trait ActorRelease: ObjectRelease {
    type AnimationSet: Clone;
    type Animation;
    type Ability: Clone;
    ///+174, reread at45E064 / 45F06A.
    fn mask_174(&mut self) -> Result<u32>;
    fn animation_set_count(&mut self) -> Result<isize>;
    fn animation_set(&mut self, index: usize) -> Result<Option<Self::AnimationSet>>;
    fn animation_count(&mut self, set: &Self::AnimationSet) -> Result<isize>;
    fn animation(&mut self, set: &Self::AnimationSet, index: usize) -> Result<Self::Animation>;
    ///Animation vf4 45EAC0 / non-virtual 45EB10.
    fn release_animation(&mut self, animation: Self::Animation, delta: u32) -> Result<()>;
    fn animation_ready(&mut self, animation: Self::Animation) -> Result<bool>;
    ///428900 / 428950 on the eight actual Actor FX lists.
    fn release_actor_fx(&mut self, offset: u32, delta: u32) -> Result<()>;
    fn actor_fx_ready(&mut self, offset: u32) -> Result<bool>;
    fn ability_count(&mut self, secondary: bool) -> Result<isize>;
    fn ability(&mut self, secondary: bool, index: usize) -> Result<Self::Ability>;
    ///4BE450 / 4BE480
    fn release_ability(&mut self, ability: Self::Ability, delta: u32) -> Result<()>;
    fn ability_ready(&mut self, ability: Self::Ability) -> Result<bool>;
    fn extra_ability_2a4(&mut self) -> Result<Option<Self::Ability>>;
    ///ActorData+534, the MobileDeployment forward target.
    fn deployment_data_534(&mut self) -> Result<Option<Self::Data>>;
}
const ACTOR_FX: [u32; 8] = [0x194, 0x19c, 0x24c, 0x254, 0x294, 0x29c, 0x598, 0x5a0];
///45DF40. Unlike45DD80 there is no+174 bit2 model-flag pass.
pub(super) fn actor_release(w: &mut impl ActorRelease, delta: u32) -> Result<()> {
    object_release(w, delta)?;
    let mut set_index = 0usize;
    while (set_index as isize) < w.animation_set_count()? {
        if let Some(set) = w.animation_set(set_index)? {
            let mut index = 0usize;
            while (index as isize) < w.animation_count(&set)? {
                let animation = w.animation(&set, index)?;
                w.release_animation(animation, delta)?;
                index += 1;
            }
        }
        set_index += 1;
    }
    for offset in ACTOR_FX {
        w.release_actor_fx(offset, delta)?;
    }
    //45E00E/45E034 capture each signed count once.
    for secondary in [false, true] {
        let count = w.ability_count(secondary)?;
        for index in 0..count.max(0) as usize {
            let ability = w.ability(secondary, index)?;
            w.release_ability(ability, delta)?;
        }
    }
    if let Some(ability) = w.extra_ability_2a4()? {
        w.release_ability(ability, delta)?;
    }
    if w.mask_174()? & (1 << 19) != 0
        && let Some(data) = w.deployment_data_534()?
    {
        w.release_data(data, delta)?;
    }
    Ok(())
}
///45EF20
pub(super) fn actor_ready(w: &mut impl ActorRelease) -> Result<bool> {
    if !object_ready(w)? {
        return Ok(false);
    }
    let mut set_index = 0usize;
    while (set_index as isize) < w.animation_set_count()? {
        if let Some(set) = w.animation_set(set_index)? {
            let mut index = 0usize;
            while (index as isize) < w.animation_count(&set)? {
                let animation = w.animation(&set, index)?;
                if !w.animation_ready(animation)? {
                    return Ok(false);
                }
                index += 1;
            }
        }
        set_index += 1;
    }
    for offset in ACTOR_FX {
        if !w.actor_fx_ready(offset)? {
            return Ok(false);
        }
    }
    for secondary in [false, true] {
        let count = w.ability_count(secondary)?;
        for index in 0..count.max(0) as usize {
            let ability = w.ability(secondary, index)?;
            if !w.ability_ready(ability)? {
                return Ok(false);
            }
        }
    }
    if let Some(ability) = w.extra_ability_2a4()?
        && !w.ability_ready(ability)?
    {
        return Ok(false);
    }
    if w.mask_174()? & (1 << 19) != 0
        && let Some(data) = w.deployment_data_534()?
        && !w.data_ready(data)?
    {
        return Ok(false);
    }
    Ok(true)
}

#[cfg(test)]
#[path = "data_release_fields_tests.rs"]
mod tests;
