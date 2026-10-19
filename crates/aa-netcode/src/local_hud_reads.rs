//!Read-only reads of the SAME activated Session for the in-game interface.
//!Every value is read from its retained owner at the call; nothing here is
//!a snapshot owner, a readiness capability or a writer.
//!
//!76AC70 (Session method, ecx = B36550): Session+E4 == 3 -> NULL; else
//![B36558]+8 (Node+8 local player); NULL -> NULL; else tail-jumps to
//!765E00, which returns NULL while GWorld B36534 is NULL and player+28
//!otherwise. An activated session always owns its live World, so the GWorld
//!test is satisfied by construction of this owner.
use super::*;

impl world_ready::ActualActivatedSession {
    ///76AC70 / 765E00 on the retained Session, Node+8 and Player+28. The
    ///player+28 binding is the BeginPlaying 765E10 write; before it there is
    ///no proven native value, so that state is an error, never a NULL.
    pub fn local_player_kingdom_76ac70(
        &self,
    ) -> Result<Option<aa_runtime::load_tail_world::KingdomId>> {
        let host = self.host();
        if host.session.state == 3 {
            return Ok(None);
        }
        let Some(local) = host.local_player else {
            return Ok(None);
        };
        let player = host
            .session
            .players
            .iter()
            .find(|p| p.core.id == local)
            .ok_or_else(|| missing("76AC70 actual Node+8 player is absent from its Session"))?;
        if !player.core.binding_complete || player.view.is_none() {
            return Err(missing(
                "765E00 player+28 has no 765E10 BeginPlaying binding yet",
            ));
        }
        Ok(player.core.field_28)
    }
    ///Session+E4 (40CEC0 and 76AC70 compare it with 3).
    pub fn session_state_e4(&self) -> u32 {
        self.host().session.state
    }
    ///Session+F4.
    pub fn session_multiplayer_f4(&self) -> bool {
        self.host().session.multiplayer
    }
    ///KKC_Sim (B36554) +28 paused byte.
    pub fn sim_paused_28(&self) -> u8 {
        self.host().sim().paused_byte()
    }
    ///Session+F8 (76AC90).
    pub fn session_phase_f8(&self) -> u32 {
        self.host().session.phase
    }
    ///Session+58 (Launch+4) launch mode, the 76C700 Creator selector.
    pub fn session_launch_mode_58(&self) -> u32 {
        self.host().session.launch_mode
    }
    ///Session+148 status text (76E4C0 formats it with "%s").
    pub fn session_status_148(&self) -> Vec<u16> {
        self.host().status_text_148().to_vec()
    }
    ///5C0E10 -> 76AC60: [B36558]+8 local player is non-null.
    pub fn local_player_present_76ac60(&self) -> bool {
        self.host().local_player.is_some()
    }
    ///Session+68 GameSettings +24 name and +2C description display text.
    pub fn game_settings_68_texts(&self) -> Result<(Vec<u16>, Vec<u16>)> {
        let settings = self.host().current_game_settings_68()?;
        let name = settings.name_pair().map_err(missing)?.text().to_vec();
        let description = settings.description_pair().map_err(missing)?.text().to_vec();
        Ok((name, description))
    }
    ///76C700(player+24) then 50DC00 on the Creator the Session+58 launch
    ///mode selects (0 -> +74 Random, 2 -> +70 Load, 5 -> +7C Metagame, other
    ///-> +6C Blank; Launch ctor 76E540 order): None when the local player
    ///is NULL or no KingdomParameter id matches (first case-sensitive
    ///NUL-terminated _wcscmp match), else that parameter's nation name.
    pub fn local_player_creator_nation_50dc00(&self) -> Result<Option<Vec<u16>>> {
        let host = self.host();
        let Some(local) = host.local_player else {
            return Ok(None);
        };
        let player = host
            .session
            .players
            .iter()
            .find(|p| p.core.id == local)
            .ok_or_else(|| missing("actual Node+8 player is absent from its Session"))?;
        let creator = match host.session.launch_mode {
            0 => host.other_launch_creators()?.random().creator(),
            2 => host.selected_load_creator()?,
            5 => host.other_launch_creators()?.metagame().creator(),
            _ => host.other_launch_creators()?.blank().creator(),
        };
        let prefix = |v: &[u16]| -> Vec<u16> { v.iter().copied().take_while(|&c| c != 0).collect() };
        let name = prefix(&player.core.kingdom);
        Ok(creator
            .parameters()
            .kingdoms
            .iter()
            .find(|k| prefix(&k.id) == name)
            .map(|k| prefix(&k.nation)))
    }
    ///Whether the local player's Player+50 selection List is empty. None
    ///when Node+8 is NULL; an absent Player+50 owner is an error.
    pub fn local_player_selection_empty(&self) -> Result<Option<bool>> {
        let host = self.host();
        let Some(local) = host.local_player else {
            return Ok(None);
        };
        let player = host
            .session
            .players
            .iter()
            .find(|p| p.core.id == local)
            .ok_or_else(|| missing("actual Node+8 player is absent from its Session"))?;
        let view = player
            .view
            .as_ref()
            .ok_or_else(|| missing("local Player+50 VWorldState is not constructed"))?;
        Ok(Some(view.selected.is_empty()))
    }
}
