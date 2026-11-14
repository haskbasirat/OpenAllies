//! Native actor landing / placement refresh `4D64A0`.
//!
//! With actor `+8` bit 0 set: the Body's `562380` when the actor has a Body
//! (`+60`), else the world's `4F6760(actor, 0)`. Otherwise the terrain
//! refresh `4D4130`, then the Character's `563D50` when `+BC` is clear and a
//! Character (`+68`) exists, then `4D1350`.
//!
//! Oracle: research/phase8-combat/native_landing.py.

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("landing callback failed: {0}")]
    World(String),
}
pub type Result<T> = std::result::Result<T, Error>;

pub trait World {
    /// Actor `+8` word.
    fn actor_flags(&mut self, actor: u32) -> Result<u32>;
    /// Actor `+60` Body.
    fn body(&mut self, actor: u32) -> Result<Option<u32>>;
    /// `562380` on the Body.
    fn body_562380(&mut self, body: u32) -> Result<()>;
    /// `4F6760(actor, 0)` on the world `[B36534]`.
    fn world_4f6760(&mut self, actor: u32) -> Result<()>;
    /// `4D4130` (terrain refresh) on the actor.
    fn refresh_terrain(&mut self, actor: u32) -> Result<()>;
    /// Actor `+BC` word.
    fn word_bc(&mut self, actor: u32) -> Result<u32>;
    /// Actor `+68` Character.
    fn character(&mut self, actor: u32) -> Result<Option<u32>>;
    /// `563D50` on the Character.
    fn character_563d50(&mut self, character: u32) -> Result<()>;
    /// `4D1350` on the actor.
    fn notify_4d1350(&mut self, actor: u32) -> Result<()>;
}

/// Native `4D64A0` on `actor`.
pub fn land(world: &mut impl World, actor: u32) -> Result<()> {
    if world.actor_flags(actor)? & 1 != 0 {
        return match world.body(actor)? {
            Some(body) => world.body_562380(body),
            None => world.world_4f6760(actor),
        };
    }
    world.refresh_terrain(actor)?;
    if world.word_bc(actor)? == 0
        && let Some(character) = world.character(actor)?
    {
        world.character_563d50(character)?;
    }
    world.notify_4d1350(actor)
}
