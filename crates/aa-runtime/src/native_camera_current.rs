//!Actual source/Session/World/visual-map bindings used by native camera math.
//!Frontend reads must capture reached current cells once per closed operation;
//!there is no cloned UI, World, Settings, Bank or heightfield allocation.
use crate::{
    catalog::Catalog,
    live_world_frame::World,
    load_world_globals::SessionPhase,
    native_camera::{directory::ReadOnlyInputs, target},
    view_grid_state::Globals,
    view_heightfield,
    view_startup_inputs::Inputs,
    view_visual_map,
};
use aa_numeric::{Binary, Precision};
type Result<T> = std::result::Result<T, String>;
///Concrete frontend borrows actualHost authorization +Ui.uvars through the
///existing UI RPC, caching each readonly reached cell once for the operation.
pub trait Frontend {
    fn can_process_40eaa0(&self) -> Result<bool>;
    fn current_uvar(&self, offset: u32) -> Result<u32>;
}
pub struct Current<'a, 'source, F> {
    catalog: &'a Catalog<'source>,
    world: &'a World<'static>,
    inputs: &'a Inputs,
    visual_map: &'a view_visual_map::Global,
    height: Option<&'a view_heightfield::State>,
    session: &'a dyn SessionPhase,
    frontend: &'a F,
    precision: Precision,
}
impl<'a, 'source, F: Frontend> Current<'a, 'source, F> {
    pub fn bind(
        catalog: &'a Catalog<'source>,
        world: &'a World<'static>,
        inputs: &'a Inputs,
        visual_map: &'a view_visual_map::Global,
        height: Option<&'a view_heightfield::State>,
        session: &'a dyn SessionPhase,
        frontend: &'a F,
        precision: Precision,
    ) -> Result<Self> {
        let parts = world.presentation_parts()?;
        inputs.validate(catalog, parts.definitions)?;
        Ok(Self {
            catalog,
            world,
            inputs,
            visual_map,
            height,
            session,
            frontend,
            precision,
        })
    }
    fn actual_height(&self, xy: [u32; 2], pc: Precision) -> Result<Binary> {
        let p = self.world.presentation_parts()?;
        //6FF288 loads actualGWorld+34 HEIGHT_MAP and calls550210 directly,
        //without overlay/terrain synthesis or4F0E50's binary32 temporary store.
        p.terrain
            .base
            .state
            .field
            .sample_interpolated(xy, pc)
            .map_err(|e| e.to_string())
    }
}
impl<F: Frontend> ReadOnlyInputs for Current<'_, '_, F> {
    fn metagame_40cee0(&self) -> Result<bool> {
        Ok(self.session.application_state()? == 5)
    }
    fn free_camera_without_directory_role(&self) -> Result<bool> {
        if self.frontend.can_process_40eaa0()? {
            return Ok(true);
        }
        if self
            .world
            .presentation_parts()?
            .camera_flags
            .current_cutscene_275()
            != 0
        {
            return Ok(true);
        }
        Ok(self.session.application_state()? == 3 && self.frontend.current_uvar(0x288)? != 0)
    }
    fn avar(&self, o: u32) -> Result<u32> {
        let p = self.world.presentation_parts()?;
        self.inputs.camera().word(self.catalog, p.definitions, o)
    }
    fn map_dimensions_74_78(&self) -> Result<Option<[i32; 2]>> {
        self.visual_map
            .dimensions_for_world(self.world.presentation_parts()?.revision, self.height)
    }
    fn fallback_dimensions_uvar_a8_ac(&self) -> Result<[i32; 2]> {
        Ok([
            self.frontend.current_uvar(0xa8)? as i32,
            self.frontend.current_uvar(0xac)? as i32,
        ])
    }
    fn visual_height_43d050(&self, xy: [u32; 2], flags: u32, pc: Precision) -> Result<Binary> {
        if flags != 0x222 {
            return Err("camera43D050 unsupported visual query flags".into());
        }
        self.height
            .ok_or("actual VWorldBC heightfield absent")?
            .sample_for_world(self.world.presentation_parts()?.revision, xy, pc)
    }
    fn seed_word(&self, a: u32) -> Result<u32> {
        self.inputs.seeds().bind(self.catalog)?.word(a)
    }
    fn free_yaw_uvar_b4(&self) -> Result<bool> {
        Ok(self.frontend.current_uvar(0xb4)? != 0)
    }
    fn world_editor_275(&self) -> Result<bool> {
        Ok(self
            .world
            .presentation_parts()?
            .camera_flags
            .current_cutscene_275()
            != 0)
    }
    fn session_state_e4(&self) -> Result<u32> {
        self.session.application_state()
    }
    fn map_height_550210(&self, xy: [u32; 2], pc: Precision) -> Result<Binary> {
        self.actual_height(xy, pc)
    }
}
///Native target child uses the same underlying Current loans, not another
///World/context. Ni update remains concrete on the passed camera allocation.
impl<F: Frontend> target::Services for Current<'_, '_, F> {
    fn map_dimensions_74_78(&mut self) -> Result<Option<[i32; 2]>> {
        ReadOnlyInputs::map_dimensions_74_78(self)
    }
    fn fallback_dimensions_uvar_a8_ac(&mut self) -> Result<[i32; 2]> {
        ReadOnlyInputs::fallback_dimensions_uvar_a8_ac(self)
    }
    fn margin_uvar_24(&mut self) -> Result<i32> {
        Ok(self.frontend.current_uvar(0x24)? as i32)
    }
    fn world_editor_275(&mut self) -> Result<bool> {
        ReadOnlyInputs::world_editor_275(self)
    }
    fn session_state_e4(&mut self) -> Result<u32> {
        ReadOnlyInputs::session_state_e4(self)
    }
    fn map_height_550210(&mut self, xy: [u32; 2]) -> Result<Binary> {
        self.actual_height(xy, self.precision)
    }
    fn update_camera_6fe050(
        &mut self,
        camera: &mut crate::native_camera::State,
        pc: Precision,
    ) -> Result<()> {
        crate::native_camera::ni::update_6fe050(
            camera,
            &mut crate::native_camera::ni::RequiredChildren,
            pc,
        )
    }
}
