pub use aa_actors::{actor_creation,components,ActorId};
use aa_actors::actor_creation::{Error,Result};
use aa_actors::components::StateBits;
use aa_numeric::{Binary,Precision};
use aa_spatial::{collision::{Grid,Shape},geometry::{Placement,TileRect}};
use aa_zones::set::Entry;
fn error(e:impl std::fmt::Display)->Error{Error::World(e.to_string())}
#[path=r"E:\Games\Axis and Alies\rust-rebuild\crates\aa-actors\src\actor_registration.rs"]pub mod actor_registration;
pub mod load_zone_entry {use super::*;
pub(crate) fn visit_indices(
    grid: &Grid,
    extents: [u32; 2],
    point: &Shape,
    placement: &Placement,
    flags: u32,
    cached: [i32; 2],
    p: Precision,
    mut visit: impl FnMut(usize) -> Result<()>,
) -> Result<()> {
    if point.kind != 0 || grid.dimensions.iter().any(|&n| n <= 0) {
        return Err(error(
            "registration requires proved point and positive map dimensions",
        ));
    }
    let multi = aa_spatial::geometry::marks_blocker(
        flags,
        placement.shape.radius_bits,
        grid.cell_size_bits,
        p,
    )
    .map_err(error)?;
    let rect = if multi {
        aa_spatial::geometry::tile_rectangle(grid, placement, p).map_err(error)?
    } else {
        let cell = [
            cached[0].clamp(0, grid.dimensions[0] - 1),
            cached[1].clamp(0, grid.dimensions[1] - 1),
        ];
        TileRect {
            min: cell,
            max: cell,
        }
    };
    let mut center = rect.min;
    let inside = placement
        .position
        .into_iter()
        .zip(extents)
        .all(|(xy, max)| f32::from_bits(xy) >= 0.0 && f32::from_bits(xy) < f32::from_bits(max));
    if inside {
        for (axis, c) in center.iter_mut().enumerate() {
            *c = Binary::from_f32_bits(placement.position[axis])?.truncate_i32()?
                >> (grid.coordinate_shift & 31);
        }
    }
    let half = Binary::from_f32_bits(grid.cell_size_bits)?
        .mul(&Binary::from_f32_bits(0x3f000000)?, p)?
        .store_f32()?;
    for y in rect.min[1]..=rect.max[1] {
        for x in rect.min[0]..=rect.max[0] {
            let cell = [x, y];
            let include = if cell == center {
                true
            } else {
                let mut position = [0; 2];
                for (axis, bits) in position.iter_mut().enumerate() {
                    *bits = Binary::from_i32(cell[axis].wrapping_shl(grid.coordinate_shift & 31))
                        .add(&Binary::from_f32_bits(half)?, p)?
                        .store_f32()?;
                }
                aa_spatial::shape_overlap::overlaps(
                    placement,
                    &Placement {
                        shape: point.clone(),
                        facing: [0, 0xbf800000],
                        position,
                    },
                    p,
                )
                .map_err(error)?
            };
            if include {
                visit(y as usize * grid.dimensions[0] as usize + x as usize)?;
            }
        }
    }
    Ok(())
}

pub(crate) trait EnteredFields{
    fn city_entered(&mut self,owner:aa_actors::ActorId)->Result<()>;
    fn detection_present(&mut self)->Result<bool>;
}
pub(crate) fn entered_fields(entry:Entry,fields:&mut impl EnteredFields)->Result<()>{
    //515620 reaches these zone-slot getters in precisely this order. Their
    // owning zone/component effects must bind before a positive call succeeds.
    for (bit, name) in [
        (0x20, "registration zone slot5/guard owner"),
        (0x10, "registration zone slot4/control owner"),
        (8, "registration zone slot3 component owner"),
    ] {
        if entry.flags & bit != 0 {
            return Err(Error::Unsupported(name));
        }
    }
    if entry.flags & 0x100 != 0 {
        fields.city_entered(aa_actors::ActorId::from_raw(entry.actor))?;
    }
    if entry.flags & 4 != 0 && fields.detection_present()? {
        return Err(Error::Unsupported(
            "registration actual marker/discovery owner",
        ));
    }
    Ok(())
}
#[path=r"E:\Games\Axis and Alies\rust-rebuild\crates\aa-runtime\src\load_zone_entry_tests.rs"]mod iterator_tests;
}
mod original_registration_tests{use super::*;
#[test]
fn native_state_count_order_and_lazy_rereads_match() {
    use serde_json::{Value, json};
    struct World {
        input: Value,
        bits: u64,
        count: i32,
        trace: Vec<Value>,
    }
    impl actor_registration::World for World {
        fn registered_state_count(&mut self) -> Result<i32> {
            Ok(self.count)
        }
        fn state_bits(&mut self) -> Result<StateBits> {
            Ok(StateBits {
                low: self.bits as u32,
                high: (self.bits >> 32) as u32,
            })
        }
        fn state_reenter(&mut self, index: u32) -> Result<()> {
            self.trace.push(json!(["enter", index, 0]));
            if self.input["stop"].as_u64() == Some(index as u64) {
                return Err(Error::Unsupported("fixture state effect"));
            }
            if let Some(mutations) = self.input["mutations"].as_array() {
                for m in mutations {
                    if m["index"].as_u64() == Some(index as u64) {
                        self.bits = m["bits"].as_u64().unwrap();
                        if let Some(c) = m["count"].as_i64() {
                            self.count = c as i32;
                        }
                    }
                }
            }
            Ok(())
        }
        fn grid_register(&mut self) -> Result<()> {
            self.trace.push(json!(["grid"]));
            if self.input["stop_grid"] == true {
                return Err(Error::Unsupported("fixture grid effect"));
            }
            Ok(())
        }
    }
    let proof: Value = serde_json::from_str(include_str!(
        r"E:\Games\Axis and Alies\rust-rebuild\research\skirmish-20261016-load-actor-registration\native-registration.json"
    ))
    .unwrap();
    for table in proof["table_runs"].as_array().unwrap() {
        assert_eq!(
            table["records"].as_array().unwrap().len(),
            aa_actors::state::DESCRIPTORS.len()
        );
        for (row, d) in table["records"]
            .as_array()
            .unwrap()
            .iter()
            .zip(aa_actors::state::DESCRIPTORS)
        {
            assert_eq!(row["name"], d.name);
            assert_eq!(row["index"], d.index);
            assert_eq!(
                row["group"].as_u64().unwrap() as u32,
                d.group.map(u32::from).unwrap_or(u32::MAX)
            );
            assert_eq!(row["exclusive"], d.exclusive);
        }
    }
    for row in proof["cases"].as_array().unwrap() {
        let input = row["input"].clone();
        let mut w = World {
            bits: input["bits"].as_u64().unwrap(),
            count: input["count"].as_i64().unwrap() as i32,
            input,
            trace: vec![],
        };
        let result = actor_registration::register(&mut w);
        assert_eq!(
            result.is_ok(),
            row["output"]["result"] == "complete",
            "{row}"
        );
        assert_eq!(json!(w.trace), row["output"]["trace"], "{row}");
        assert_eq!(json!(w.bits), row["output"]["bits"], "{row}");
        assert_eq!(
            json!(w.count as u32),
            row["output"]["stored_count"],
            "{row}"
        );
    }
}
}
#[path=r"E:\Games\Axis and Alies\rust-rebuild\crates\aa-runtime\src\stored_actor_registration_tests.rs"]mod new_entry_tests;
