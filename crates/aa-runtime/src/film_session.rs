//! Actual76C310 film header/roster publication order. The map prelude is
//! 5A01E0 (59EA60 header+creator, then59FDA0 MAPSTRINGS), never full WorldSave.
//! The output/resource and roster implementations own their actual storage.
use aa_actors::ActorId;

pub type Result<T> = std::result::Result<T, String>;

/// Session+BC allocation order and current player fields. No roster snapshot.
pub trait Roster {
    type Player;
    fn count(&mut self) -> Result<usize>;
    fn player_at(&mut self, index: usize) -> Result<Self::Player>;
    fn id(&mut self, player: &Self::Player) -> Result<u32>;
    fn name_24(&mut self, player: &Self::Player) -> Result<&[u16]>;
    /// Actual765E00: null when the actual global World is absent, otherwise
    /// current player+28. A positive call is intentionally repeated natively.
    fn kingdom_28(&mut self, player: &Self::Player) -> Result<Option<ActorId>>;
    fn ai_implicit(&mut self, player: &Self::Player) -> Result<(u8, u8)>;
}

pub trait Output {
    fn save_map_header_and_strings(&mut self) -> Result<()>;
    fn compression_enabled(&mut self) -> Result<u8>;
    fn open_chunk(&mut self, name: &str, compressed: u8) -> Result<()>;
    fn write(&mut self, bytes: &[u8]) -> Result<()>;
    fn kingdom_index_188(&mut self, kingdom: ActorId) -> Result<u32>;
    fn diagnostic_player(&mut self, ai: u8, implicit: u8, id: u32) -> Result<()>;
    /// Native stream+3C ->856DC0 -> actual backing stream+44.
    fn flush(&mut self) -> Result<()>;
    /// Actual74D5F0, only after the same stream flush succeeds.
    fn attach_recorder(&mut self) -> Result<()>;
}

/// Native76C310. The per-player first DWORD is ONLY player+20 ID; the AI and
/// implicit bytes are diagnostics after serialization, never packed flags.
pub fn open<R: Roster, O: Output>(roster: &mut R, output: &mut O) -> Result<()> {
    output.save_map_header_and_strings()?;
    let compressed = output.compression_enabled()?;
    output.open_chunk("SessionPlayers", compressed)?;
    let count = u32::try_from(roster.count()?)
        .map_err(|_| "film SessionPlayers count exceeds its native DWORD wire representation")?;
    output.write(&count.to_le_bytes())?;
    for index in 0..count as usize {
        let player = roster.player_at(index)?;
        let id = roster.id(&player)?;
        output.write(&id.to_le_bytes())?;
        //76C3B2 reads only the low16 CString length bits. The subsequent
        //payload read happens after the length write, from the current string.
        let units = roster.name_24(&player)?.len() as u16;
        output.write(&units.to_le_bytes())?;
        let name = roster.name_24(&player)?;
        let prefix = name
            .get(..units as usize)
            .ok_or("film current CString allocation is shorter than its already-written length")?;
        let bytes: Vec<u8> = prefix.iter().flat_map(|unit| unit.to_le_bytes()).collect();
        output.write(&bytes)?;
        let kingdom_index = if roster.kingdom_28(&player)?.is_some() {
            let kingdom = roster
                .kingdom_28(&player)?
                .ok_or("film second765E00 returned null after the positive first read")?;
            output.kingdom_index_188(kingdom)?
        } else {
            u32::MAX
        };
        output.write(&kingdom_index.to_le_bytes())?;
        let (ai, implicit) = roster.ai_implicit(&player)?;
        //76C40E reloads the ID for the diagnostic after the last stream write.
        let diagnostic_id = roster.id(&player)?;
        output.diagnostic_player(ai, implicit, diagnostic_id)?;
    }
    let compressed = output.compression_enabled()?;
    output.open_chunk("Orders", compressed)?;
    output.flush()?;
    output.attach_recorder()
}
