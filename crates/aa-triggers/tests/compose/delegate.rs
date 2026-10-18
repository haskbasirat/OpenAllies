// Generated delegation (tests only): Composite -> FixtureWorld.
use aa_triggers::elements::{Bank, Services};
use aa_triggers::actions::{AbilityRequest, CameraFields, Command, Effects, SpawnRequest};
impl Services for Composite {
    fn data_by_ids(&mut self, bank: Bank, name: &[u16]) -> Result<Option<Object>> {
        Services::data_by_ids(&mut self.base, bank, name)
    }
    fn flags8(&self, object: Object) -> Result<u32> {
        Services::flags8(&self.base, object)
    }
    fn owner(&mut self, actor: Object) -> Result<Option<Object>> {
        Services::owner(&mut self.base, actor)
    }
    fn transfer_control(&mut self, actor: Object, kingdom: Object) -> Result<()> {
        Services::transfer_control(&mut self.base, actor, kingdom)
    }
    fn kingdom_default_lose(&mut self, kingdom: Object) -> Result<bool> {
        Services::kingdom_default_lose(&mut self.base, kingdom)
    }
    fn world_remove_object(&mut self, actor: Object) -> Result<()> {
        Services::world_remove_object(&mut self.base, actor)
    }
    fn actor_destroy(&mut self, actor: Object) -> Result<()> {
        Services::actor_destroy(&mut self.base, actor)
    }
    fn play_speech(&mut self, data: Option<Object>) -> Result<()> {
        Services::play_speech(&mut self.base, data)
    }
    fn flash_ui_button(&mut self, name: &[u16]) -> Result<()> {
        Services::flash_ui_button(&mut self.base, name)
    }
    fn difficulty_by_name(&mut self, name: &[u16]) -> Result<Option<i32>> {
        Services::difficulty_by_name(&mut self.base, name)
    }
    fn map_difficulty(&self) -> Result<i32> {
        Services::map_difficulty(&self.base)
    }
    fn is_actor_data(&mut self, data: Object) -> Result<bool> {
        Services::is_actor_data(&mut self.base, data)
    }
    fn actor_virtual_158(&mut self, actor: Object) -> Result<bool> {
        Services::actor_virtual_158(&mut self.base, actor)
    }
    fn actor_virtual_15c(&mut self, actor: Object) -> Result<Option<Object>> {
        Services::actor_virtual_15c(&mut self.base, actor)
    }
    fn organization(&self, actor: Object) -> Result<Option<Object>> {
        Services::organization(&self.base, actor)
    }
    fn organization_has_member(&mut self, organization: Object, id: u32) -> Result<bool> {
        Services::organization_has_member(&mut self.base, organization, id)
    }
    fn definition(&self, actor: Object) -> Result<Option<Object>> {
        Services::definition(&self.base, actor)
    }
    fn state_bits(&self, actor: Object) -> Result<u64> {
        Services::state_bits(&self.base, actor)
    }
    fn body(&self, actor: Object) -> Result<Option<(u32, u32)>> {
        Services::body(&self.base, actor)
    }
    fn area_first_actor(&mut self, area: Option<Object>) -> Result<Option<Object>> {
        Services::area_first_actor(&mut self.base, area)
    }
    fn actor_inside_area(&mut self, actor: Option<Object>, area: Option<Object>) -> Result<bool> {
        Services::actor_inside_area(&mut self.base, actor, area)
    }
    fn kingdom_player(&mut self, kingdom: Object) -> Result<Option<Object>> {
        Services::kingdom_player(&mut self.base, kingdom)
    }
    fn player_selection(&self, player: Object) -> Result<Option<Object>> {
        Services::player_selection(&self.base, player)
    }
    fn selection_actors(&self, selection: Object) -> Result<Vec<Option<Object>>> {
        Services::selection_actors(&self.base, selection)
    }
    fn kingdom_has_property(&mut self, kingdom: Object, property: Object) -> Result<u32> {
        Services::kingdom_has_property(&mut self.base, kingdom, property)
    }
    fn actor_seen_by(&mut self, actor: Object, kingdom: Object, explored: bool) -> Result<u32> {
        Services::actor_seen_by(&mut self.base, actor, kingdom, explored)
    }
}
impl Effects for Composite {
    fn warn_invalid_ids(&self) -> Result<bool> {
        Effects::warn_invalid_ids(&self.base)
    }
    fn camera_debug_log(&self) -> Result<bool> {
        Effects::camera_debug_log(&self.base)
    }
    fn world_time_bits(&self) -> Result<u32> {
        Effects::world_time_bits(&self.base)
    }
    fn object_name(&self, object: Object) -> Result<Vec<u16>> {
        Effects::object_name(&self.base, object)
    }
    fn camera_fields(&self, camera: Object) -> Result<CameraFields> {
        Effects::camera_fields(&self.base, camera)
    }
    fn camera_pitch(&mut self, camera: Object) -> Result<u32> {
        Effects::camera_pitch(&mut self.base, camera)
    }
    fn camera_yaw(&mut self, camera: Object) -> Result<u32> {
        Effects::camera_yaw(&mut self.base, camera)
    }
    fn real_time_ms(&mut self) -> Result<u32> {
        Effects::real_time_ms(&mut self.base)
    }
    fn camera_var(&self, offset: u32) -> Result<u32> {
        Effects::camera_var(&self.base, offset)
    }
    fn ui_script_command(&mut self, command: &[u16]) -> Result<()> {
        Effects::ui_script_command(&mut self.base, command)
    }
    fn world_set_cutscene_mode(&mut self, enabled: bool) -> Result<()> {
        Effects::world_set_cutscene_mode(&mut self.base, enabled)
    }
    fn camera_6fe710(&mut self) -> Result<()> {
        Effects::camera_6fe710(&mut self.base)
    }
    fn camera_700b10(&mut self) -> Result<u32> {
        Effects::camera_700b10(&mut self.base)
    }
    fn camera_700a90(&mut self, value: u32) -> Result<()> {
        Effects::camera_700a90(&mut self.base, value)
    }
    fn kingdom_add_property(&mut self, kingdom: Object, property: Object, kind: u32, a: u32, notify: u32) -> Result<()> {
        Effects::kingdom_add_property(&mut self.base, kingdom, property, kind, a, notify)
    }
    fn kingdom_remove_property(&mut self, kingdom: Object, property: Object, a: u32) -> Result<()> {
        Effects::kingdom_remove_property(&mut self.base, kingdom, property, a)
    }
    fn actor_remove_property(&mut self, actor: Object, property: Object, a: u32) -> Result<()> {
        Effects::actor_remove_property(&mut self.base, actor, property, a)
    }
    fn world_kingdom_slots(&self) -> Result<Vec<Option<Object>>> {
        Effects::world_kingdom_slots(&self.base)
    }
    fn ui_display_banner(&mut self, text: &[u16], duration: u32) -> Result<()> {
        Effects::ui_display_banner(&mut self.base, text, duration)
    }
    fn sun_colors(&self, sun: Object) -> Result<[[u32; 3]; 3]> {
        Effects::sun_colors(&self.base, sun)
    }
    fn view_set_sun_colors(&mut self, colors: &[[u32; 3]; 3], duration: u32) -> Result<()> {
        Effects::view_set_sun_colors(&mut self.base, colors, duration)
    }
    fn resource_flags18(&self) -> Result<Vec<u32>> {
        Effects::resource_flags18(&self.base)
    }
    fn kingdom_add_resource(&mut self, kingdom: Object, index: u32, amount: u32) -> Result<()> {
        Effects::kingdom_add_resource(&mut self.base, kingdom, index, amount)
    }
    fn world_show_counter(&mut self, start: u32, duration: u32) -> Result<()> {
        Effects::world_show_counter(&mut self.base, start, duration)
    }
    fn world_hide_counter(&mut self) -> Result<()> {
        Effects::world_hide_counter(&mut self.base)
    }
    fn set_weather(&mut self, weather: Option<Object>) -> Result<()> {
        Effects::set_weather(&mut self.base, weather)
    }
    fn play_sound(&mut self, data: Option<Object>, location: Option<[u32; 3]>) -> Result<()> {
        Effects::play_sound(&mut self.base, data, location)
    }
    fn ui_flash_actor(&mut self, actor: Object) -> Result<()> {
        Effects::ui_flash_actor(&mut self.base, actor)
    }
    fn ui_root(&self) -> Result<Option<Object>> {
        Effects::ui_root(&self.base)
    }
    fn ui_find_widget(&mut self, root: Object, name: &[u16]) -> Result<Option<Object>> {
        Effects::ui_find_widget(&mut self.base, root, name)
    }
    fn widget_name(&self, widget: Object) -> Result<Vec<u16>> {
        Effects::widget_name(&self.base, widget)
    }
    fn world_script_listen(&mut self, name: &[u16]) -> Result<()> {
        Effects::world_script_listen(&mut self.base, name)
    }
    fn world_script_unlisten(&mut self, name: &[u16]) -> Result<bool> {
        Effects::world_script_unlisten(&mut self.base, name)
    }
    fn actor_gain_experience(&mut self, actor: Object, amount: u32) -> Result<()> {
        Effects::actor_gain_experience(&mut self.base, actor, amount)
    }
    fn is_vfx(&mut self, data: Object) -> Result<bool> {
        Effects::is_vfx(&mut self.base, data)
    }
    fn default_vfx_orientation(&self) -> Result<[u32; 2]> {
        Effects::default_vfx_orientation(&self.base)
    }
    fn rotate_2d(&mut self, vector: [u32; 2], angle: u32) -> Result<[u32; 2]> {
        Effects::rotate_2d(&mut self.base, vector, angle)
    }
    fn spawn_vfx(&mut self, vfx: Object, position: [u32; 3], orientation: [u32; 3]) -> Result<()> {
        Effects::spawn_vfx(&mut self.base, vfx, position, orientation)
    }
    fn chat_message_all(&mut self, text: &[u16]) -> Result<u32> {
        Effects::chat_message_all(&mut self.base, text)
    }
    fn chat_message_kingdom(&mut self, kingdom: Object, text: &[u16]) -> Result<()> {
        Effects::chat_message_kingdom(&mut self.base, kingdom, text)
    }
    fn animation_set(&self, actor: Object) -> Result<Option<Object>> {
        Effects::animation_set(&self.base, actor)
    }
    fn animation_by_name(&mut self, set: Object, name: &[u16]) -> Result<Option<Object>> {
        Effects::animation_by_name(&mut self.base, set, name)
    }
    fn actor_play_animation(&mut self, actor: Object, animation: Object) -> Result<bool> {
        Effects::actor_play_animation(&mut self.base, actor, animation)
    }
    fn actor_4d4c60(&mut self, actor: Object, flag: u8) -> Result<()> {
        Effects::actor_4d4c60(&mut self.base, actor, flag)
    }
    fn process_command(&mut self, actor: Object, command: &mut Command) -> Result<bool> {
        Effects::process_command(&mut self.base, actor, command)
    }
    fn release_command_shared(&mut self, shared: u64) -> Result<()> {
        Effects::release_command_shared(&mut self.base, shared)
    }
    fn world_slots_148(&self) -> Result<Vec<Option<Object>>> {
        Effects::world_slots_148(&self.base)
    }
    fn member_head(&self, kingdom: Object) -> Result<Option<u64>> {
        Effects::member_head(&self.base, kingdom)
    }
    fn member_value(&self, node: u64) -> Result<Option<Object>> {
        Effects::member_value(&self.base, node)
    }
    fn member_next(&self, node: u64) -> Result<Option<u64>> {
        Effects::member_next(&self.base, node)
    }
    fn actor_80_ready(&self, actor: Object) -> Result<bool> {
        Effects::actor_80_ready(&self.base, actor)
    }
    fn video_path(&mut self, name: &[u16]) -> Result<Vec<u16>> {
        Effects::video_path(&mut self.base, name)
    }
    fn game_var_string(&mut self, name: &[u16]) -> Result<Vec<u16>> {
        Effects::game_var_string(&mut self.base, name)
    }
    fn command_definition(&self, ordinal: u32) -> Result<Option<Object>> {
        Effects::command_definition(&self.base, ordinal)
    }
    fn move_factor(&self, actor: Object) -> Result<Option<u32>> {
        Effects::move_factor(&self.base, actor)
    }
    fn find_placement(&mut self, position: &mut [u32; 2], facing: &mut [u32; 2], data: Object, radius: u32, mode: u32) -> Result<bool> {
        Effects::find_placement(&mut self.base, position, facing, data, radius, mode)
    }
    fn data_word(&self, object: Object, offset: u32) -> Result<u32> {
        Effects::data_word(&self.base, object, offset)
    }
    fn data_ref(&self, object: Object, offset: u32) -> Result<Option<Object>> {
        Effects::data_ref(&self.base, object, offset)
    }
    fn transport_path(&mut self, from: [u32; 2], to: [u32; 2]) -> Result<[u32; 2]> {
        Effects::transport_path(&mut self.base, from, to)
    }
    fn transport_path_valid(&mut self, point: [u32; 2], to: [u32; 2]) -> Result<bool> {
        Effects::transport_path_valid(&mut self.base, point, to)
    }
    fn create_transport(&mut self, request: &SpawnRequest) -> Result<Option<Object>> {
        Effects::create_transport(&mut self.base, request)
    }
    fn or_flags8(&mut self, object: Object, bits: u32) -> Result<()> {
        Effects::or_flags8(&mut self.base, object, bits)
    }
    fn create_group(&mut self, word_2f0: u32) -> Result<Option<Object>> {
        Effects::create_group(&mut self.base, word_2f0)
    }
    fn flare_ready(&mut self, config: Object, on_actor: bool) -> Result<bool> {
        Effects::flare_ready(&mut self.base, config, on_actor)
    }
    fn flare_var(&mut self, on_actor: bool) -> Result<u32> {
        Effects::flare_var(&mut self.base, on_actor)
    }
    fn create_flare(&mut self, request: &SpawnRequest, kingdom: Object, actor: Option<Object>, duration: u32, variable: u32) -> Result<()> {
        Effects::create_flare(&mut self.base, request, kingdom, actor, duration, variable)
    }
    fn ability_user_init(&mut self, ability: Object) -> Result<()> {
        Effects::ability_user_init(&mut self.base, ability)
    }
    fn ability_use(&mut self, request: &AbilityRequest) -> Result<()> {
        Effects::ability_use(&mut self.base, request)
    }
    fn ability_user_release(&mut self) -> Result<()> {
        Effects::ability_user_release(&mut self.base)
    }
    fn transport_load(&mut self, component: Object, cargo: Object, name: &[u16], group: Option<Object>, position: [u32; 2], destination: [u32; 2]) -> Result<()> {
        Effects::transport_load(&mut self.base, component, cargo, name, group, position, destination)
    }
    fn data_flags174(&self, data: Object) -> Result<u32> {
        Effects::data_flags174(&self.base, data)
    }
    fn create_actor(&mut self, request: &SpawnRequest, alternate: bool) -> Result<Option<Object>> {
        Effects::create_actor(&mut self.base, request, alternate)
    }
}
impl aa_triggers::extended::Effects for Composite {
    fn view_adjust_sun_dimmer(&mut self, a: u32, b: u32) -> Result<()> {
        aa_triggers::extended::Effects::view_adjust_sun_dimmer(&mut self.base, a, b)
    }
    fn sai_change_profile(&mut self, kingdom: Object, sai: Object) -> Result<()> {
        aa_triggers::extended::Effects::sai_change_profile(&mut self.base, kingdom, sai)
    }
    fn play_music(&mut self, data: Option<Object>) -> Result<()> {
        aa_triggers::extended::Effects::play_music(&mut self.base, data)
    }
    fn ui_vfx_on_actor(&mut self, vfx: Object, actor: Object) -> Result<()> {
        aa_triggers::extended::Effects::ui_vfx_on_actor(&mut self.base, vfx, actor)
    }
    fn fidget_var_flags(&self) -> Result<u32> {
        aa_triggers::extended::Effects::fidget_var_flags(&self.base)
    }
    fn var_permission(&mut self, kind: u32) -> Result<bool> {
        aa_triggers::extended::Effects::var_permission(&mut self.base, kind)
    }
    fn set_fidget_var(&mut self, value: u8) -> Result<()> {
        aa_triggers::extended::Effects::set_fidget_var(&mut self.base, value)
    }
    fn set_game_speed(&mut self, bits: u32) -> Result<()> {
        aa_triggers::extended::Effects::set_game_speed(&mut self.base, bits)
    }
    fn kingdom_hero_entry(&mut self, kingdom: Object, data: Object) -> Result<Option<Object>> {
        aa_triggers::extended::Effects::kingdom_hero_entry(&mut self.base, kingdom, data)
    }
    fn set_hero_entry_18(&mut self, entry: Object, bits: u32) -> Result<()> {
        aa_triggers::extended::Effects::set_hero_entry_18(&mut self.base, entry, bits)
    }
    fn organization_modify_morale(&mut self, organization: Object, value: u32, mode: u32) -> Result<()> {
        aa_triggers::extended::Effects::organization_modify_morale(&mut self.base, organization, value, mode)
    }
    fn actor_display_name(&mut self, actor: Object) -> Result<Vec<u16>> {
        aa_triggers::extended::Effects::actor_display_name(&mut self.base, actor)
    }
    fn area_actor_ids(&self, area: Object) -> Result<Vec<u32>> {
        aa_triggers::extended::Effects::area_actor_ids(&self.base, area)
    }
    fn actor_4d12c0(&mut self, actor: Object) -> Result<bool> {
        aa_triggers::extended::Effects::actor_4d12c0(&mut self.base, actor)
    }
    fn world_4f6760(&mut self, actor: Object, flag: u32) -> Result<()> {
        aa_triggers::extended::Effects::world_4f6760(&mut self.base, actor, flag)
    }
    fn actor_teleport(&mut self, actor: Object, position: [u32; 3]) -> Result<()> {
        aa_triggers::extended::Effects::actor_teleport(&mut self.base, actor, position)
    }
    fn member_teleport(&mut self, member: Object, position: [u32; 2]) -> Result<()> {
        aa_triggers::extended::Effects::member_teleport(&mut self.base, member, position)
    }
    fn store_flags8(&mut self, object: Object, value: u32) -> Result<()> {
        aa_triggers::extended::Effects::store_flags8(&mut self.base, object, value)
    }
    fn world_4f03c0(&mut self) -> Result<bool> {
        aa_triggers::extended::Effects::world_4f03c0(&mut self.base)
    }
    fn players_76ac60(&mut self) -> Result<Option<Object>> {
        aa_triggers::extended::Effects::players_76ac60(&mut self.base)
    }
    fn player_765e00(&mut self, player: Option<Object>) -> Result<Option<Object>> {
        aa_triggers::extended::Effects::player_765e00(&mut self.base, player)
    }
    fn audio_lookup(&mut self, name: &[u16]) -> Result<Option<Object>> {
        aa_triggers::extended::Effects::audio_lookup(&mut self.base, name)
    }
    fn new_hero_entry(&mut self, data: Object) -> Result<Option<Object>> {
        aa_triggers::extended::Effects::new_hero_entry(&mut self.base, data)
    }
    fn kingdom_add_hero(&mut self, kingdom: Object, entry: Option<Object>) -> Result<()> {
        aa_triggers::extended::Effects::kingdom_add_hero(&mut self.base, kingdom, entry)
    }
    fn kingdom_remove_hero(&mut self, kingdom: Object, entry: Option<Object>) -> Result<()> {
        aa_triggers::extended::Effects::kingdom_remove_hero(&mut self.base, kingdom, entry)
    }
    fn fog_restore_tiles(&mut self, area: Object, query: [u32; 4]) -> Result<()> {
        aa_triggers::extended::Effects::fog_restore_tiles(&mut self.base, area, query)
    }
    fn actor_data_bank(&self) -> Result<Vec<Option<Object>>> {
        aa_triggers::extended::Effects::actor_data_bank(&self.base)
    }
    fn world_rng(&mut self) -> Result<Object> {
        aa_triggers::extended::Effects::world_rng(&mut self.base)
    }
    fn random(&mut self, rng: Object) -> Result<u32> {
        aa_triggers::extended::Effects::random(&mut self.base, rng)
    }
    fn kingdom_hero_count(&self, kingdom: Object) -> Result<u32> {
        aa_triggers::extended::Effects::kingdom_hero_count(&self.base, kingdom)
    }
    fn kingdom_hero_at(&mut self, kingdom: Object, index: u32) -> Result<Option<Object>> {
        aa_triggers::extended::Effects::kingdom_hero_at(&mut self.base, kingdom, index)
    }
}
