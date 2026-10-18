//!562290 over synchronous references to the original Body and Actor fields.
//!The held-load and stored adapters use this one coordinator. Nonconstant
//!children are reached services, with every preceding native store retained.
use aa_actors::actor_creation::Result;

pub(crate) trait Fields {
    fn state_low(&self) -> Result<u32>;
    fn health(&self) -> Result<u32>;
    fn maximum(&self) -> Result<u32>;
    ///565070's current Construction owner/state29/target gate; no component
    ///means None. A corrupt nonnull portable token is refused by the adapter.
    fn construction_target(&self) -> Result<Option<u32>>;
    ///Read the current4CF110 Properties context, then enter561D40(target,1).
    fn update_maximum(&mut self, target: u32) -> Result<()>;
    fn store_health(&mut self, bits: u32) -> Result<()>;
    fn set_health(&mut self, requested: u32) -> Result<()>;
}

///562294 state21 precedes all health reads. The comparison is captured before
///Construction/561D40, while the fields are read AGAIN before the health store.
///The existing finite source/body scope is unchanged by this storage split.
pub(crate) fn initialize(fields: &mut impl Fields) -> Result<bool> {
    if fields.state_low()? & (1 << 21) != 0 {
        return Ok(true);
    }
    let differs = f32::from_bits(fields.health()?) != f32::from_bits(fields.maximum()?);
    if let Some(target) = fields.construction_target()? {
        fields.update_maximum(target)?;
    }
    if differs {
        let requested = fields.health()?;
        let maximum = fields.maximum()?;
        fields.store_health(maximum)?;
        fields.set_health(requested)?;
    }
    Ok(true)
}
