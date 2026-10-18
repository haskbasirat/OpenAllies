//!5A6D10/5A6C70,5A9330 and5A6340 over the current Texture allocation.
//!Successful allocations and stable/read-only setting inputs are the admitted
//!array domain. Loaded source children and diagnostics remain required services.
type Result<T> = std::result::Result<T, String>;

pub(crate) const SLOT_CONSTRUCTOR: [u32; 9] = [2, 0, 0, 0, 0, 0, 8, 0x22, 0];

///The Vec length represents initialized native allocation capacity, not count60.
///5A5070 initializes unused slots too, and copies the whole previous capacity.
pub(crate) fn push_5a6c70(slots: &mut Vec<[u32; 9]>, count: &mut usize, value: [u32; 9]) -> Result<()> {
    if *count > slots.len() || slots.len() > i32::MAX as usize {
        return Err("Texture render list has invalid native count/capacity".into());
    }
    let next = count.checked_add(1).filter(|v| *v <= i32::MAX as usize)
        .ok_or("Texture render list exceeds admitted signed capacity domain")?;
    if next > slots.len() || slots.is_empty() {
        let doubled = slots.len().checked_mul(2).filter(|v| *v <= i32::MAX as usize)
            .ok_or("Texture render list doubled capacity outside admitted domain")?;
        let mut capacity = doubled.max(next);
        if !slots.is_empty() { capacity = capacity.max(4); }
        slots.resize(capacity, SLOT_CONSTRUCTOR);
    }
    *count = next; //5A6D02 stores count before copying the nine source words.
    slots[next - 1] = value;
    Ok(())
}

pub(crate) trait EntryWorld {
    type Texture: Clone;
    fn flags_08(&mut self, texture: &Self::Texture) -> Result<u32>;
    fn requests_04(&mut self, texture: &Self::Texture) -> Result<u32>;
    fn store_flags_08(&mut self, texture: &Self::Texture, flags: u32) -> Result<()>;
    fn warning_404ed0(&mut self, texture: &Self::Texture, reading: bool) -> Result<()>;
    fn lock_857e10(&mut self, texture: &Self::Texture) -> Result<()>;
    fn loaded_reading_child(&mut self, texture: &Self::Texture, enabled: bool) -> Result<()>;
    fn unlock_857de0(&mut self, texture: &Self::Texture) -> Result<()>;
}
pub(crate) trait World:EntryWorld{
    fn last_location_writable(&mut self, texture: &Self::Texture) -> Result<bool>;
    fn writable_child_40(&mut self, texture: &Self::Texture) -> Result<Option<Self::Texture>>;
}

fn target<W: World>(world: &mut W, mut texture: W::Texture) -> Result<W::Texture> {
    //The native recursion/iteration rereads each current location and child.
    while world.last_location_writable(&texture)? {
        let Some(child) = world.writable_child_40(&texture)? else { break };
        texture = child;
    }
    Ok(texture)
}

pub(crate) fn set_reading_5a9330<W: World>(world: &mut W, texture: W::Texture, raw: u32) -> Result<()> {
    let texture = target(world, texture)?;
    set_reading_at(world,texture,raw)
}
pub(crate) fn set_reading_at<W:EntryWorld>(world:&mut W,texture:W::Texture,raw:u32)->Result<()>{
    let byte = raw as u8;
    let flags = world.flags_08(&texture)?;
    if byte == ((flags >> 20) & 1) as u8 { return Ok(()) }
    if byte == 0 && (world.requests_04(&texture)? as i32) > 0 {
        return world.warning_404ed0(&texture, true);
    }
    world.lock_857e10(&texture)?;
    //Native rereads flags after the lock service, before either store.
    let flags = world.flags_08(&texture)?;
    let flags = if byte == 0 { flags & !0x100000 } else { flags | 0x100000 };
    world.store_flags_08(&texture, flags)?;
    if flags & 1 != 0 { world.loaded_reading_child(&texture, byte != 0)?; }
    world.unlock_857de0(&texture)
}

pub(crate) fn set_rendering_5a6340<W: World>(world: &mut W, texture: W::Texture, raw: u32) -> Result<()> {
    let texture = target(world, texture)?;
    set_rendering_at(world,texture,raw)
}
pub(crate) fn set_rendering_at<W:EntryWorld>(world:&mut W,texture:W::Texture,raw:u32)->Result<()>{
    let byte = raw as u8;
    let flags = world.flags_08(&texture)?;
    if byte == ((flags >> 21) & 1) as u8 { return Ok(()) }
    if byte != 0 { return world.store_flags_08(&texture, flags | 0x200000) }
    if (world.requests_04(&texture)? as i32) > 0 {
        return world.warning_404ed0(&texture, false);
    }
    let flags = world.flags_08(&texture)?;
    world.store_flags_08(&texture, flags & !0x200000)
}

#[cfg(test)]
#[path = "resource_texture_settings_tests.rs"]
mod tests;
