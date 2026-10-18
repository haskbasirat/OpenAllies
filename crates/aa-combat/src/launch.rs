//! Native projectile factory handoff4C8CB0, including projectile branch4BE780.
//! Actor creation and hit-surface geometry are required world operations.
use crate::{Result, ability::Ability, performer::Context};
use aa_body::F32;
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Local {
    pub spawn_position: [F32; 3],
    pub spawn_bone: u32,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Source {
    pub position: [F32; 3],
    pub facing: [F32; 2],
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpawnRequest {
    pub definition: u32,
    pub position: [F32; 3],
    pub direction: [F32; 2],
    pub name_utf16: Vec<u16>,
    /// Native creation record+20. Original all-ones auto identity sentinel.
    pub saved_id: u32,
    pub word24: u32,
    pub allegiance: Option<u32>,
    pub word2c: u32,
    pub word30: u32,
    pub variant: i32,
    pub reason: u32,
    pub words3c_4c: [u32; 5],
    pub flags50_52: [bool; 3],
}
pub trait World {
    fn projectile_definition(&self, ability_definition: u32) -> Result<u32>;
    fn local_definition(&self, local: u32) -> Result<Local>;
    fn is_kingdom(&mut self, source: u32) -> Result<bool>;
    fn source(&self, actor: u32) -> Result<Source>;
    fn missing_local_definition(&mut self, ability_definition: u32) -> Result<()>;
    /// Original4F9C10. Must run the actual actor factory, not create a timer stub.
    fn create_actor(&mut self, request: &SpawnRequest) -> Result<Option<u32>>;
    fn target_hit_position(&mut self, target: u32, direction: [F32; 3]) -> Result<[F32; 3]>;
    /// Execute projectile::start on the created actor's owned component. Its
    /// activation return is ignored by this caller, matching native4C8CB0.
    fn start_projectile(
        &mut self,
        actor: u32,
        context: &Context,
        ability: &Ability,
        bone: u32,
        p: Precision,
    ) -> Result<()>;
}
fn n(f: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(f.0)?)
}
/// Projectile-specific4BE780. Nonprojectile bone/model spawning is a different
/// branch and is deliberately not claimed by this function.
pub fn spawn_position(
    local: Local,
    source: Option<u32>,
    fallback: [F32; 3],
    world: &mut impl World,
    p: Precision,
) -> Result<[F32; 3]> {
    let Some(actor) = source else {
        return Ok([
            fallback[0],
            fallback[1],
            F32(n(fallback[2])?
                .add(&n(local.spawn_position[2])?, p)?
                .store_f32()?),
        ]);
    };
    if world.is_kingdom(actor)? {
        return Ok(fallback);
    }
    let s = world.source(actor)?;
    let v = local.spawn_position;
    let x = n(v[0])?
        .mul(&n(s.facing[1])?, p)?
        .add(&n(v[1])?.mul(&n(s.facing[0])?, p)?, p)?
        .add(&n(s.position[0])?, p)?;
    let y = n(v[1])?
        .mul(&n(s.facing[1])?, p)?
        .sub(&n(v[0])?.mul(&n(s.facing[0])?, p)?, p)?
        .add(&n(s.position[1])?, p)?;
    let z = n(v[2])?.add(&n(s.position[2])?, p)?;
    Ok([
        F32(x.store_f32()?),
        F32(y.store_f32()?),
        F32(z.store_f32()?),
    ])
}
pub fn launch(
    ability: &Ability,
    context: &mut Context,
    world: &mut impl World,
    p: Precision,
) -> Result<Option<u32>> {
    let definition = world.projectile_definition(ability.definition)?;
    let position = if let Some(local) = ability.local_definition {
        spawn_position(
            world.local_definition(local)?,
            context.source,
            context.source_position,
            world,
            p,
        )?
    } else {
        world.missing_local_definition(ability.definition)?;
        context.source_position
    };
    let request = SpawnRequest {
        definition,
        position,
        direction: [F32::ZERO, F32(0xbf800000)],
        name_utf16: vec![],
        saved_id: u32::MAX,
        word24: 0,
        allegiance: context.allegiance,
        word2c: 0,
        word30: 0,
        variant: -1,
        reason: 0,
        words3c_4c: [0; 5],
        flags50_52: [false; 3],
    };
    let Some(actor) = world.create_actor(&request)? else {
        return Ok(None);
    };
    if let Some(target) = context.target {
        let target_position = world.source(target)?.position;
        let direction = crate::projectile::normalized_xy(position, target_position, p)?;
        context.target_position = world.target_hit_position(target, direction)?;
    } else if let Some(local) = ability.local_definition {
        context.target_position[2] = F32(n(world.local_definition(local)?.spawn_position[2])?
            .add(&n(context.target_position[2])?, p)?
            .store_f32()?);
    }
    let bone = match ability.local_definition {
        Some(local) => world.local_definition(local)?.spawn_bone,
        None => 0,
    };
    world.start_projectile(actor, context, ability, bone, p)?;
    Ok(Some(actor))
}
