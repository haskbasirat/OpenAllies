//! Native credit dependencies: actor value4D01E0(actor,0), experience routing
//! 4CE770 and regiment member iteration578710. Actual promotion571A60 is a
//! required synchronous callback; this module does not invent rank thresholds.
use crate::{Error, F32, Result, credit::Object};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::BTreeSet};

/// Component values at native+7C/+6C, +98/+34, +74/+60 and +88/+14.
/// None means the actual component is absent. The argument=true resupply-value
/// variant of578C00 is outside this API; kill credit requests argument=false.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValueFields {
    pub regiment: Option<F32>,
    pub wall: Option<F32>,
    pub combat: Option<F32>,
    pub component88: Option<F32>,
}

/// Exact extended result: no implicit binary32 store between component adds.
pub fn actor_value(fields: &ValueFields, precision: Precision) -> Result<Binary> {
    let mut value = match fields.regiment {
        Some(v) => v.number()?,
        None => Binary::from_u32(0),
    };
    for field in [fields.wall, fields.combat, fields.component88]
        .into_iter()
        .flatten()
    {
        value = value.add(&field.number()?, precision)?;
    }
    Ok(value)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    pub experience_component: Option<Object>,
    pub regiment: Option<Object>,
    pub organization_leader: Option<Object>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    pub amount: F32,
    pub argument1: bool,
    pub argument2: bool,
}

pub trait World {
    /// Pure live field views. Callers must not synthesize absent actors or array
    /// entries, and callbacks must not silently replace the supplied handles.
    fn actor(&self, actor: Object) -> Result<Actor>;
    fn regiment_count(&self, regiment: Object) -> Result<i32>;
    fn regiment_member(&self, regiment: Object, index: i32) -> Result<Option<Object>>;
    /// Native571A60(component,amount,1). This callback applies experience and
    /// promotions; it can mutate actors and regiment arrays synchronously.
    fn apply(&mut self, component: Object, amount: F32, argument: bool) -> Result<()>;
}

pub fn award<W: World>(actor: Object, request: Request, world: &mut W) -> Result<()> {
    // These keys identify repeated routing only while the world has not had a
    // mutating callback. An actor revisited after apply can have an entirely
    // different route even if its identity and request flags are unchanged.
    let mut active = BTreeSet::new();
    award_checked(actor, request, world, &mut active)
}

fn award_checked<W: World>(
    actor: Object,
    request: Request,
    world: &mut W,
    active: &mut BTreeSet<(Object, bool, bool)>,
) -> Result<()> {
    let key = (actor, request.argument1, request.argument2);
    if !active.insert(key) {
        return Err(Error::Callback(
            "cyclic native experience propagation graph".into(),
        ));
    }
    let result = award_inner(actor, request, world, active);
    active.remove(&key);
    result
}

fn award_inner<W: World>(
    mut actor: Object,
    mut request: Request,
    world: &mut W,
    active: &mut BTreeSet<(Object, bool, bool)>,
) -> Result<()> {
    if request.amount.compare(F32(0x38d1_b717))? == Ordering::Less {
        return Ok(());
    }
    loop {
        let fields = world.actor(actor)?;
        if fields.experience_component.is_some() {
            if let Some(leader) = fields.organization_leader {
                if request.argument2 {
                    award_checked(
                        leader,
                        Request {
                            amount: request.amount,
                            argument1: false,
                            argument2: false,
                        },
                        world,
                        active,
                    )?;
                }
                if !request.argument1 {
                    return Ok(());
                }
            }
            // Native rereads+A4 after leader callbacks. An absent component here
            // is an invalid native dereference domain, never a silent no-op.
            let component = world.actor(actor)?.experience_component.ok_or_else(|| {
                Error::Callback("experience component disappeared after leader award".into())
            })?;
            world.apply(component, request.amount, true)?;
            // Every suspended ancestor's routing assumptions are now stale.
            // Clear all keys, rather than only this actor: callbacks can change
            // any ancestor's components, regiment members or member count.
            active.clear();
            return Ok(());
        }
        if let Some(regiment) = fields.regiment {
            let mut index = 0;
            while index < world.regiment_count(regiment)? {
                if let Some(member) = world.regiment_member(regiment, index)? {
                    award_checked(
                        member,
                        Request {
                            amount: request.amount,
                            argument1: true,
                            argument2: false,
                        },
                        world,
                        active,
                    )?;
                }
                index += 1;
            }
            return Ok(());
        }
        let Some(leader) = fields.organization_leader else {
            return Ok(());
        };
        if !request.argument2 {
            return Ok(());
        }
        actor = leader;
        request.argument1 = false;
        request.argument2 = false;
    }
}
