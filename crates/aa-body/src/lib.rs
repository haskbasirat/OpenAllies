//! Original BodyComponent health, damage sequencing and corpse initialization.
//! Required synchronous world callbacks preserve actor graph, effects, capture,
//! ownership and death-queue boundaries. Legacy plain helpers deliberately reject
//! linked bodies; source-aware health and damage APIs dispatch them explicitly.
//! Credit/statistics and member/leader experience routing have separate modules;
//! actual world ownership, promotions and effect application remain required.
#![forbid(unsafe_code)]

use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

pub mod damage;
pub mod dying;
pub mod credit;
pub mod economy_modifiers;
pub mod experience;
pub mod experience_amounts;
pub mod promotion;
pub mod properties;
pub mod property_effects;
pub mod property_flags;
pub mod property_resistance;
pub mod property_refresh;
pub mod property_admission;
pub mod property_api;
pub mod updates;
pub mod modifier_load;
pub mod modifier_tables;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("nonfinite body input or callback-produced state")]
    NonFinite,
    #[error("maximum health must be positive at this runtime boundary")]
    InvalidMaximum,
    #[error("linked-actor propagation must be resolved by the actor runtime")]
    LinkedActorUnsupported,
    #[error("healer routes healing through DealDamage; this combat branch is not implemented")]
    DamageConvertingHealerUnsupported,
    #[error("callback failed: {0}; prior side effects are not rolled back")]
    Callback(String),
    #[error(transparent)]
    Arithmetic(#[from] aa_numeric::Error),
}
pub type Result<T> = std::result::Result<T, Error>;

/// Bit-preserved original binary32 field. Validation occurs at each API entry
/// and after callbacks; serialization does not create unverified live defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct F32(pub u32);
impl F32 {
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(0x3f80_0000);
    fn number(self) -> Result<Binary> {
        Binary::from_f32_bits(self.0).map_err(|_| Error::NonFinite)
    }
    fn compare(self, other: Self) -> Result<Ordering> {
        Ok(self.number()?.compare(&other.number()?))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Body {
    /// Native+0x10.
    pub health: F32,
    /// Native+0x14, resolved from derived getter45D730 at construction.
    pub maximum: F32,
    /// Native+0x18, resolved from derived getter45D760; unused by these operations.
    pub defense: F32,
    /// Native+0x1C, set by dying::start_dying (562380).
    pub dying_started: F32,
    /// Native+0x20.
    pub last_heal_time: F32,
    /// Native+0x24, set by start-dying, possibly terrain-adjusted.
    pub sink_duration: F32,
    /// Native+0x28..+0x40. Damage information remains opaque here.
    pub last_damage: [u32; 7],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthRule {
    /// Native FXC+0x30:0 crosses down,1 crosses up,2 drop,3 gain;
    /// other values are skipped by the original switch.
    pub kind: u32,
    pub threshold: F32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealRequest {
    pub source_id: u32,
    pub amount: F32,
    pub token: u32,
    pub converts_to_damage: bool,
}

/// Native561E50 arguments with the damage record's first GTID resolved by the
/// host only when a health-effect rule matches. A missing record is `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthChange {
    pub requested: F32,
    pub token: u32,
    pub notify: bool,
    pub source_id: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Environment {
    /// Native GTID, opaque to this component; aa-actors owns registry validation.
    pub actor_id: u32,
    /// A valid linked actor with Body. Null/stale references resolve to false.
    /// Plain helpers reject true; source-aware health/damage require host dispatch.
    pub has_linked_body: bool,
    /// Owner+8 bit0 (already destructed).
    pub destructed: bool,
    /// Data+0x291, original BodyComponent.destructible.
    pub destructible: bool,
    /// Original SVar BodyRestoreDisabledHealthRatio, not a guessed default.
    pub restore_disabled_ratio: F32,
    /// Owner+0x100 raw flags;0x200000 suppresses health FX,0x20000000
    /// selects derived normalization maximum instead of current maximum.
    pub actor_flags: u32,
    pub derived_normalization_maximum: F32,
    /// Original linked-list order; no sorting is performed.
    pub health_rules: Vec<HealthRule>,
    pub owner_position: [F32; 3],
    /// Owner+0x9C presence. Callback566AC0 implementation is external.
    pub has_recovery_component: bool,
    /// Native world+0xE0, read after the healing callbacks.
    pub world_time: F32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    /// Native actor4D0050, after storing health and before disabled/FX checks.
    RefreshActorStance,
    /// Native actor4D64F0(state39,enabled); actual state-machine side effects
    /// remain external. Repeated identical requests must not be invented here.
    ActorState {
        state: u32,
        enabled: bool,
    },
    /// Native429290 call on the whole WhenHealth list, triggered by first match.
    /// Directional attacker offset is outside this no-attacker boundary.
    HealthEffects {
        first_matching_rule: usize,
        old_ratio: F32,
        new_ratio: F32,
        position: [F32; 3],
        offset: [F32; 3],
    },
    /// Native566AC0, when health newly reaches full; original opaque heal token.
    Recovered {
        token: u32,
    },
    /// Native50C380 before heal modifies health.
    HealingNotification {
        source_id: u32,
        amount: F32,
        token: u32,
    },
    /// Native owner virtual+0x2C(1), followed by actor4D7960.
    ActorVirtual2c {
        argument: u32,
    },
    BeginDeath,
}

/// Callbacks may mutate body/context, as original code rereads them. Failure is
/// nontransactional: callers must not automatically retry an operation.
pub trait Host {
    fn event(
        &mut self,
        event: Event,
        body: &mut Body,
        environment: &mut Environment,
    ) -> std::result::Result<(), String>;

    /// The actual linked-Body target at actor+168 must receive this setter before
    /// local clamping. Source context is intentionally cleared by the native call.
    fn linked_health(
        &mut self,
        _change: HealthChange,
        _body: &mut Body,
        _environment: &mut Environment,
    ) -> std::result::Result<(), String> {
        Err("linked Body setter requires an actor registry/graph implementation".into())
    }

    /// Pure generation-checked401060/432E40 lookup at the point of FX emission.
    /// Return None only for a null/stale/missing native reference, not as fallback
    /// for an unimplemented world. Existing no-source operations never call this.
    fn health_source_position(
        &mut self,
        _source_id: u32,
    ) -> std::result::Result<Option<[F32; 3]>, String> {
        Err("damage-source position requires native actor registry resolution".into())
    }

    ///45D730 with the actual4CF110 modifier context. The native result can be
    /// extended (base health times modifier) and must not be prematurely stored
    /// to binary32 before FX normalization. Only the source-aware API calls this.
    fn health_normalization_maximum(
        &mut self,
        _body: &Body,
        _environment: &Environment,
        _precision: Precision,
    ) -> std::result::Result<Binary, String> {
        Err("derived health normalization requires the actual modifier context".into())
    }
}

/// 561E50's directional FX branch: subtract XYZ with three binary32 stores,
/// normalize XY through409230, scale XY by abs(new_ratio-rule_threshold), retain
/// the original unscaled Z difference. This is deliberately not a3D normal.
fn health_effect_offset(
    owner: [F32; 3],
    source: [F32; 3],
    ratio: F32,
    threshold: F32,
    precision: Precision,
) -> Result<[F32; 3]> {
    let mut offset = [F32::ZERO; 3];
    for i in 0..3 {
        offset[i] = F32(owner[i]
            .number()?
            .sub(&source[i].number()?, precision)?
            .store_f32()?);
    }
    let x = offset[0].number()?;
    let y = offset[1].number()?;
    let length = x
        .mul(&x, precision)?
        .add(&y.mul(&y, precision)?, precision)?
        .sqrt(precision)?;
    if length.compare(&F32(0x38d1_b717).number()?) == Ordering::Greater {
        let inverse = Binary::from_u32(1).div(&length, precision)?;
        offset[0] = F32(inverse.mul(&x, precision)?.store_f32()?);
        offset[1] = F32(inverse.mul(&y, precision)?.store_f32()?);
    } else {
        offset[0] = F32::ZERO;
        offset[1] = F32::ZERO;
    }
    let scale = ratio.number()?.sub(&threshold.number()?, precision)?;
    let scale = if scale.is_negative_nonzero() {
        scale.negated()
    } else {
        scale
    };
    offset[0] = F32(offset[0].number()?.mul(&scale, precision)?.store_f32()?);
    offset[1] = F32(offset[1].number()?.mul(&scale, precision)?.store_f32()?);
    Ok(offset)
}

impl Body {
    ///561D40 after the derived maximum getter returned an exact binary32 value
    /// (the null-kingdom path). Ratio is stored to f32 before multiplying.
    /// The native below-one early return can leave health1 with maximum<1.
    pub fn update_resolved_maximum(
        &mut self,
        maximum: F32,
        preserve_ratio: bool,
        env: &Environment,
        precision: Precision,
    ) -> Result<()> {
        self.validate(env)?;
        if maximum.compare(F32::ZERO)? != Ordering::Greater {
            return Err(Error::InvalidMaximum);
        }
        let ratio = F32(self
            .health
            .number()?
            .div(&self.maximum.number()?, precision)?
            .store_f32()?);
        self.maximum = maximum;
        if preserve_ratio {
            self.health = F32(ratio
                .number()?
                .mul(&maximum.number()?, precision)?
                .store_f32()?);
        }
        if self.health.compare(F32::ONE)? == Ordering::Less {
            self.health = F32::ONE;
        } else if self.maximum.compare(self.health)? == Ordering::Less {
            self.health = self.maximum;
        }
        Ok(())
    }

    ///5617E0 observed through a binary32 caller store. It does not clamp progress
    /// to1 and does not begin dying, remove an actor, or schedule anything.
    pub fn sink_fraction_f32(&self, now: F32, rot_time: F32, precision: Precision) -> Result<F32> {
        self.dying_started.number()?;
        self.sink_duration.number()?;
        now.number()?;
        rot_time.number()?;
        if self.dying_started.compare(F32::ZERO)? != Ordering::Less {
            let elapsed = F32(now
                .number()?
                .sub(&self.dying_started.number()?, precision)?
                .store_f32()?);
            if elapsed.compare(rot_time)? != Ordering::Less
                && self.sink_duration.compare(F32(0x38d1_b717))? != Ordering::Less
            {
                return Ok(F32(elapsed
                    .number()?
                    .sub(&rot_time.number()?, precision)?
                    .div(&self.sink_duration.number()?, precision)?
                    .store_f32()?));
            }
        }
        Ok(F32::ZERO)
    }

    /// Exact body field initialization of561960 after the actor base constructor
    /// and two derived getters supplied these values. It does not evaluate TGI.
    pub fn from_resolved(maximum: F32, defense: F32) -> Result<Self> {
        if maximum.compare(F32::ZERO)? != Ordering::Greater {
            return Err(Error::InvalidMaximum);
        }
        defense.number()?;
        Ok(Self {
            health: maximum,
            maximum,
            defense,
            dying_started: F32(0xbf80_0000),
            last_heal_time: F32(0xc479_c000),
            sink_duration: F32::ZERO,
            last_damage: [0, 0, 0, 0, 0, 0, 10],
        })
    }
    fn validate(&self, env: &Environment) -> Result<()> {
        self.validate_fields(env)?;
        if env.has_linked_body {
            return Err(Error::LinkedActorUnsupported);
        }
        Ok(())
    }
    fn validate_fields(&self, env: &Environment) -> Result<()> {
        for value in [
            self.health,
            self.maximum,
            self.defense,
            self.dying_started,
            self.last_heal_time,
            self.sink_duration,
            env.restore_disabled_ratio,
            env.derived_normalization_maximum,
            env.world_time,
        ] {
            value.number()?;
        }
        for value in env.owner_position {
            value.number()?;
        }
        for rule in &env.health_rules {
            rule.threshold.number()?;
        }
        if self.maximum.compare(F32::ZERO)? != Ordering::Greater {
            return Err(Error::InvalidMaximum);
        }
        Ok(())
    }
    fn emit(&mut self, env: &mut Environment, host: &mut impl Host, event: Event) -> Result<()> {
        host.event(event, self, env).map_err(Error::Callback)?;
        self.validate(env)
    }
    /// Full561E50 branch for no linked body and no attacker-direction context.
    /// This setter itself does not destroy a zero-health actor.
    pub fn set_health(
        &mut self,
        requested: F32,
        token: u32,
        notify: bool,
        env: &mut Environment,
        host: &mut impl Host,
        precision: Precision,
    ) -> Result<()> {
        self.validate(env)?;
        self.set_health_core(
            HealthChange {
                requested,
                token,
                notify,
                source_id: None,
            },
            env,
            host,
            precision,
            false,
        )
    }

    /// Complete finite setter including linked-body propagation and directional
    /// health effects. The host must supply the real graph and actor registry.
    pub fn set_health_from_source(
        &mut self,
        change: HealthChange,
        env: &mut Environment,
        host: &mut impl Host,
        precision: Precision,
    ) -> Result<()> {
        self.validate_fields(env)?;
        change.requested.number()?;
        if env.has_linked_body {
            host.linked_health(
                HealthChange {
                    source_id: None,
                    ..change
                },
                self,
                env,
            )
            .map_err(Error::Callback)?;
            self.validate_fields(env)?;
        }
        self.set_health_core(change, env, host, precision, true)
    }

    fn emit_health(
        &mut self,
        env: &mut Environment,
        host: &mut impl Host,
        event: Event,
        linked_supported: bool,
    ) -> Result<()> {
        host.event(event, self, env).map_err(Error::Callback)?;
        if linked_supported {
            self.validate_fields(env)
        } else {
            self.validate(env)
        }
    }

    fn set_health_core(
        &mut self,
        change: HealthChange,
        env: &mut Environment,
        host: &mut impl Host,
        precision: Precision,
        linked_supported: bool,
    ) -> Result<()> {
        let HealthChange {
            requested,
            token,
            notify,
            source_id,
        } = change;
        requested.number()?;
        let next = if requested.compare(F32::ZERO)? == Ordering::Less {
            F32::ZERO
        } else if requested.compare(self.maximum)? == Ordering::Greater {
            self.maximum
        } else {
            requested
        };
        if next.compare(self.health)? == Ordering::Equal {
            return Ok(());
        }
        let old = self.health;
        let was_full = old.compare(self.maximum)? != Ordering::Less;
        self.health = next;
        if notify {
            self.emit_health(env, host, Event::RefreshActorStance, linked_supported)?;
        }
        if !env.destructible {
            let direction = self.health.compare(old)?;
            if direction == Ordering::Less && self.health.compare(F32::ZERO)? != Ordering::Greater {
                self.emit_health(
                    env,
                    host,
                    Event::ActorState {
                        state: 39,
                        enabled: true,
                    },
                    linked_supported,
                )?;
            } else if direction == Ordering::Greater
                && self
                    .health
                    .number()?
                    .div(&self.maximum.number()?, precision)?
                    .compare(&env.restore_disabled_ratio.number()?)
                    != Ordering::Less
            {
                self.emit_health(
                    env,
                    host,
                    Event::ActorState {
                        state: 39,
                        enabled: false,
                    },
                    linked_supported,
                )?;
            }
        }
        if !env.health_rules.is_empty() && env.actor_flags & 0x20_0000 == 0 {
            let maximum = if env.actor_flags & 0x2000_0000 != 0 && linked_supported {
                host.health_normalization_maximum(self, env, precision)
                    .map_err(Error::Callback)?
            } else if env.actor_flags & 0x2000_0000 != 0 {
                env.derived_normalization_maximum.number()?
            } else {
                self.maximum.number()?
            };
            if maximum.compare(&Binary::from_u32(0)) != Ordering::Greater {
                return Err(Error::InvalidMaximum);
            }
            let inverse = Binary::from_u32(1).div(&maximum, precision)?;
            let old_ratio = F32(old.number()?.mul(&inverse, precision)?.store_f32()?);
            let new_ratio = F32(inverse
                .mul(&self.health.number()?, precision)?
                .store_f32()?);
            for (index, rule) in env.health_rules.iter().enumerate() {
                let threshold = rule.threshold.number()?;
                let matched = match rule.kind {
                    0 => {
                        new_ratio.compare(rule.threshold)? != Ordering::Greater
                            && old_ratio.compare(rule.threshold)? == Ordering::Greater
                    }
                    1 => {
                        new_ratio.compare(rule.threshold)? == Ordering::Greater
                            && old_ratio.compare(rule.threshold)? != Ordering::Greater
                    }
                    2 => {
                        threshold
                            .negated()
                            .compare(&new_ratio.number()?.sub(&old_ratio.number()?, precision)?)
                            == Ordering::Greater
                    }
                    3 => {
                        new_ratio
                            .number()?
                            .sub(&old_ratio.number()?, precision)?
                            .compare(&threshold)
                            == Ordering::Greater
                    }
                    _ => false,
                };
                if matched {
                    let offset = if let Some(source_id) = source_id {
                        match host
                            .health_source_position(source_id)
                            .map_err(Error::Callback)?
                        {
                            Some(position) => health_effect_offset(
                                env.owner_position,
                                position,
                                new_ratio,
                                rule.threshold,
                                precision,
                            )?,
                            None => [F32::ZERO; 3],
                        }
                    } else {
                        [F32::ZERO; 3]
                    };
                    self.emit_health(
                        env,
                        host,
                        Event::HealthEffects {
                            first_matching_rule: index,
                            old_ratio,
                            new_ratio,
                            position: env.owner_position,
                            offset,
                        },
                        linked_supported,
                    )?;
                    break;
                }
            }
        }
        if self.health.compare(self.maximum)? != Ordering::Less
            && !was_full
            && env.has_recovery_component
        {
            self.emit_health(env, host, Event::Recovered { token }, linked_supported)?;
        }
        Ok(())
    }
    /// Full562200 for a body without a valid linked Body; maximum lower bound is
    /// the original binary32 constant0x38D1B717 (0.0001), not1 or authored health.
    pub fn set_health_and_maximum(
        &mut self,
        health: F32,
        maximum: F32,
        env: &mut Environment,
        host: &mut impl Host,
        precision: Precision,
    ) -> Result<()> {
        self.validate(env)?;
        health.number()?;
        maximum.number()?;
        let minimum = F32(0x38d1_b717);
        let maximum = if maximum.compare(minimum)? == Ordering::Less {
            minimum
        } else {
            maximum
        };
        let health = if health.compare(maximum)? == Ordering::Greater {
            maximum
        } else {
            health
        };
        self.maximum = maximum;
        self.set_health(health, 0, true, env, host, precision)
    }
    /// Full562BA0 ordinary-healer/no-linked-body path. The source Mind+0xA8
    /// predicate can route a heal into DealDamage; true rejects before mutation.
    pub fn heal_plain(
        &mut self,
        request: HealRequest,
        env: &mut Environment,
        host: &mut impl Host,
        precision: Precision,
    ) -> Result<()> {
        let HealRequest {
            source_id,
            amount,
            token,
            converts_to_damage,
        } = request;
        self.validate(env)?;
        amount.number()?;
        if converts_to_damage {
            return Err(Error::DamageConvertingHealerUnsupported);
        }
        if self.health.compare(self.maximum)? == Ordering::Less
            && amount.compare(F32::ZERO)? == Ordering::Greater
            && !env.destructed
        {
            self.emit(
                env,
                host,
                Event::HealingNotification {
                    source_id,
                    amount,
                    token,
                },
            )?;
            let next = F32(self
                .health
                .number()?
                .add(&amount.number()?, precision)?
                .store_f32()?);
            self.set_health(next, token, true, env, host, precision)?;
            self.last_heal_time = env.world_time;
        }
        Ok(())
    }
    /// Full561C20: dispatches death callbacks for an un-destructed zero-health
    /// actor. `Event::BeginDeath` is actor 4D7960, ported as
    /// `aa_combat::begin_death::begin_death` (bound by the host).
    pub fn check_death(&mut self, env: &mut Environment, host: &mut impl Host) -> Result<()> {
        self.validate(env)?;
        if !env.destructed && self.health.compare(F32::ZERO)? != Ordering::Greater {
            self.emit(env, host, Event::ActorVirtual2c { argument: 1 })?;
            self.emit(env, host, Event::BeginDeath)?;
        }
        Ok(())
    }
}
pub mod actor_property_list;
