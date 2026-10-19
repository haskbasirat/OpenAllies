//! KKC_TellSelectedCommandOrder::Process 0x61F210 description (GameOrders.cpp).
//!
//! 61F210 changes no state of its own: its only effects are the 4DA570
//! guess (already the decree kernel's, on the same arena command) and one
//! 0x404F90 diagnostic on channel B38B70. This module computes that
//! diagnostic's exact arguments in native read order and hands them to the
//! host sink. Native reads (61F210 + 74B3B0/765E00/766380/40B000):
//!
//! * Empty selection ([Player+50]+4 == NULL), line 0x15A: order name
//!   (flag bit6 ? [cmd+8]+8 : NULL; NULL -> L"?" 9F7228), player kingdom
//!   765E00 ([B36534] ? Player+28 : NULL; +188, NULL -> 999), Player+8
//!   name, time = 59ABA0(B396E0); then 404F90 and "rejecting" (no 4ED360).
//! * Selected, line 0x16F: first actor vt+B4/vt+108 (kingdom), the 4DA570
//!   guess for a default command, order name (bit6: [cmd+8]; default: the
//!   guess; NULL -> "?"), actor kingdom +188 (999), player kingdom, name,
//!   view = [B39B1C] (binary32), actor+14, [actor+4]+8 IDS, time =
//!   40B000(B39690) = 59ABA0(B39690+50 == B396E0); then 404F90, then 4ED360.
//!
//! Time: `fild` (signed32), `fadd` binary32 2^32 when negative, `fmul`
//! binary32 0.001 (9F4580) at the thread's x87 precision, `fstp qword`.
//! Proven by research/phase8-orders-input/recruit/native-describe-61f210.json.
use crate::{
    actor_world::ActorAccess,
    catalog::Catalog,
    mind_world::Commands,
    player_order_world::{self as orders, Banks, Error, Result},
    player_selection::View,
};
use aa_actors::ActorId;
use aa_numeric::{Binary, Precision};

pub const CHANNEL_B38B70: u32 = 0xB38B70;
pub const FILE: &str = ".\\Source\\K2ui\\Game\\GameOrders.cpp";
pub const FUNCTION: &str = "KKC_TellSelectedCommandOrder::Process";
pub const EMPTY_LINE: u32 = 0x15A;
pub const SELECTED_LINE: u32 = 0x16F;
/// A5B900.
pub const EMPTY_FORMAT: &str =
    "[time %.1f] KKC_TellSelectedCommandOrder::Process - Player(%s K%d) nothing selected Order(%s) - rejecting";
/// A5B7C8.
pub const SELECTED_FORMAT: &str = "[time %.1f] [view %.1f] KKC_TellSelectedCommandOrder::Process - Player(%s K%d) FirstSelected(%d %s K%d) Order(%s)";
/// 9F7228 L"?": the name of a NULL order definition.
pub const NULL_ORDER: &str = "?";
/// 0x3E7: the number of a NULL kingdom.
pub const NULL_KINGDOM: i32 = 999;

/// The exact 404F90 varargs (doubles as raw binary64 bits, `%d` as i32,
/// `%s` as UTF-16 code units). Rendering the text is the sink's business.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Description {
    Empty { time_f64: u64, player: Vec<u16>, player_kingdom: i32, order: Vec<u16> },
    Selected {
        time_f64: u64,
        view_f64: u64,
        player: Vec<u16>,
        player_kingdom: i32,
        actor: i32,
        actor_ids: Vec<u16>,
        actor_kingdom: i32,
        order: Vec<u16>,
    },
}
impl Description {
    pub fn line(&self) -> u32 {
        match self {
            Self::Empty { .. } => EMPTY_LINE,
            Self::Selected { .. } => SELECTED_LINE,
        }
    }
    pub fn format(&self) -> &'static str {
        match self {
            Self::Empty { .. } => EMPTY_FORMAT,
            Self::Selected { .. } => SELECTED_FORMAT,
        }
    }
}

/// 61F210's reads outside the World, all on the ordering Admin+284 player.
/// Each is a required host service; none has a default.
pub trait Sources {
    /// 74B3B0 -> Admin+284 -> 766380(&s, 0): Player+8.
    fn ordering_player_name_8(&mut self) -> std::result::Result<Vec<u16>, String>;
    /// 765E00: `[B36534]` GWorld present ? Player+28 : NULL.
    fn ordering_player_kingdom_765e00(&mut self) -> std::result::Result<Option<ActorId>, String>;
    /// 59ABA0 on the application timer B396E0 (= B39690+50).
    fn application_milliseconds_59aba0(&mut self) -> std::result::Result<u32, String>;
    /// `[B39B1C]` binary32 bits (main-thread view time; 0 outside a view update).
    fn view_time_b39b1c(&mut self) -> std::result::Result<u32, String>;
    /// 404F90(B38B70, [`FILE`], line, [`FUNCTION`], format, ...).
    fn diagnostic_404f90(&mut self, description: Description) -> std::result::Result<(), String>;
}

fn source<T>(r: std::result::Result<T, String>) -> Result<T> {
    r.map_err(Error::Required)
}
fn units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

/// 59ABA0 milliseconds -> the double 61F210/40B000 pass to 404F90.
pub fn seconds_f64_bits(milliseconds: u32, pc: Precision) -> std::result::Result<u64, String> {
    let e = |e: aa_numeric::Error| e.to_string();
    let mut value = Binary::from_i32(milliseconds as i32);
    if (milliseconds as i32) < 0 {
        value = value.add(&Binary::from_f32_bits(0x4f80_0000).map_err(e)?, pc).map_err(e)?;
    }
    let value = value.mul(&Binary::from_f32_bits(0x3a83_126f).map_err(e)?, pc).map_err(e)?;
    aa_numeric::crt_pow::store_binary64(&value).map_err(|e| e.to_string())
}

/// `fld dword [B39B1C]; fstp qword` (exact widening of a quiet value).
pub fn view_f64_bits(bits: u32) -> u64 {
    f64::from(f32::from_bits(bits)).to_bits()
}

/// 61F210's order-name rule: an explicit (flag bit6) command names its own
/// `[cmd+8]`, a default command the 4DA570 result; NULL is "?".
pub fn order_choice<'n>(flags: u32, explicit: Option<&'n str>, guessed: Option<&'n str>) -> &'n str {
    let chosen = if flags & (1 << 6) != 0 { explicit } else { guessed };
    chosen.unwrap_or(NULL_ORDER)
}

/// The pure 61F210 argument assembly over already-read native values.
/// `actor` is (actor+14, [actor+4]+8 IDS, actor kingdom +188) for a nonempty
/// selection; kingdoms are their +188 numbers (None = NULL kingdom = 999).
#[allow(clippy::too_many_arguments)]
pub fn assemble(
    order: &str,
    player_kingdom: Option<i32>,
    player: Vec<u16>,
    milliseconds: u32,
    view_bits: u32,
    actor: Option<(i32, Vec<u16>, Option<i32>)>,
    pc: Precision,
) -> std::result::Result<Description, String> {
    let order = units(order);
    let player_kingdom = player_kingdom.unwrap_or(NULL_KINGDOM);
    let time_f64 = seconds_f64_bits(milliseconds, pc)?;
    Ok(match actor {
        None => Description::Empty { time_f64, player, player_kingdom, order },
        Some((actor, actor_ids, actor_kingdom)) => Description::Selected {
            time_f64,
            view_f64: view_f64_bits(view_bits),
            player,
            player_kingdom,
            actor,
            actor_ids,
            actor_kingdom: actor_kingdom.unwrap_or(NULL_KINGDOM),
            order,
        },
    })
}

fn kingdom_number(catalog: &Catalog<'_>, banks: &dyn Banks, actors: &ActorAccess<'_>, kingdom: Option<ActorId>) -> Result<Option<i32>> {
    kingdom
        .map(|k| orders::guess::kingdom_slot_188(catalog, banks, actors, k).map(|v| v as i32))
        .transpose()
}

fn explicit_name(catalog: &Catalog<'_>, banks: &dyn Banks, commands: &Commands, command: u32) -> Result<(u32, Option<&'static str>)> {
    let value = commands.value(command).map_err(|e| Error::Required(e.to_string()))?;
    let name = if value.flags & (1 << 6) != 0 && value.definition.is_some() {
        Some(orders::definition_with_bank(catalog, banks, value)?.id)
    } else {
        None
    };
    Ok((value.flags, name))
}

fn guessed_name(guessed: Option<u32>) -> Result<Option<&'static str>> {
    guessed
        .map(|i| {
            aa_rules::native::actor_command::PREDEFINED_ACTOR_COMMANDS
                .get(i as usize)
                .map(|d| d.id)
                .ok_or_else(|| Error::Required("61F210 guessed ActorCommand ordinal".into()))
        })
        .transpose()
}

/// 61F210's diagnostic for the decree kernel's two inspection callbacks, with
/// every read in native order. `guessed` is the 4DA570 ordinal the kernel
/// already computed on this same command (None: not guessed or NULL).
#[allow(clippy::too_many_arguments)]
pub(crate) fn describe_61f210<S: Sources + ?Sized>(
    catalog: &Catalog<'_>,
    banks: &dyn Banks,
    actors: &ActorAccess<'_>,
    commands: &Commands,
    view: &impl View,
    command: u32,
    empty: bool,
    guessed: Option<u32>,
    sources: &mut S,
    pc: Precision,
) -> Result<()> {
    if empty {
        let (flags, explicit) = explicit_name(catalog, banks, commands, command)?;
        let order = order_choice(flags, explicit, None);
        let kingdom = source(sources.ordering_player_kingdom_765e00())?;
        let player_kingdom = kingdom_number(catalog, banks, actors, kingdom)?;
        let player = source(sources.ordering_player_name_8())?;
        let milliseconds = source(sources.application_milliseconds_59aba0())?;
        let d = source(assemble(order, player_kingdom, player, milliseconds, 0, None, pc))?;
        return source(sources.diagnostic_404f90(d));
    }
    let head = view.list().head().ok_or_else(|| Error::Required("61F210 nonempty selection head".into()))?;
    let actor = view.list().node(head).map_err(|e| Error::Required(e.to_string()))?.actor;
    orders::object_with_bank(catalog, banks, actors, actor)?;
    // vt+108 = 4D1E80: [Actor+E8].
    let k = orders::actor_word_with_bank(catalog, banks, actors, actor, 0xe8)?;
    let (flags, explicit) = explicit_name(catalog, banks, commands, command)?;
    let order = order_choice(flags, explicit, guessed_name(guessed)?);
    let actor_kingdom = kingdom_number(catalog, banks, actors, (k != 0).then(|| ActorId::from_raw(k)))?;
    let kingdom = source(sources.ordering_player_kingdom_765e00())?;
    let player_kingdom = kingdom_number(catalog, banks, actors, kingdom)?;
    let player = source(sources.ordering_player_name_8())?;
    let view_bits = source(sources.view_time_b39b1c())?;
    let actor_ids = units(&orders::object_with_bank(catalog, banks, actors, actor)?.definition.proof.id);
    let milliseconds = source(sources.application_milliseconds_59aba0())?;
    let d = source(assemble(order, player_kingdom, player, milliseconds, view_bits,
        Some((actor.raw() as i32, actor_ids, actor_kingdom)), pc))?;
    source(sources.diagnostic_404f90(d))
}

#[cfg(test)]
#[path = "player_order_describe_tests.rs"]
mod tests;
