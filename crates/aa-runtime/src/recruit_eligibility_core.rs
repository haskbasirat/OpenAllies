//!581F70 as reached by5827B0 with check=1 and no diagnostic destination.
//!The temporary missing lists belong to its requirements child; errors never
//!roll back effects of a preceding native owner/item callback.
use aa_actors::{
    ActorId,
    ability_flags::{Error, Result},
};

pub(crate) struct RequirementResult {
    pub met: bool,
    pub properties_met: bool,
}
pub(crate) trait World {
    type Definition: Clone + Ord;
    type Layout: Clone;
    fn organization(&mut self, definition: &Self::Definition) -> Result<bool>;
    fn default_layout(&mut self, definition: &Self::Definition) -> Result<Option<Self::Layout>>;
    fn owner(&mut self) -> Result<Option<ActorId>>;
    fn owner_allows(
        &mut self,
        owner: ActorId,
        definition: &Self::Definition,
        layout: Option<&Self::Layout>,
    ) -> Result<bool>;
    fn own_recruit_list_present(&mut self) -> Result<bool>;
    fn own_recruit_list_contains(&mut self, definition: &Self::Definition) -> Result<bool>;
    fn layout_count(&mut self, layout: &Self::Layout) -> Result<i32>;
    fn layout_member(
        &mut self,
        layout: &Self::Layout,
        index: i32,
    ) -> Result<Option<Self::Definition>>;
    fn requirements(&mut self, definition: &Self::Definition) -> Result<RequirementResult>;
    fn strict_requirements(&mut self, definition: &Self::Definition) -> Result<bool>;
}

pub(crate) fn eligibility<W: World>(
    world: &mut W,
    definition: Option<&W::Definition>,
    layout: Option<&W::Layout>,
    check: bool,
) -> Result<u32> {
    evaluate(
        world,
        definition,
        layout,
        check,
        &mut std::collections::BTreeSet::new(),
    )
}
fn evaluate<W: World>(
    world: &mut W,
    definition: Option<&W::Definition>,
    layout: Option<&W::Layout>,
    check: bool,
    active: &mut std::collections::BTreeSet<W::Definition>,
) -> Result<u32> {
    let Some(definition) = definition else {
        return Ok(0);
    };
    if !active.insert(definition.clone()) {
        return Err(Error::Missing(
            "581F70 recursive default Layout cycle is unsupported",
        ));
    }
    let result = (|| {
        let fallback;
        let layout = if let Some(layout) = layout {
            Some(layout)
        } else if world.organization(definition)? {
            fallback = world.default_layout(definition)?;
            fallback.as_ref()
        } else {
            None
        };
        if world.owner()?.is_some() {
            // Native rereads +108 after the first nonnull test.
            let owner = world
                .owner()?
                .ok_or(Error::Missing("581F70 reached null owner reread"))?;
            if !world.owner_allows(owner, definition, layout)? {
                return Ok(2);
            }
        }
        //529C30/+4EC is read before native tests the check byte.
        if world.own_recruit_list_present()?
            && !check
            && !world.own_recruit_list_contains(definition)?
        {
            return Ok(2);
        }
        if let Some(layout) = layout {
            let mut result = 0;
            let mut index = 0;
            while index < world.layout_count(layout)? {
                let member = world.layout_member(layout, index)?;
                let child = evaluate(world, member.as_ref(), None, true, active)?;
                if child != 0 {
                    result = child;
                }
                index += 1;
            }
            return Ok(result);
        }
        let requirements = world.requirements(definition)?;
        if requirements.met {
            Ok(0)
        } else if !requirements.properties_met && world.strict_requirements(definition)? {
            Ok(2)
        } else {
            Ok(1)
        }
    })();
    active.remove(definition);
    result
}

#[cfg(test)]
#[path = "recruit_eligibility_core_tests.rs"]
mod tests;
