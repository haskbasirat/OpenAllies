use aa_actors::{ActorId,actor_creation::{Error,Result},detection_memory::Record};
trait World {
    fn time(&mut self) -> Result<u32>;
    fn name(&mut self) -> Result<Vec<u16>>;
    fn position(&mut self) -> Result<[u32; 2]>;
    fn definition(&mut self) -> Result<String>;
    fn owner(&mut self) -> Result<ActorId>;
    fn aspect(&mut self) -> Result<u32>;
    fn stat(&mut self, slot: u32) -> Result<u32>;
    fn flags(&mut self) -> Result<u32>;
    fn record(&mut self) -> Result<&mut Record>;
}
fn refresh_inner(w: &mut impl World) -> Result<()> {
    let time = w.time()?;
    w.record()?.timestamp_bits = time;
    let name = w.name()?;
    w.record()?.name_utf16 = name;
    let position = w.position()?;
    w.record()?.position_bits = position;
    let definition = w.definition()?;
    w.record()?.definition = Some(definition);
    let owner = w.owner()?;
    w.record()?.owner = owner;
    let aspect = w.aspect()?;
    w.record()?.aspect = aspect;
    //Native calls114 before118/11C; its destination is record+2C (index4).
    for (slot, index) in [(0x10c, 0), (0x110, 1), (0x114, 4), (0x118, 2), (0x11c, 3)] {
        let value = w.stat(slot)?;
        w.record()?.stat_words[index] = Some(value);
    }
    let flags = w.flags()?;
    w.record()?.byte_30 = ((flags >> 30) & 1) as u8;
    let flags = w.flags()?;
    w.record()?.flags = flags;
    Ok(())
}

#[path="E:/Games/Axis and Alies/rust-rebuild/crates/aa-runtime/src/load_store_memory_refresh_tests.rs"]
mod tests;
