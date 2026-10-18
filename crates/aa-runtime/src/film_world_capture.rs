//! Concrete5A26E0 current-World composition. All children borrow the same
//! retained World/actor/text/roster owners; the destination is an independent
//! fresh film Header, never a decoded source Header or a final clone commit.
use crate::{
    actor_world::ActorStore, application_mode::GameAppState, catalog::Catalog, definitions::Bank,
    film_header_entries, film_header_fields, film_header_owner, load_kingdom_glyph,
    load_sai_players, load_world::World, localization::Localization, player_glyph,
};

pub type Result<T> = std::result::Result<T, String>;

/// Loans from the caller's actual managed graph. `players` is one roster
/// facade providing both4E93C0 and5A17D0; formatter and glyph variables remain
/// application-owned across repeated captures. The clock is called lazily at
/// the native6FB1F0 boundary, not when constructing this context.
pub struct Context<'a, 'assets, P, C> {
    pub catalog: &'a Catalog<'assets>,
    pub bank: &'a Bank,
    pub world: &'a mut World,
    pub actors: &'a ActorStore,
    pub live_text: &'a mut Localization,
    pub application: &'a GameAppState,
    pub ai_manager: &'a aa_mind::sai::manager::Sai,
    pub ai_players: &'a load_sai_players::Arena,
    pub formatter: &'a mut load_kingdom_glyph::Formatter,
    pub glyph_variables: &'a mut player_glyph::State,
    pub players: &'a mut P,
    pub current_local_filetime: C,
}

/// Immediate Header mutations and current49AE00 text writes survive every
/// child failure. No eager child calls, fallback Campaign/roster values,
/// inferred terrain dimensions or replacement source-rule owner are used.
pub fn capture<P, C>(
    target: &mut film_header_owner::State,
    mut context: Context<'_, '_, P, C>,
) -> Result<()>
where
    P: load_kingdom_glyph::Players + film_header_entries::Players,
    C: FnMut() -> Result<[u32; 2]>,
{
    film_header_fields::capture(target, &mut context)
}

impl<P, C> film_header_fields::Source<film_header_owner::State> for Context<'_, '_, P, C>
where
    P: load_kingdom_glyph::Players + film_header_entries::Players,
    C: FnMut() -> Result<[u32; 2]>,
{
    type Campaign = crate::campaign_manager::Reference;
    fn current_local_filetime(&mut self) -> Result<[u32; 2]> {
        (self.current_local_filetime)()
    }
    fn mode_10(&mut self) -> Result<u32> {
        Ok(self.world.world_rules.mode())
    }
    fn game_type_14(&mut self) -> Result<Vec<u16>> {
        Ok(self.world.world_rules.game_type().to_vec())
    }
    fn copy_rules_18(&mut self, target: &mut crate::load_world_rules::CreatorRules) -> Result<()> {
        self.world
            .world_rules
            .copy_records_into(target, self.catalog, self.bank)
    }
    fn rule_extra_24(&mut self) -> Result<u32> {
        Ok(self.world.world_rules.changes())
    }
    fn capture_parameters_510eb0(
        &mut self,
        target: &mut crate::film_parameters::State,
    ) -> Result<()> {
        target.capture_teams(
            self.catalog,
            self.bank,
            self.actors,
            &self.world.registration,
        )?;
        target.capture_kingdoms_current(
            self.catalog,
            self.bank,
            self.actors,
            &self.world.kingdoms,
            self.live_text,
            self.formatter,
            self.glyph_variables,
            self.players,
        )?;
        crate::film_parameter_extras::capture_current(
            target,
            self.actors,
            self.world.registration.names(),
            self.world
                .trigger_manager
                .as_ref()
                .ok_or("5A26E0 current World trigger manager is absent")?,
            &mut self.world.objectives,
            self.live_text,
        )
    }
    fn extent_3c(&mut self) -> Result<u32> {
        Ok(self.world.creator_extents.as_ref().ok_or("actual WorldInit Creator extents are unbound")?.bits()[0])
    }
    fn extent_40(&mut self) -> Result<u32> {
        Ok(self.world.creator_extents.as_ref().ok_or("actual WorldInit Creator extents are unbound")?.bits()[1])
    }
    fn current_campaign_4f1310(&mut self) -> Result<Option<Self::Campaign>> {
        self.application
            .campaigns()
            .current_4f1310(self.application)
    }
    fn lookup_campaign_411790(&mut self, create: bool) -> Result<Option<Self::Campaign>> {
        self.application
            .campaigns()
            .lookup_411790(self.application, create)
    }
    fn campaign_name_08(&mut self, current: &Self::Campaign) -> Result<Vec<u16>> {
        Ok(self
            .application
            .campaigns()
            .name_08(self.application, current)?
            .to_vec())
    }
    fn campaign_index_14(&mut self, looked_up: &Self::Campaign) -> Result<u32> {
        self.application
            .campaigns()
            .index_14(self.application, looked_up)
    }
    fn capture_entries_5a17d0(
        &mut self,
        parameters: &mut crate::film_parameters::State,
        entries: &mut Vec<aa_mapworld::header::HeaderEntry>,
    ) -> Result<()> {
        film_header_entries::capture_current(
            parameters,
            entries,
            self.players,
            &mut film_header_entries::LiveWorld::new(self.actors, self.ai_manager, self.ai_players),
        )
    }
}
