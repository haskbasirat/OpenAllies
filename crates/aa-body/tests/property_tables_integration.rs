//! property_flags insert/remove driving the table modifiers (502B30/502C60)
//! through the shared 581630/581810 lifetime: the immunity purge removes a
//! hostile enchantment through the same removal path, and removal unwinds
//! the tables. The table operations themselves are native-proven by
//! tests/native_modifier_tables.rs; this checks the composition.
use aa_body::{
    F32, Result,
    credit::Object,
    modifier_tables::TableModifier,
    properties::{ModifierId, PropertiesState},
    property_effects::{self as life, Instance, Instances, NumericModifier, Selection},
    property_flags::{self, Definition, Definitions, Modifier},
};
use aa_numeric::Precision;

const ENCH_TYPE: Object = Object(500);
const KINGDOM_ALLY: u32 = (1 << 16) | 3;
const KINGDOM_ENEMY: u32 = (1 << 16) | 4;

#[derive(Default)]
struct Host {
    log: Vec<String>,
    created: u32,
}
impl life::Effects for Host {
    fn resolve_source(&self, id: u32) -> Result<Option<Object>> {
        Ok((id >> 16 == 1).then_some(Object(id & 0xffff)))
    }
    fn allegiance(&mut self, _: &mut PropertiesState, _: &mut Instances) -> Result<Option<Object>> {
        Ok(None)
    }
    fn numeric_changed(
        &mut self,
        _: &NumericModifier,
        _: &mut PropertiesState,
        _: &mut Instances,
    ) -> Result<()> {
        Ok(())
    }
    fn property_added(
        &mut self,
        h: Object,
        _: &mut PropertiesState,
        _: &mut Instances,
    ) -> Result<()> {
        self.log.push(format!("added {}", h.0));
        Ok(())
    }
    fn actor_added(&mut self, _: Object, _: &mut PropertiesState, _: &mut Instances) -> Result<()> {
        Ok(())
    }
    fn actor_removed(
        &mut self,
        _: Object,
        _: &mut PropertiesState,
        _: &mut Instances,
    ) -> Result<()> {
        Ok(())
    }
    fn property_removed(
        &mut self,
        h: Object,
        _: &mut PropertiesState,
        _: &mut Instances,
    ) -> Result<()> {
        self.log.push(format!("removed {}", h.0));
        Ok(())
    }
    fn released(&mut self, _: Instance) -> Result<()> {
        Ok(())
    }
}
impl property_flags::Effects for Host {
    fn has_mover(&self, _: Object) -> Result<bool> {
        Ok(false)
    }
    fn set_mover_byte_4c(
        &mut self,
        _: Object,
        _: bool,
        _: &mut PropertiesState,
        _: &mut Instances,
    ) -> Result<()> {
        Ok(())
    }
    fn actor_stop_4d4c60(
        &mut self,
        _: Object,
        _: bool,
        _: &mut PropertiesState,
        _: &mut Instances,
    ) -> Result<()> {
        Ok(())
    }
    fn has_character(&self, _: Object) -> Result<bool> {
        Ok(false)
    }
    fn character_pause_4daf20(
        &mut self,
        _: Object,
        _: &mut PropertiesState,
        _: &mut Instances,
    ) -> Result<()> {
        Ok(())
    }
    fn character_resume_4daf40(
        &mut self,
        _: Object,
        _: &mut PropertiesState,
        _: &mut Instances,
    ) -> Result<()> {
        Ok(())
    }
    fn mover_576800(
        &mut self,
        _: Object,
        _: bool,
        _: &mut PropertiesState,
        _: &mut Instances,
    ) -> Result<()> {
        Ok(())
    }
    fn has_economy(&self, _: Object) -> Result<bool> {
        Ok(false)
    }
    fn economy_570020(
        &mut self,
        _: Object,
        _: &mut PropertiesState,
        _: &mut Instances,
    ) -> Result<()> {
        Ok(())
    }
    fn rebuild_actions_4d1350(
        &mut self,
        _: Object,
        _: &mut PropertiesState,
        _: &mut Instances,
    ) -> Result<()> {
        Ok(())
    }
    fn resource_count(&self) -> Result<usize> {
        Ok(2)
    }
    fn refresh_actor_4cf0f0(
        &mut self,
        _: Object,
        _: &mut PropertiesState,
        _: &mut Instances,
    ) -> Result<()> {
        self.log.push("refresh".into());
        Ok(())
    }
    fn create_reverse_ability(&mut self, _: Object) -> Result<Object> {
        self.created += 1;
        Ok(Object(900 + self.created))
    }
    fn destroy_reverse_ability(&mut self, ability: Object) -> Result<()> {
        self.log.push(format!("destroy {}", ability.0));
        Ok(())
    }
    fn enchantment_type(&self, definition: Object) -> Result<Option<Object>> {
        Ok((definition == Object(20)).then_some(ENCH_TYPE))
    }
    fn allied(
        &mut self,
        _: Object,
        allegiance: Option<Object>,
        _: &mut PropertiesState,
        _: &mut Instances,
    ) -> Result<bool> {
        Ok(allegiance == Some(Object(KINGDOM_ALLY & 0xffff)))
    }
}

fn definitions() -> Definitions {
    let mut d = Definitions::new();
    // 10: immunity + periodic damage + production + reverse ability.
    d.insert(
        Object(10),
        Definition {
            flags: 0,
            modifiers: vec![
                Modifier::Table(TableModifier::Immunity {
                    enchantment_type: Some(ENCH_TYPE),
                }),
                Modifier::Table(TableModifier::PeriodicDamage {
                    damage_type: Object(7),
                    amount: F32(5.0f32.to_bits()),
                }),
                Modifier::Table(TableModifier::ResourceProduction {
                    values: vec![F32(1.5f32.to_bits()), F32(2.0f32.to_bits())],
                }),
                Modifier::Table(TableModifier::ReverseAbility {
                    definition: Object(33),
                }),
            ],
        },
    );
    // 20: an enchantment property of the immune type with a heal.
    d.insert(
        Object(20),
        Definition {
            flags: 0,
            modifiers: vec![Modifier::Table(TableModifier::PeriodicHeal {
                amount: F32(3.0f32.to_bits()),
            })],
        },
    );
    d
}

#[test]
fn immunity_purges_hostile_enchantments_and_removal_unwinds() {
    let defs = definitions();
    let mut host = Host::default();
    let mut state = PropertiesState::new(Object(1), 0).unwrap();
    let mut instances = Instances::new();
    let pc = Precision::Double53;
    let minimum = F32(0x3dcc_cccd);
    // Two enchantments of the immune type: one from an ally, one hostile.
    let friendly = Instance::enchantment(
        Object(101),
        Object(20),
        F32::ZERO,
        Some((1 << 16) | 8),
        Some(KINGDOM_ALLY),
    );
    let hostile = Instance::enchantment(
        Object(102),
        Object(20),
        F32::ZERO,
        Some((1 << 16) | 9),
        Some(KINGDOM_ENEMY),
    );
    assert!(
        property_flags::insert(
            friendly,
            &mut state,
            &mut instances,
            &defs,
            minimum,
            pc,
            &mut host
        )
        .unwrap()
    );
    // Duplicate definition: disabled, no table add.
    assert!(
        !property_flags::insert(
            hostile,
            &mut state,
            &mut instances,
            &defs,
            minimum,
            pc,
            &mut host
        )
        .unwrap()
    );
    assert_eq!(state.modifier.periodic_heals.len(), 1);
    // The immunity property: purges the hostile (disabled) duplicate only.
    let immune = Instance::new(Object(103), Object(10), 2, F32::ZERO, Some((1 << 16) | 5));
    assert!(
        property_flags::insert(
            immune,
            &mut state,
            &mut instances,
            &defs,
            minimum,
            pc,
            &mut host
        )
        .unwrap()
    );
    assert_eq!(state.effects, vec![Object(101), Object(103)]);
    assert_eq!(state.modifier.immunity_types, vec![Some(ENCH_TYPE)]);
    assert_eq!(state.modifier.periodic_damage.len(), 1);
    let record = state.modifier.periodic_damage[0];
    assert_eq!(
        record.modifier,
        ModifierId {
            definition: Object(10),
            index: 1
        }
    );
    assert_eq!((record.source_id, record.allegiance_id), ((1 << 16) | 5, 0));
    assert_eq!(
        state.modifier.production,
        vec![F32(1.5f32.to_bits()), F32(2.0f32.to_bits())]
    );
    assert_eq!(state.modifier.reverse_abilities.len(), 1);
    assert!(host.log.contains(&"refresh".to_owned()));
    // Removing the immunity property unwinds every table.
    assert!(
        property_flags::remove(
            Selection {
                definition: Object(10),
                source: Some(Object(5)),
            },
            &mut state,
            &mut instances,
            &defs,
            minimum,
            pc,
            &mut host,
        )
        .unwrap()
    );
    assert!(state.modifier.immunity_types.is_empty());
    assert!(state.modifier.periodic_damage.is_empty());
    assert!(state.modifier.reverse_abilities.is_empty());
    assert_eq!(state.modifier.production, vec![F32::ZERO, F32::ZERO]);
    assert!(host.log.contains(&"destroy 901".to_owned()));
    // The friendly heal stays.
    assert_eq!(state.modifier.periodic_heals.len(), 1);
}
