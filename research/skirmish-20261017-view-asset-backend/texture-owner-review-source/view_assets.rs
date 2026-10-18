//!Retained constructor fields for native VWorld assets (+110..+204).
//!The actual ordered initializer borrows the same resource/source/current
//!owners. This segment does not write VWorld AD or complete the graphics scene.
use crate::{
    catalog::Catalog,
    definitions::Bank,
    view_init_assets::{EffectSlot, TextureSlot},
    view_vfx_source::Effect,
    world_revision::{ConstructedWorldRevision, Witness},
};
type Result<T> = std::result::Result<T, String>;
///Actual TextureResource allocation, not a decoded/GPU-ready texture claim.
pub struct Texture {
    resource: crate::resource_objects::Handle,
}
impl Texture {
    pub fn resource(&self) -> &crate::resource_objects::Handle {
        &self.resource
    }
}
struct EnvironmentEffect {
    core: crate::view_texture_effect::Core,
    source_154: Option<crate::resource_texture::SourceTexture>,
}
///Reached effects outside the ResourceManager's own mutable fields. The real
///client supplies current UI policy, diagnostics and decoder/task services.
pub trait Services:
    crate::resource_depot::Backend + crate::resource_objects::TexturePolicy
{
    fn warning_404ea0(&mut self, _line: u32, _text: &'static str) -> Result<()> {
        Err("VWorld actual404EA0 warning channel is required".into())
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Constructed,
    Initializing,
    Complete,
    Failed,
}
pub struct State {
    world: Option<Witness>,
    phase: Phase,
    environment_texture_110: Option<Texture>,
    environment_effect_114: Option<EnvironmentEffect>,
    textures: [Option<Texture>; 4],
    effects: [Option<Effect>; 18],
    failed_environment_constructor: Option<crate::view_texture_effect::Core>,
}
fn texture_index(slot: TextureSlot) -> usize {
    match slot {
        TextureSlot::Circle => 0,
        TextureSlot::Square => 1,
        TextureSlot::HollowCircle => 2,
        TextureSlot::HollowSquare => 3,
    }
}
fn effect_index(slot: EffectSlot) -> usize {
    match slot {
        EffectSlot::MoveCommand => 0,
        EffectSlot::SelectFacing => 1,
        EffectSlot::HealthBar => 2,
        EffectSlot::MoraleBar => 3,
        EffectSlot::SupplyStockpileBar => 4,
        EffectSlot::AttackEfficiencyBar => 5,
        EffectSlot::Ghosting => 6,
        EffectSlot::IllegalPlacement => 7,
        EffectSlot::CircleBrush => 8,
        EffectSlot::SquareBrush => 9,
        EffectSlot::ProjectedBoxTexture => 10,
        EffectSlot::ProjectedCircleTexture => 11,
        EffectSlot::LocationMarkerEditor => 12,
        EffectSlot::RecruitMarkerRallyPoint => 13,
        EffectSlot::Text => 14,
        EffectSlot::TextUnscaled => 15,
        EffectSlot::DamageNumbers => 16,
        EffectSlot::HealNumbers => 17,
    }
}
impl State {
    ///Only the442C00 null-pointer field projection. No source scan, lookup,
    ///allocation-null approximation, child callback or complete view is implied.
    pub fn from_constructor_fields() -> Self {
        Self {
            world: None,
            phase: Phase::Constructed,
            environment_texture_110: None,
            environment_effect_114: None,
            textures: std::array::from_fn(|_| None),
            effects: std::array::from_fn(|_| None),
            failed_environment_constructor: None,
        }
    }
    pub fn texture(&self, slot: TextureSlot) -> Option<&Texture> {
        self.textures[texture_index(slot)].as_ref()
    }
    pub fn effect(&self, slot: EffectSlot) -> Option<&Effect> {
        self.effects[effect_index(slot)].as_ref()
    }
    pub fn environment_texture_present(&self) -> bool {
        self.environment_texture_110.is_some()
    }
    pub fn environment_effect_present(&self) -> bool {
        self.environment_effect_114.is_some()
    }
    ///Native43EA50 and43F1D4..43F948 using actual same manager allocations and
    ///current alias cells. On failure all completed stores remain in this
    ///owner and ResourceManager. A second Init requires the native teardown
    ///path; it cannot erase a failed prefix by restarting the coordinator.
    #[allow(clippy::too_many_arguments)]
    pub fn initialize(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        world: &ConstructedWorldRevision,
        aliases: &crate::view_asset_aliases::State,
        application: &mut crate::resource_depot::Application,
        services: &mut impl Services,
        globals: &mut impl crate::view_grid_state::Globals,
        pc: aa_numeric::Precision,
    ) -> Result<()> {
        if self.phase != Phase::Constructed {
            return Err(
                "VWorld assets Init already started; native teardown is required before restart"
                    .into(),
            );
        }
        aliases.validate_in_bank(catalog, bank)?;
        self.world = Some(world.witness());
        self.phase = Phase::Initializing;
        let result = crate::view_init_assets::initialize(&mut Bound {
            state: self,
            catalog,
            bank,
            aliases,
            resources: application.resources(),
            services,
            globals,
            pc,
        });
        self.phase = if result.is_ok() {
            Phase::Complete
        } else {
            Phase::Failed
        };
        result
    }
    ///A concrete completion guard, never a caller-supplied success bool. It
    ///certifies this asset segment only; actual render/scene completion is a
    ///separate owner and no AD publication occurs here.
    pub fn require_initialized(
        &self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        world: &ConstructedWorldRevision,
    ) -> Result<()> {
        if self.phase != Phase::Complete {
            return Err(
                "VWorld assets actual environment/resource initialization is incomplete".into(),
            );
        }
        if !self.world.as_ref().is_some_and(|v| v.is_for(world)) {
            return Err("VWorld assets belong to a replaced/different World".into());
        }
        catalog.require_source_owner()?;
        let source = bank.combat_source()?;
        source.completed_means()?;
        if source.source_identities() != catalog.actors.sources.as_slice() {
            return Err("VWorld assets source Bank differs".into());
        }
        for effect in self.effects.iter().flatten() {
            effect.validate(catalog, bank)?;
        }
        Ok(())
    }
}

struct Bound<'a, 'assets, B, G> {
    state: &'a mut State,
    catalog: &'a Catalog<'assets>,
    bank: &'a Bank,
    aliases: &'a crate::view_asset_aliases::State,
    resources: crate::resource_depot::Resources<'a>,
    services: &'a mut B,
    globals: &'a mut G,
    pc: aa_numeric::Precision,
}
impl<B: Services, G: crate::view_grid_state::Globals> Bound<'_, '_, B, G> {
    fn environment(&mut self) -> Result<()> {
        //43EA50 reads/copies current text twice: the first CString retires
        //after the empty test, before the reached Resource factory read.
        let alias = self
            .aliases
            .alias_410cb0(self.catalog, self.bank, "TextureEnvironmentMap")?;
        let nonempty = !alias.units().is_empty();
        drop(alias);
        if nonempty {
            let alias =
                self.aliases
                    .alias_410cb0(self.catalog, self.bank, "TextureEnvironmentMap")?;
            let resource = self.resources.texture_5a77e0_with_policy(
                self.catalog,
                self.bank,
                alias.units(),
                0,
                self.services,
            )?;
            self.state.environment_texture_110 = resource.map(|resource| Texture { resource });
            let texture = self
                .state
                .environment_texture_110
                .as_ref()
                .ok_or("43EA50 NULL TextureResource would fault before virtual0C")?;
            let priority = self.resources.current_priority_b282f0();
            self.resources
                .request_0c(&texture.resource, 1, priority, self.services)?;
            self.resources.block_18(&texture.resource, self.services)?;
            drop(alias);
        }
        let core = match crate::view_texture_effect::Core::from_native_constructor(self.globals) {
            Ok(core) => core,
            Err(failure) => {
                self.state.failed_environment_constructor = Some(failure.core);
                return Err(failure.error);
            }
        };
        self.state.environment_effect_114 = Some(EnvironmentEffect {
            core,
            source_154: None,
        });
        if let Some(texture) = &self.state.environment_texture_110 {
            let source = self.resources.texture_5a63d0(&texture.resource)?;
            //5A63D0's actual output temporary retains before the154 smart
            //assignment; its release follows the destination's own retain.
            if let Some(source) = &source {
                self.resources.retain_source_texture(source)?;
            }
            let effect = self
                .state
                .environment_effect_114
                .as_mut()
                .expect("stored actual effect");
            let same = match (&effect.source_154, &source) {
                (None, None) => true,
                (Some(a), Some(b)) => a.same_identity(b),
                _ => false,
            };
            if !same {
                if let Some(old) = &effect.source_154 {
                    self.resources.release_source_texture(old)?;
                }
                effect.source_154 = source.clone();
                if let Some(source) = &source {
                    self.resources.retain_source_texture(source)?;
                }
            }
            if let Some(source) = &source {
                self.resources.release_source_texture(source)?;
            }
        }
        let effect = self
            .state
            .environment_effect_114
            .as_mut()
            .expect("stored actual effect");
        effect.core.configure_environment_43ea50()?;
        effect.core.project_mode2_86ba10(self.pc)?;
        effect.core.enable_after_projection_43ea50()?;
        effect.core.update_fresh_86e8d0(0, 1, self.pc)
    }
}
impl<B: Services, G: crate::view_grid_state::Globals> crate::view_init_assets::Services
    for Bound<'_, '_, B, G>
{
    type Alias = crate::view_asset_aliases::Alias;
    type Texture = Texture;
    type Effect = Effect;
    fn environment_43ea50(&mut self) -> Result<()> {
        self.environment()
    }
    fn alias_410cb0(&mut self, key: &'static str) -> Result<Self::Alias> {
        self.aliases.alias_410cb0(self.catalog, self.bank, key)
    }
    fn texture_5a77e0(&mut self, alias: &Self::Alias, flags: u32) -> Result<Option<Self::Texture>> {
        self.resources
            .texture_5a77e0_with_policy(
                self.catalog,
                self.bank,
                alias.units(),
                flags,
                self.services,
            )
            .map(|v| v.map(|resource| Texture { resource }))
    }
    fn effect_4bdb70(&mut self, alias: &Self::Alias) -> Result<Option<Self::Effect>> {
        crate::view_vfx_source::find(self.catalog, self.bank, alias.units())
    }
    fn store_texture(&mut self, slot: TextureSlot, value: Option<Self::Texture>) {
        self.state.textures[texture_index(slot)] = value
    }
    fn store_effect(&mut self, slot: EffectSlot, value: Option<Self::Effect>) {
        self.state.effects[effect_index(slot)] = value
    }
    fn effect_present(&self, slot: EffectSlot) -> bool {
        self.state.effects[effect_index(slot)].is_some()
    }
    fn effect_kind(&self, slot: EffectSlot) -> Result<u32> {
        self.state.effects[effect_index(slot)]
            .as_ref()
            .ok_or("VWorld effect field is NULL")?
            .kind(self.catalog, self.bank)
    }
    fn warning_404ea0(&mut self, line: u32, text: &'static str) -> Result<()> {
        self.services.warning_404ea0(line, text)
    }
}
