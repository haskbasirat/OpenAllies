//!Borrowed469FE0/46AEC0. Child calls can mutate the same fields; all reads
//!after a child are fresh. This contains no source decoder or resource loader.
pub(crate) type Result<T> = std::result::Result<T, String>;

pub(crate) trait World {
    type Model: Clone;
    type Biome: Clone;
    fn context_sensitive(&mut self) -> Result<bool>;
    fn current_biome(&mut self) -> Result<Self::Biome>;
    fn stored_biome(&mut self) -> Result<Option<Self::Biome>>;
    fn same_biome(&self, a: &Self::Biome, b: &Self::Biome) -> bool;
    fn store_biome(&mut self, value: Self::Biome) -> Result<()>;
    fn model(&mut self) -> Result<Option<Self::Model>>;
    fn same_model(&self, a: &Self::Model, b: &Self::Model) -> bool;
    fn make_model(&mut self) -> Result<Self::Model>;
    fn store_model(&mut self, value: Self::Model) -> Result<()>;
    fn model_flags(&mut self, value: &Self::Model) -> Result<u32>;
    fn mark_model(&mut self, value: &Self::Model) -> Result<()>;
    fn extra_requested(&mut self) -> Result<bool>;
    fn assertion_suppressed(&mut self) -> Result<bool>;
    fn assert_context_change(&mut self) -> Result<()>;
    fn clear_extra_count(&mut self) -> Result<()>;
    fn store_extra_requested(&mut self, value: bool) -> Result<()>;
    fn request_skin(&mut self, index: usize, delta: u32, mode: u32) -> Result<()>;
    fn request_animations(&mut self, delta: u32) -> Result<()>;
    fn request_model(&mut self, model: &Self::Model, delta: u32, mode: u32) -> Result<()>;
    fn request_extra(&mut self, delta: u32, mode: u32) -> Result<()>;
    fn count(&mut self) -> Result<u32>;
    fn store_count(&mut self, value: u32) -> Result<()>;
}

///469FE0: the sensitive branch reads current GMap+24 before the model guard;
///the other branch never reads GMap. Store28 precedes Model flags callbacks.
pub(crate) fn resolve(world: &mut impl World) -> Result<()> {
    if !world.context_sensitive()? {
        if world.model()?.is_some() {
            return Ok(());
        }
        let model = world.make_model()?;
        world.store_model(model.clone())?;
        return world.mark_model(&model);
    }
    let biome = world.current_biome()?;
    if world.model()?.is_some()
        && world
            .stored_biome()?
            .is_some_and(|old| world.same_biome(&biome, &old))
    {
        return Ok(());
    }
    let model = world.make_model()?;
    if !world.assertion_suppressed()? && world.extra_requested()? {
        world.assert_context_change()?;
    }
    if !world
        .model()?
        .is_some_and(|old| world.same_model(&model, &old))
    {
        world.store_model(model.clone())?;
        world.mark_model(&model)?;
    }
    world.clear_extra_count()?;
    world.store_extra_requested(false)?;
    world.store_biome(biome)
}

///46AEC0: current model and count are reread following resource/extra calls.
///The wrapping count store happens LAST, only after all reached children.
pub(crate) fn request(world: &mut impl World, delta: u32, mode: u32) -> Result<()> {
    resolve(world)?;
    world.request_skin(0, delta, mode)?;
    world.request_skin(1, delta, mode)?;
    world.request_animations(delta)?;
    let model = world.model()?.ok_or("46AEC0 current Model28 is null")?;
    world.request_model(&model, delta, mode)?;
    let model = world.model()?.ok_or("46AEC0 current Model28 became null")?;
    if world.model_flags(&model)? & 1 != 0 {
        let extra_delta = if world.extra_requested()? {
            delta
        } else {
            world.count()?.wrapping_add(delta)
        };
        world.request_extra(extra_delta, mode)?;
    } else {
        world.store_extra_requested(false)?;
    }
    let count = world.count()?.wrapping_add(delta);
    world.store_count(count)
}

#[cfg(test)]
#[path = "data_aspect_fields_tests.rs"]
mod tests;
