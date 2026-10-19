//!Borrowed native Object49B9F0/Actor45DD80 request dispatch. Mandatory
//!children/readers preserve actual owners and partial writes; no source or
//!Resource readiness is manufactured by this coordinator.
type Result<T> = std::result::Result<T, String>;
pub(super) trait ObjectWorld {
    type Data: Clone;
    type Aspect: Clone;
    type Model: Clone;
    fn base_request(&mut self, delta: u32, mode: u32) -> Result<()>;
    fn portrait(&mut self) -> Result<Option<Self::Data>>;
    fn request_data(&mut self, data: Self::Data, delta: u32, mode: u32) -> Result<()>;
    fn aspect_count(&mut self) -> Result<isize>;
    fn aspect(&mut self, index: usize) -> Result<Self::Aspect>;
    fn request_aspect(&mut self, aspect: Self::Aspect, delta: u32, mode: u32) -> Result<()>;
    fn flags_1c(&mut self) -> Result<u32>;
    fn unique_fc(&mut self) -> Result<bool>;
    fn current_aspect_model(&mut self, index: usize) -> Result<Self::Model>;
    fn or_model_flags(&mut self, model: Self::Model, flags: u32) -> Result<()>;
    fn request_texture(&mut self, offset: u32, delta: u32, mode: u32) -> Result<()>;
    fn request_banner(&mut self, delta: u32, mode: u32) -> Result<()>;
    fn request_view_fx(&mut self, index: usize, delta: u32, mode: u32) -> Result<()>;
}
pub(super) fn object(w: &mut impl ObjectWorld, delta: u32, mode: u32) -> Result<()> {
    w.base_request(delta, mode)?;
    if let Some(portrait) = w.portrait()? {
        w.request_data(portrait, delta, mode)?;
    }
    let mut index = 0usize;
    while (index as isize) < w.aspect_count()? {
        let aspect = w.aspect(index)?;
        w.request_aspect(aspect, delta, mode)?;
        if w.flags_1c()? & (1 << 14) != 0 {
            let model = w.current_aspect_model(index)?;
            w.or_model_flags(model, 0x800)?;
        }
        if w.unique_fc()? {
            let model = w.current_aspect_model(index)?;
            w.or_model_flags(model, 0x100)?;
        }
        index += 1;
    }
    for offset in [0xec, 0xf0] {
        w.request_texture(offset, delta, mode)?;
    }
    w.request_banner(delta, mode)?;
    w.request_texture(0x138, delta, mode)?;
    for index in 0..6 {
        w.request_view_fx(index, delta, mode)?;
    }
    Ok(())
}
pub(super) trait ActorWorld: ObjectWorld {
    type AnimationSet: Clone;
    type Animation;
    type Ability: Clone;
    fn mask_174(&mut self) -> Result<u32>;
    fn mark_all_aspect_models(&mut self, flags: u32) -> Result<()>;
    fn animation_set_count(&mut self) -> Result<isize>;
    fn animation_set(&mut self, index: usize) -> Result<Option<Self::AnimationSet>>;
    fn animation_count(&mut self, set: &Self::AnimationSet) -> Result<isize>;
    fn animation(&mut self, set: &Self::AnimationSet, index: usize) -> Result<Self::Animation>;
    fn request_animation(&mut self, animation: Self::Animation, delta: u32) -> Result<()>;
    fn request_actor_fx(&mut self, offset: u32, delta: u32, mode: u32) -> Result<()>;
    fn ability_count(&mut self, secondary: bool) -> Result<isize>;
    fn ability(&mut self, secondary: bool, index: usize) -> Result<Self::Ability>;
    fn request_ability(&mut self, ability: Self::Ability, delta: u32, mode: u32) -> Result<()>;
    fn extra_ability_2a4(&mut self) -> Result<Option<Self::Ability>>;
    ///ActorData+534, the MobileDeployment forward unpack target written by
    ///the deferred4605B0 pass (not a Settlement payload). Null otherwise.
    fn deployment_data_534(&mut self) -> Result<Option<Self::Data>>;
}
pub(super) fn actor(w: &mut impl ActorWorld, delta: u32, mode: u32) -> Result<()> {
    object(w, delta, mode)?;
    if w.mask_174()? & 4 != 0 {
        w.mark_all_aspect_models(0x1000)?;
    }
    let mut index = 0usize;
    while (index as isize) < w.animation_set_count()? {
        if let Some(set) = w.animation_set(index)? {
            let mut item = 0usize;
            while (item as isize) < w.animation_count(&set)? {
                let animation = w.animation(&set, item)?;
                w.request_animation(animation, delta)?;
                item += 1;
            }
        }
        index += 1;
    }
    for offset in [0x194, 0x19c, 0x24c, 0x254, 0x294, 0x29c, 0x598, 0x5a0] {
        w.request_actor_fx(offset, delta, mode)?;
    }
    for secondary in [false, true] {
        //45DE9C/45DEC9 capture these signed counts once, unlike the current
        //animation counts above. The actual array entry is still reread.
        let count = w.ability_count(secondary)?;
        for index in 0..count.max(0) as usize {
            let ability = w.ability(secondary, index)?;
            w.request_ability(ability, delta, mode)?;
        }
    }
    if let Some(ability) = w.extra_ability_2a4()? {
        w.request_ability(ability, delta, mode)?;
    }
    //45DF0B..45DF29: bit19 is reread here; (+534)->vf14(delta, 0) is the
    //target's own Data request, so its children see mode 0.
    if w.mask_174()? & (1 << 19) != 0 {
        if let Some(data) = w.deployment_data_534()? {
            w.request_data(data, delta, 0)?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "data_request_fields_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "data_request_deployment_tests.rs"]
mod deployment_tests;
