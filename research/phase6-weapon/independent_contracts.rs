//! Standalone reviewer tests: checked-domain and world-error boundaries.
use aa_body::{F32, damage::DamageRequest};
use aa_combat::{DamageRule, Error, Hit, Result, Target, Weapon, World};
use aa_numeric::{Binary, Precision};
use std::cell::RefCell;

#[derive(Default)]
struct Services {
    calls: RefCell<Vec<&'static str>>,
    absent: bool,
    fail_offense: bool,
    fail_delivery: bool,
    deliveries: usize,
}
impl Services {
    fn called(&self, name: &'static str) {
        self.calls.borrow_mut().push(name);
    }
}
impl World for Services {
    fn resolve_target(&mut self, _: Option<u32>) -> Result<Option<Target>> {
        self.called("target");
        Ok((!self.absent).then_some(Target { actor: 3, body: 7 }))
    }
    fn actor_id(&self, actor: u32) -> Result<u32> {
        self.called("id");
        Ok(actor)
    }
    fn modifier(&self, _: u32) -> Result<Option<u32>> {
        self.called("modifier");
        Ok(Some(1))
    }
    fn damage_type_factor(&self, _: u32, _: u32) -> Result<Binary> {
        self.called("type_factor");
        Ok(Binary::from_i32(1))
    }
    fn offensive_component(&self, _: u32) -> Result<Option<u32>> {
        self.called("offense_component");
        Ok(Some(1))
    }
    fn offensive_factor(&self, _: u32) -> Result<Binary> {
        self.called("offense");
        if self.fail_offense {
            Err(Error::World("offense failed".into()))
        } else {
            Ok(Binary::from_i32(1))
        }
    }
    fn defense(&self, _: u32) -> Result<F32> {
        self.called("defense");
        Ok(F32::ZERO)
    }
    fn minimum_weapon_damage(&self) -> Result<F32> {
        self.called("minimum");
        Ok(F32::ONE)
    }
    fn deliver(&mut self, _: Target, _: DamageRequest, _: Precision) -> Result<()> {
        self.called("deliver");
        self.deliveries += 1;
        if self.fail_delivery {
            Err(Error::World("delivery failed after effect".into()))
        } else {
            Ok(())
        }
    }
}
fn hit() -> Hit {
    Hit {
        rule: DamageRule {
            kind: 4,
            damage_type: 1,
            amount: F32::ONE,
        },
        weapon: Some(Weapon {
            id: 1,
            amount: F32::ONE,
            flags: 0,
            propagation: aa_body::damage::WeaponPropagation {
                definition_word38: F32::ZERO,
                definition_word4c: F32::ZERO,
            },
        }),
        source_actor: Some(1),
        allegiance_actor: Some(2),
        target_actor: Some(3),
        scale: F32::ONE,
        time: F32::ZERO,
        event_kind: 1,
    }
}
#[test]
fn target_gate_precedes_invalid_payload_validation() {
    let mut services = Services {
        absent: true,
        ..Default::default()
    };
    let mut input = hit();
    input.weapon = None;
    input.scale = F32(0x7fc00000);
    assert!(!aa_combat::hit(input, &mut services, Precision::Double53).unwrap());
    assert_eq!(*services.calls.borrow(), ["target"]);
}
#[test]
fn invalid_weapon_or_nonfinite_base_stops_before_modifier() {
    for amount in [None, Some(F32(0x7fc00000)), Some(F32(0x7f800000))] {
        let mut services = Services::default();
        let mut input = hit();
        input.weapon = amount.map(|amount| Weapon {
            amount,
            ..input.weapon.unwrap()
        });
        assert!(aa_combat::hit(input, &mut services, Precision::Double53).is_err());
        assert_eq!(*services.calls.borrow(), ["target", "id", "id"]);
        assert_eq!(services.deliveries, 0);
    }
}
#[test]
fn ignored_offense_still_calls_getter_and_stops_on_its_error() {
    let mut services = Services {
        fail_offense: true,
        ..Default::default()
    };
    let mut input = hit();
    input.weapon.as_mut().unwrap().flags = 2;
    assert!(aa_combat::hit(input, &mut services, Precision::Double53).is_err());
    assert_eq!(
        *services.calls.borrow(),
        [
            "target",
            "id",
            "id",
            "modifier",
            "type_factor",
            "offense_component",
            "offense"
        ]
    );
    assert_eq!(services.deliveries, 0);
}
#[test]
fn finite_input_overflow_is_a_checked_domain_error_before_delivery() {
    let mut services = Services::default();
    let mut input = hit();
    input.weapon.as_mut().unwrap().amount = F32(0x7f7fffff);
    input.scale = F32(0x7f7fffff);
    assert!(matches!(
        aa_combat::hit(input, &mut services, Precision::Extended64),
        Err(Error::Numeric(aa_numeric::Error::Binary32Overflow))
    ));
    assert_eq!(services.calls.borrow().last(), Some(&"defense"));
    assert_eq!(services.deliveries, 0);
}
#[test]
fn delivery_error_keeps_already_committed_external_effect() {
    let mut services = Services {
        fail_delivery: true,
        ..Default::default()
    };
    assert!(aa_combat::hit(hit(), &mut services, Precision::Single24).is_err());
    assert_eq!(services.deliveries, 1);
    assert_eq!(services.calls.borrow().last(), Some(&"deliver"));
}
