//!46B1D0: each Sound vf10 receives the original delta/mode. Count and entry
//!are read at every iteration; duplicate handles remain duplicate calls.
pub(crate) type Result<T> = std::result::Result<T, String>;
///46BD40 registers these exact category names/ordinals;497220 folds ASCII.
pub(crate) fn category(value: &str) -> Option<usize> {
    ["SOUND", "MUSIC", "SPEECH"]
        .iter()
        .position(|v| v.eq_ignore_ascii_case(value))
}
pub(crate) trait World {
    type Sound;
    fn count(&mut self, list: usize) -> Result<usize>;
    fn sound(&mut self, list: usize, index: usize) -> Result<Self::Sound>;
    fn request(&mut self, sound: Self::Sound, delta: i32, mode: u32) -> Result<()>;
}
pub(crate) fn request(world: &mut impl World, delta: i32, mode: u32) -> Result<()> {
    for list in 0..3 {
        let mut index = 0usize;
        loop {
            if index == world.count(list)? {
                break;
            }
            //Growable host storage removes the native signed32 iteration cap.
            //The world must report a truthful error for a malformed index.
            let sound = world.sound(list, index)?;
            world.request(sound, delta, mode)?;
            index = index
                .checked_add(1)
                .ok_or("Audio list host index overflow")?;
        }
    }
    Ok(())
}
pub(crate) trait SourceWorld {
    type Sound;
    fn text(&mut self, list: usize) -> Result<String>;
    fn resource(&mut self, path: &str, streaming: bool) -> Result<Self::Sound>;
    fn standard_rate(&mut self, sound: &Self::Sound) -> Result<()>;
}
///Selected46C5D3..67C source suffix. The caller already owns the actual
///category and ignore_rate reads; every successful Find precedes its append.
pub(crate) fn load_lists<W: SourceWorld>(
    world: &mut W,
    lists: &mut [Vec<W::Sound>; 3],
    streaming: bool,
    ignore_rate: bool,
) -> Result<()> {
    for (index, list) in lists.iter_mut().enumerate() {
        let value = world.text(index)?;
        for token in tokens(&value)? {
            let sound = world.resource(token, streaming)?;
            list.try_reserve(1).map_err(|e| e.to_string())?;
            list.push(sound);
        }
    }
    if !ignore_rate {
        for list in lists {
            for sound in list {
                world.standard_rate(sound)?;
            }
        }
    }
    Ok(())
}
///424FA0/412480 literal comma+space delimiter set. Quoted/control forms
///remain explicit requirements until those native tokenizer paths are proved.
pub(crate) fn tokens(value: &str) -> Result<impl Iterator<Item = &str>> {
    if value.bytes().any(|b| b < 32 || b == b'"' || b == 127) {
        return Err("Audio token quoted/control source domain is unproved".into());
    }
    Ok(value.split([',', ' ']).filter(|v| !v.is_empty()))
}
