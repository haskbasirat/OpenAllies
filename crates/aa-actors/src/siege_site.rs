//! Native actor siege site `4D11D0`: the word it returns is the actor's own
//! virtual `+100` answer when nonzero; else, when the actor's Mind (`+70`)
//! target GTID (`+28`) still resolves in the `[B364C0]` registry and that
//! thing's virtual `+100` answers nonzero, that answer; else the world's
//! `4F2220` at the actor's `+20/+24` position.
//!
//! Oracle: research/phase8-combat/native_landing.py.

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("siege site callback failed: {0}")]
    World(String),
}
pub type Result<T> = std::result::Result<T, Error>;

pub trait World {
    /// Thing virtual `+100`.
    fn virtual100(&mut self, thing: u32) -> Result<u32>;
    /// Actor `+70` Mind.
    fn mind(&mut self, actor: u32) -> Result<Option<u32>>;
    /// Mind `+28` GTID.
    fn mind_target_gtid(&mut self, mind: u32) -> Result<u32>;
    /// Registry `[B364C0]+4+index*2` generation word.
    fn registry_generation(&mut self, index: u32) -> Result<u16>;
    /// Registry `[B364C0]+20004+index*4` object.
    fn registry_object(&mut self, index: u32) -> Result<Option<u32>>;
    /// Actor `+20/+24` (binary32 bits).
    fn position(&mut self, actor: u32) -> Result<[u32; 2]>;
    /// `4F2220(x, y)` on the world `[B36534]`.
    fn world_4f2220(&mut self, point: [u32; 2]) -> Result<u32>;
}

/// Native `4D11D0` on `actor`.
pub fn siege_site(world: &mut impl World, actor: u32) -> Result<u32> {
    let own = world.virtual100(actor)?;
    if own != 0 {
        return Ok(own);
    }
    if let Some(mind) = world.mind(actor)? {
        let gtid = world.mind_target_gtid(mind)?;
        let index = gtid & 0xFFFF;
        if (gtid >> 16) as u16 == world.registry_generation(index)?
            && let Some(target) = world.registry_object(index)?
        {
            let value = world.virtual100(target)?;
            if value != 0 {
                return Ok(value);
            }
        }
    }
    let point = world.position(actor)?;
    world.world_4f2220(point)
}
