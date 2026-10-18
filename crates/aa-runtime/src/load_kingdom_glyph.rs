//!4E93C0 over the same current Kingdom, player roster, glyph variables/registry
//!and KingdomColor source. No display text or online-manager defaults exist.
use crate::{actor_world::ActorStore, catalog::Catalog, definitions::Bank, player_glyph};
use aa_actors::ActorId;
use aa_rules::native::TypedValue;

pub type Result<T> = std::result::Result<T, String>;

///Actual Session+BC order and the SAME765E00 World/player+28 owners.
///76CE10 compares766380(flag0)'s Player+8, not Player+24.
pub trait Players: player_glyph::PlayerRead {
    fn roster_len(&mut self) -> Result<usize>;
    fn roster_player(&mut self, index: usize) -> Result<Self::Player>;
    fn display_name_8(&mut self, player: &Self::Player) -> Result<&[u16]>;
    fn kingdom_765e00(&mut self, player: &Self::Player) -> Result<Option<ActorId>>;
}

struct Cache<V> {
    flags: u32,
    variable: Option<V>,
}
trait World {
    type Variable;
    type Player;
    fn minor(&mut self) -> Result<bool>;
    fn variable(&mut self) -> Result<Self::Variable>;
    fn find_player(&mut self) -> Result<Option<Self::Player>>;
    fn player_glyph(&mut self, player: &Self::Player) -> Result<Option<u16>>;
    fn fallback(&mut self, variable: &Self::Variable) -> Result<u16>;
    fn rgb(&mut self) -> Result<[u32; 3]>;
    fn formatted(&mut self, rgb: [u32; 3], glyph: u16) -> Result<Vec<u16>>;
}
fn run<W: World>(cache: &mut Cache<W::Variable>, world: &mut W) -> Result<Vec<u16>> {
    if world.minor()? {
        return Ok(Vec::new());
    }
    if cache.flags & 1 == 0 {
        cache.flags |= 1;
        cache.variable = Some(world.variable()?);
    }
    let glyph = match world.find_player()? {
        Some(player) => world.player_glyph(&player)?,
        None => None,
    };
    let glyph = match glyph {
        Some(value) => value,
        None => world.fallback(
            cache
                .variable
                .as_ref()
                .ok_or("4E93C0 cached String pointer is unbound")?,
        )?,
    };
    let rgb = world.rgb()?;
    world.formatted(rgb, glyph)
}

///One fresh application4E93C0 cache. Native B3CBF4 is initially PE-zero; the
///current String owner is bound lazily. An error retains that exact prefix.
pub struct Formatter {
    cache: Cache<player_glyph::VariableWitness>,
    stopped: Option<String>,
}
impl Formatter {
    pub fn from_fresh_application() -> Self {
        Self {
            cache: Cache {
                flags: 0,
                variable: None,
            },
            stopped: None,
        }
    }
    pub fn display<P: Players + ?Sized>(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        actors: &ActorStore,
        kingdom: ActorId,
        variables: &mut player_glyph::State,
        players: &mut P,
    ) -> Result<Vec<u16>> {
        if let Some(error) = &self.stopped {
            return Err(format!("4E93C0 retained unsupported prefix: {error}"));
        }
        catalog.require_source_owner()?;
        let source = bank.combat_source()?;
        source.completed_means()?;
        if source.source_identities() != catalog.actors.sources {
            return Err("4E93C0 Bank/Catalog source owner differs".into());
        }
        let shell = crate::load_sai_players::canonical_kingdom(
            catalog,
            bank,
            actors,
            kingdom,
            source.source_precision(),
        )?;
        let entity = actors
            .entity(
                actors
                    .key(kingdom)
                    .ok_or("4E93C0 current generation absent")?,
            )
            .map_err(|e| e.to_string())?;
        if !matches!(&entity.source,crate::actor_world::Source::Special(s) if s.derived_payload_loaded)
        {
            return Err("4E93C0 current Kingdom derived payload is incomplete".into());
        }
        if shell.words.get(&0x190) == Some(&0)
            && self
                .cache
                .variable
                .as_ref()
                .is_some_and(|v| !v.is_for(variables))
        {
            return Err("4E93C0 cached String belongs to another current-variable owner".into());
        }
        let result = run(
            &mut self.cache,
            &mut Live {
                catalog,
                bank,
                shell,
                kingdom,
                variables,
                players,
            },
        );
        if let Err(error) = &result {
            self.stopped = Some(error.clone());
        }
        result
    }
}
struct Live<'a, 'source, P: Players + ?Sized> {
    catalog: &'a Catalog<'source>,
    bank: &'a Bank,
    shell: &'a aa_mapworld::special_state::SpecialShell,
    kingdom: ActorId,
    variables: &'a mut player_glyph::State,
    players: &'a mut P,
}
fn nul(value: &[u16]) -> &[u16] {
    &value[..value.iter().position(|&v| v == 0).unwrap_or(value.len())]
}
impl<P: Players + ?Sized> World for Live<'_, '_, P> {
    type Variable = player_glyph::VariableWitness;
    type Player = P::Player;
    fn minor(&mut self) -> Result<bool> {
        Ok(*self
            .shell
            .words
            .get(&0x190)
            .ok_or("4E93C0 current Kingdom+190 owner absent")?
            != 0)
    }
    fn variable(&mut self) -> Result<Self::Variable> {
        self.variables.kingdom_variable(self.catalog)
    }
    fn find_player(&mut self) -> Result<Option<P::Player>> {
        let state = self.players.session_state_e4()?;
        //Only the non-state2 branch calls actual Kingdomvirtual24=4E5340.
        let name = if state == 2 {
            None
        } else {
            Some(
                self.shell
                    .strings_utf16
                    .get(&0x198)
                    .ok_or("4E5340 current Kingdom+198 CString absent")?,
            )
        };
        for index in 0..self.players.roster_len()? {
            let player = self.players.roster_player(index)?;
            let matches = if state == 2 {
                self.players.kingdom_765e00(&player)? == Some(self.kingdom)
            } else {
                nul(self.players.display_name_8(&player)?) == nul(name.unwrap())
            };
            if matches {
                return Ok(Some(player));
            }
        }
        Ok(None)
    }
    fn player_glyph(&mut self, player: &P::Player) -> Result<Option<u16>> {
        self.variables
            .glyph(self.catalog, self.bank, self.players, player)
            .map(|v| v.map(|g| g.index()))
    }
    fn fallback(&mut self, variable: &Self::Variable) -> Result<u16> {
        self.variables
            .kingdom_fallback(self.catalog, self.bank, variable)?
            .map(|g| g.index())
            .ok_or_else(|| "4E93C0 would dereference a null fallback GlyphData".into())
    }
    fn rgb(&mut self) -> Result<[u32; 3]> {
        let color = self
            .shell
            .references
            .get(&0x1e8)
            .ok_or("4E93C0 current KingdomColor reference unbound")?
            .as_ref()
            .ok_or("4E93C0 would dereference null KingdomColor")?;
        current_rgb(self.catalog, self.bank, color)
    }
    fn formatted(&mut self, rgb: [u32; 3], glyph: u16) -> Result<Vec<u16>> {
        format_color(rgb, glyph)
    }
}

///The same actual ColorData+18 RGB source used by451F00. This does not
///supply a color when the current KingdomColor pointer is null/unbound.
pub fn current_rgb(
    catalog: &Catalog<'_>,
    bank: &Bank,
    color: &aa_mapworld::restore::NamedDefinition,
) -> Result<[u32; 3]> {
    catalog.require_source_owner()?;
    let source = bank.combat_source()?;
    source.completed_means()?;
    if source.source_identities() != catalog.actors.sources {
        return Err("4E93C0 Color Bank/source owner differs".into());
    }
    let registry = catalog
        .actors
        .native_data_registry("KingdomColor")
        .ok_or("4E93C0 KingdomColor registry absent")?;
    let mut matches = registry
        .entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.ids.eq_ignore_ascii_case(&color.id));
    let (index, entry) = matches
        .next()
        .ok_or("4E93C0 current Color source identity absent")?;
    if matches.next().is_some() || color.index.is_some_and(|v| v as usize != index) {
        return Err("4E93C0 Color current source allocation is ambiguous or changed".into());
    }
    let node = catalog
        .actors
        .native_data_node(entry)
        .map_err(|e| e.to_string())?;
    let Some(TypedValue::Point { bits, .. }) = node
        .field("color", source.source_precision())
        .map_err(|e| e.to_string())?
    else {
        return Err("47E0F0 actual Color typed point3 getter absent".into());
    };
    bits.try_into()
        .map_err(|_| "47E0F0 Color getter is not point3".into())
}

///VC7.1 default %g: six significant decimal digits, half-up conversion,
///trailing zeros suppressed, scientific exponent padded to three digits.
///The4E93C0 arguments are exact promoted binary32 values, so decimal integer
///digits suffice without any host floating-point or locale dependency.
fn general(bits: u32) -> Result<String> {
    let exponent = (bits >> 23) & 255;
    if exponent == 255 {
        return Err("4E93C0 nonfinite Color CRT formatting is unsupported".into());
    }
    let negative = bits >> 31 != 0;
    let mantissa = if exponent == 0 {
        bits & 0x7fffff
    } else {
        (bits & 0x7fffff) | 0x800000
    };
    //VC7.1's default %g suppresses the sign of exact negative zero.
    if mantissa == 0 {
        return Ok("0".into());
    }
    let power = if exponent == 0 {
        -149
    } else {
        exponent as i32 - 150
    };
    let mut digits: Vec<u8> = mantissa
        .to_string()
        .bytes()
        .rev()
        .map(|b| b - b'0')
        .collect();
    for _ in 0..power.unsigned_abs() {
        let mut carry = 0u16;
        for digit in &mut digits {
            let value = u16::from(*digit) * if power < 0 { 5 } else { 2 } + carry;
            *digit = (value % 10) as u8;
            carry = value / 10;
        }
        while carry != 0 {
            digits.push((carry % 10) as u8);
            carry /= 10;
        }
    }
    digits.reverse();
    let mut decimal_exponent = digits.len() as i32 - if power < 0 { -power } else { 0 } - 1;
    let round = digits.get(6).is_some_and(|&v| v >= 5);
    digits.truncate(6);
    if round {
        let mut index = digits.len();
        loop {
            if index == 0 {
                digits.fill(0);
                digits[0] = 1;
                decimal_exponent += 1;
                break;
            }
            index -= 1;
            if digits[index] < 9 {
                digits[index] += 1;
                break;
            }
            digits[index] = 0;
        }
    }
    while digits.len() > 1 && digits.last() == Some(&0) {
        digits.pop();
    }
    let mut text = String::new();
    if negative {
        text.push('-');
    }
    if (-4..6).contains(&decimal_exponent) {
        let point = decimal_exponent + 1;
        if point <= 0 {
            text.push_str("0.");
            for _ in 0..-point {
                text.push('0');
            }
            for d in digits {
                text.push(char::from(b'0' + d));
            }
        } else {
            let width = point as usize;
            for i in 0..digits.len().max(width) {
                if i == width {
                    text.push('.');
                }
                text.push(char::from(b'0' + digits.get(i).copied().unwrap_or(0)));
            }
        }
    } else {
        text.push(char::from(b'0' + digits[0]));
        if digits.len() > 1 {
            text.push('.');
            for d in &digits[1..] {
                text.push(char::from(b'0' + d));
            }
        }
        text.push('e');
        text.push(if decimal_exponent < 0 { '-' } else { '+' });
        text.push_str(&format!("{:03}", decimal_exponent.unsigned_abs()));
    }
    Ok(text)
}
fn format_color(rgb: [u32; 3], glyph: u16) -> Result<Vec<u16>> {
    if glyph == u16::MAX {
        return Err("4E93C0 CRT%c WEOF conversion failure is unsupported".into());
    }
    let mut text = format!(
        "<color={},{},{}>",
        general(rgb[0])?,
        general(rgb[1])?,
        general(rgb[2])?
    )
    .encode_utf16()
    .collect::<Vec<_>>();
    text.push(glyph);
    Ok(text)
}

#[cfg(test)]
#[path = "load_kingdom_glyph_tests.rs"]
mod tests;
