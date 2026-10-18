//! The one GWorld `+4C` Sun: constructor `4EE0A0`, saved Load `4EDBD0`.
//! Raw binary32 words are retained; Load performs no floating arithmetic.
//! Scheduling and saved-ID registration belong to the caller's real world.
use aa_actors::organization_state::{Arg, Cursor, LoadError, Message};

pub type Result<T> = std::result::Result<T, LoadError>;

/// Services of the actual Sun's Updatable base (`50BCD0`).
pub trait World {
    /// `50B250`: cancel this Sun's actual scheduled entries.
    fn cancel_scheduled(&mut self, head: &mut Option<u64>) -> Result<()>;
    /// `[GWorld+284]+0`, before the native signed `id > word0` check.
    fn table_word0(&mut self) -> Result<i32>;
    /// Read the actual allocated slot. An id outside its allocation must fail.
    fn slot_occupied(&mut self, id: i32) -> Result<bool>;
    /// Store this same Sun in the actual saved Updatable table.
    fn register_updatable(&mut self, id: i32) -> Result<()>;
}

#[derive(Debug, PartialEq, Eq)]
pub struct State {
    /// Updatable `+4`, null after constructor `50AB90`. A live queue owner
    /// must keep this cancellation-chain identity current.
    pub scheduled_head: Option<u64>,
    pub direction: [u32; 3],
    /// Native `+14`, `+20`, `+2C`.
    pub target_colors: [[u32; 3]; 3],
    /// Native `+38`, `+44`, `+50`.
    pub current_colors: [[u32; 3]; 3],
    pub color_duration: u32,
    pub current_dimmer: u32,
    pub target_dimmer: u32,
    pub dimmer_duration: u32,
    pub start_time: u32,
    pub byte_70: u8,
    pub word_74: u32,
    pub word_78: u32,
    pub byte_7c: u8,
    pub word_80: u32,
    pub word_84: u32,
}

impl State {
    /// `4EE0A0`: the three words read from `B2814C..B28154` are explicit.
    /// The real Updatable queue owner is constructed separately with no entries.
    pub fn new(color_b2814c: [u32; 3]) -> Self {
        Self {
            scheduled_head: None,
            direction: [0, 0, 0x3f80_0000],
            target_colors: [color_b2814c; 3],
            current_colors: [color_b2814c; 3],
            color_duration: 0,
            current_dimmer: 0,
            target_dimmer: 0,
            dimmer_duration: 0,
            start_time: 0xbf80_0000,
            byte_70: 0,
            word_74: 0,
            word_78: 0,
            byte_7c: 0,
            word_80: 0,
            word_84: 0,
        }
    }

    /// Effects before a failed native read/service remain committed.
    pub fn load(&mut self, bytes: &[u8], world: &mut impl World) -> Result<usize> {
        let mut cursor = Cursor::new(bytes);
        self.load_cursor(&mut cursor, world)?;
        Ok(cursor.position())
    }

    pub fn load_cursor(&mut self, c: &mut Cursor<'_>, world: &mut impl World) -> Result<()> {
        // Outer Sun version is read BEFORE its Updatable base's version.
        let version = c.version(2)?;
        let updatable_version = c.version(1)?;
        world.cancel_scheduled(&mut self.scheduled_head)?;
        if updatable_version >= 1 {
            let id = c.i32()?;
            if id >= 0 {
                let bound = world.table_word0()?;
                if id > bound || world.slot_occupied(id)? {
                    return Err(LoadError::Fatal(Message::new(
                        "Bad or duplicate updatable ID %d. There are %d.",
                        vec![Arg::Int(id), Arg::Int(bound)],
                    )));
                }
                world.register_updatable(id)?;
            }
        }
        // Each triple is ONE native 12-byte stream call, not three reads.
        self.direction = triple(c)?;
        for color in &mut self.current_colors {
            *color = triple(c)?;
        }
        for color in &mut self.target_colors {
            *color = triple(c)?;
        }
        self.color_duration = c.u32()?;
        self.current_dimmer = c.u32()?;
        self.target_dimmer = c.u32()?;
        self.dimmer_duration = c.u32()?;
        self.start_time = c.u32()?;
        if version >= 1 {
            self.byte_70 = c.u8()?;
            self.word_74 = c.u32()?;
            self.word_78 = c.u32()?;
            self.byte_7c = c.u8()?;
            self.word_80 = c.u32()?;
            self.word_84 = c.u32()?;
        }
        if version <= 1 {
            self.word_74 = 0;
            self.word_78 = 0;
            self.word_80 = 0;
            self.word_84 = 0;
        }
        Ok(())
    }
}

fn triple(c: &mut Cursor<'_>) -> Result<[u32; 3]> {
    let bytes = c.take(12)?;
    Ok(std::array::from_fn(|i| {
        u32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap())
    }))
}
