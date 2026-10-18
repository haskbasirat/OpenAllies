use aa_actors::{
    ActorId,
    economy_values::{
        self as v, ActorSnapshot, Definition, Error, GateWorld, Modifier, Result, VectorKind, World,
    },
};
use aa_body::F32;
use aa_numeric::Precision;
use std::cell::Cell;

struct Probe {
    definition: Definition<u32>,
    calls: Cell<usize>,
    backref: Option<bool>,
}
impl GateWorld for Probe {
    type Definition = u32;
    fn actor(&self, _: ActorId) -> Result<ActorSnapshot<u32>> {
        panic!("definition leaf")
    }
    fn definition(&self, _: &u32) -> Result<Definition<u32>> {
        Ok(self.definition.clone())
    }
}
impl World for Probe {
    fn resource_count(&self) -> Result<usize> {
        Ok(2)
    }
    fn actor_modifier(&self, _: ActorId) -> Result<Option<u32>> {
        panic!("definition leaf")
    }
    fn modifier(&self, _: u32) -> Result<Modifier> {
        Ok(Modifier {
            flags: 0,
            factor4c: F32(0x40000000),
            factor60: F32(0x40000000),
            production_adjustments: vec![F32::ZERO; 2],
        })
    }
    fn recruit_backref_present(&self, _: &u32) -> Result<bool> {
        self.calls.set(self.calls.get() + 1);
        self.backref
            .ok_or_else(|| Error::World("reached501730+17C".into()))
    }
    fn transport_vector(&self, _: u32, _: VectorKind) -> Result<Vec<F32>> {
        panic!("definition leaf")
    }
    fn layout_slot_count(&self, _: u32) -> Result<usize> {
        panic!("definition leaf")
    }
    fn layout_slot_definition(&self, _: u32, _: usize) -> Result<Option<u32>> {
        panic!("definition leaf")
    }
    fn member_count(&self, _: u32) -> Result<usize> {
        panic!("definition leaf")
    }
    fn member(&self, _: u32, _: usize) -> Result<Option<ActorId>> {
        panic!("definition leaf")
    }
    fn store_member_production(&mut self, _: u32, _: &[F32]) -> Result<()> {
        panic!("definition leaf")
    }
}
fn probe() -> Probe {
    Probe {
        definition: Definition {
            flags174: 1 << 9,
            class178: 0,
            word17c: 0,
            layout_kind18: None,
            supply_required: false,
            packed_definition: None,
            cost: vec![F32(0x3f800000), F32(0x40400000)],
            upkeep: vec![F32::ZERO; 2],
            production: vec![F32::ZERO; 2],
        },
        calls: Cell::new(0),
        backref: None,
    }
}
#[test]
fn backref_dependency_is_only_read_at_nonrecruitable_cost_branch() {
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        for (kind, context, flags, class) in [
            (VectorKind::Cost, None, 1 << 9, 0),
            (VectorKind::Upkeep, Some(1), 1 << 9, 0),
            (VectorKind::Production, Some(1), 1 << 9, 0),
            (VectorKind::Cost, Some(1), (1 << 9) | (1 << 15), 0),
            (VectorKind::Cost, Some(1), 1 << 9, 1),
        ] {
            let mut w = probe();
            w.definition.flags174 = flags;
            w.definition.class178 = class;
            v::definition_values(&mut Vec::new(), &1, kind, context, &w, pc).unwrap();
            assert_eq!(w.calls.get(), 0);
        }
        let w = probe();
        let mut out = vec![];
        assert!(
            v::definition_values(&mut out, &1, VectorKind::Cost, Some(1), &w, pc)
                .unwrap_err()
                .to_string()
                .contains("501730")
        );
        assert_eq!(w.calls.get(), 1);
        assert_eq!(
            out, w.definition.cost,
            "native copy precedes the reached modifier dependency"
        );
        for present in [false, true] {
            let mut w = probe();
            w.backref = Some(present);
            v::definition_values(&mut out, &1, VectorKind::Cost, Some(1), &w, pc).unwrap();
            assert_eq!(w.calls.get(), 2, "one lazy17C read per native scalar");
            assert_eq!(
                out,
                if present {
                    vec![F32(0x40000000), F32(0x40c00000)]
                } else {
                    w.definition.cost
                }
            );
        }
    }
}
