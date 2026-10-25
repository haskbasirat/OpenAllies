# Open items after Development 21

What each owner left open when the 2026-10-25 round stopped. Addresses are AA.exe.

## HUD (aa-bevy ui/, input.rs, aa-ui)

GroupDisplay for multiple selections is the next visible step. Our panel shows only the head object.

- **Where it lives:** ObjectDisplay 0x62CBA0 owns four sub-displays:
  - SimpleActorDisplay 0x62CA40
  - CompanyDisplay 0x62A5D0
  - SettlementDisplay 0x62A740
  - GroupDisplay 0x62CAF0 (constructor 0x62C3C0, vtable 0xA5CEB8)
- **Template:** `UI/Game/object_display.tgi:ObjectDisplay/GroupDisplay`, from UnitDisplay. It holds:
  - Name and Owner labels (UIGroupDisplayUseNameLabel = true in AVars_AA);
  - a FormationButtons panel ("GroupFormationButtons");
  - pools of six SimpleActorMini (ids 0-5), six CompanyMini (6-11) and six SettlementMini (12-17), each renamed "%s_%i" (0x5F0F40).
- **SetSelection** (vtable +0xE0, 0x6296C0) handles up to six objects in list order:
  - Each takes a mini from its pool (0x625670): an organization gets a CompanyMini, a settlement goes through 0x7A32B0, anything else gets a SimpleActorMini.
  - Then SetActor (+0xEC) is called and the mini is added as a child.
  - Position: x = i * floor((width - formation panel width) / 6), y = mini_display_offset (6).
  - Each controllable organization with set_formation (command 0x21) merges into up to three "FormationChange <id> " buttons (0x628490).
  - The last actor processed feeds the Name and Owner labels (0x626310).
- **The minis** derive from MiniDisplay 0x62AD70, which builds PortraitButton (Portrait / ModelPortrait / Icon) and DescriptionLabel (0x628940).
  - SimpleActorMini 0x62AE90 adds a Health bar (0x628D40; refresh 0x628DD0).
  - CompanyMini 0x62FF20 adds:
    - PrimaryHealth (only with a leader);
    - Morale and AttackEfficiency, each with a target value;
    - HealthBars, two per row (0x62EB90);
    - the banner model (0x62F1B0);
    - RetreatRoutButton (0x62D6B0; refresh 0x62E9C0).
- **Clicking a mini:** "DisplayObjectSelect <gtid> 0" (0x6277E0, handler 0x610680), and "CenterOnActor <gtid>" (0x627A80).
- **Candidate highlight while dragging:** 0x437DC0 -> 0x4371A0 rebuilds the box candidates each frame. Candidates get view flag 0x8000.

Then:

- the Unpack impact preview (building minus truck, 0x602820);
- ghosts for organization layouts and city variants (component bits 7 / 29 in 0x688DC0);
- the hot construct command on unfinished structures (0x4DA570 type 4).

## Aircraft (core: combat/air*, minds.rs; movement: aa-movement aircraft*)

Air orders are still refused. B1 (the hosted aircraft Mind) is done. B2 (attack) is written behind the temporary `air-b2` feature in crates/aa-play/Cargo.toml.

1. **Wire the flight ports into aa-play** (movement owner). The ports in aa-movement are exact and fixture-proven: fly_to 0x560DB0, replace_path 0x560ED0, fly_formation 0x560F10, join/leave_formation 0x55EB30 / 0x55EDC0. aa-play still has no-op stubs. Use an adapter over `books.actor(a).aircraft.flight` and `books.groups`. Then:
   - put MoveTick 0x5612F0 on the queue-0 Mover;
   - start that Mover at actor init: 0x4D7287 calls the aircraft's vf+8, 0x55E600, which calls 0x576B20. This runs before the Mind's InitExisting 0x517C60;
   - on death, run 0x55ED00 (vf+0x34): leader handover, path clear, then the fall;
   - at map edges, call 0x4D04B0 / 0x4D7EA0 from the pose commit, which also set and clear `AircraftBooks::hidden`;
   - at the path end, call `combat::air::path_finished` (0x561747).
2. **Write the fire service** 0x57E030: mana debit, recharge, the bomb projectile.
3. **Combat** (combat.rs owner): skip actors where `combat::air::is_aircraft` in lone_think and fire.
4. **Finish B2:**
   - turn `air-b2` on and remove the feature;
   - port the dog-fight branches of 0x51A920;
   - do a real run: a bomber takes off, bombs, returns and lands. Only then stop refusing AirAttack / AirRecon.
5. **Readings to check against the original:**
   - slot health at InitCreate ignores creation mode 4;
   - recon on an actor uses its position;
   - join's object +0x10 radius is taken from the blocking footprint;
   - Mind+0x1D0 is read as projectile_type != ANTIGRAV;
   - 0x4C8F20 is reduced to filter / range / mana / recharge;
   - actor vt+0x104 and 0x45D6F0(0) are taken as speed_max.
6. **FPS:** repeat the airmind FPS run on a quiet machine. It showed about 7 FPS while 18 builds were running.

Movement also left: OrgMind on queue 4, and an idle re-measure of the planning worst tick.

## Effects and sound (effects owner: aa-bevy effects/, audio/, battle_view.rs)

Corpses now sink as in the original (0x459E10 -> 0x457DF0, 0x5617E0). The rest of the audit comparison is in qa-output/fx-audit/original/.

1. **Actor-state effect lists are never run.** These are the root-level WhenEnter / WhenLeave lists keyed by `actor_state` (FXC field 0x42C1B0, event names 0x4985E0, state names 0x477FC0, states set by 0x4D64F0). That leaves out:
   - pack and unpack sounds and the `building_pack` dust;
   - construction effects and the construction-finished sounds;
   - the motor pool sounds;
   - transport unloading;
   - plane crash on leaving `falling`.
2. **Tint modifiers** (`under_siege_vfx` red, `out_of_supply_vfx` grey-green) are not drawn. The timing is in 0x449920, and the colour comes from lerp(white, colour, pct) via 0x717E40. "Under siege" also needs the sim state 0x4D6660 / 0x586370, which is core's.
3. Grenade trails and contrails (VFXExtrusion) are not drawn.
4. **Not yet compared with the original:**
   - explosion size and duration, frame by frame;
   - acknowledgement voices;
   - ambient sounds;
   - weapon sound falloff with distance.

## Random maps (parity owner: aa-play mapgen*, aa-parity)

Film 2 matches through terrain, noise, required objects and the village balanced group (250,128 draws in total; 57/57 log lines).

Next:

- water doodads 0x555F20 (first mismatch at draw 193,874);
- auto doodads 0x552BF0;
- cleanup 0x592170;
- heroes;
- heights: 29,848 of 37,249 base heights differ, because brush and relax are approximate;
- water areas;
- the settlement-zone branch of 0x4F3640;
- CreateExtraMarkers 0x594260;
- plain groups 0x594590.

Then continue film parity: the Berlin replay is at 28/266 checkpoints, and its first difference is at tick 493.

Debug environment variables still in the code: AA_RMG_GROUPS, AA_RMG_DUMP_DEF, AA_RMG_BALANCE_DEBUG, AA_RMG_HIT_DEBUG, AA_RMG_TILES.

Note: SVars.tgi sets `RMGBalanceMarkerNudgeStepDelta`, but the game registers `RMGBalanceMarkerNudgeDelta`. The file value is ignored, so the object balancer's delta stays 4.0 and the kingdom balancer's is 2.
