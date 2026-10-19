PHASE 8 - CAUGHT MISSING-SOURCE PATHS OF THE RESOURCE FACTORIES (5A80D0, 59B4B0/59B280)
========================================================================================

Problem: 13 definitions failed pre-World source preparation with
"5130A0 actual requested Resource source file is missing" (Berlin census02/look01;
reproduced headless by resource_depot_missing_tests::actual_pre_world_missing_source_census:
prepared 1336, failed 133, the same 13 paths).

Result: case (b) for all 13. Every requested file is absent from the original
install (Data.rwd and every other depot, in any ASCII case), and the native
factory wrapper catches the TGC_MissingResourceException itself. With the
shipped UVars (DataCompatibilityMode true, DataMissingResourceWarning false)
the original logs a level-10 diagnostic and continues:
  - Sequence (.kf): 5A80D0 returns NULL. 463B50 builds the Animation with a
    NULL sequence; 49DA40 skips that ObjectAnimation. No file is published.
  - Sound (.wav): 59B4B0 returns 59B280's default resource
    (AVar DefaultAudioResource = audio/Default/SoundDefault.wav, present).
With DataCompatibilityMode false both handlers rethrow (96BEF5(0,0)).

Requested paths (all absent):
  cow                     Units/Ambient/Cow/Cowjog.kf            (%s%s.kf: Cow/Cow + jog)
  horse                   Units/Ambient/Horse/Horsejog.kf        (Horse/Horse + jog)
  fish                    Units/Ambient/Fish/Fishdie1.kf         (Fish/Fish + die1)
  sheep_ewe, sheep_ram    Units/Ambient/Sheep/SheepFidget2.kf    (explicit kf)
  usa_infantry_dummy      Units/USA/USAInfantryFlame/USAInfantryFlameJogKneel.kf
                          (+ ...IdleAttackKneel.kf, ...Attack1Kneel.kf further on)
  jpn_battleship(_dynamic) Units/JPN/GJPNBattleship/JPNBattleshipRecoil5.kf (stray "G")
  ger_bunker_aa           Buildings/GER/GERBunkerAA/GERBunkerAADisabledRecoil1.kf
  ger_bunker_artillery    Buildings/GER/GERBunkerArtillery/GERBunkerArtilleryDisabledRecoil1.kf
  tropical_bamboothicket  Doodads/Tropical/TropicalBamboo/TropicalBamboo01Rustle.kf
  tropical_grasstall      Doodads/Tropical/TropicalGrassTall/TropicalTallGrass.kf
  rus_rocket_truck        Audio/Sounds/RUS/Attack/RUSRocketTruck01.wav (and ...02.wav)
The Rust path building is correct (same strings the native forms); the casing
(Horsejog, Cowjog, Fishdie1) comes from "%s%s.kf" with the lower-case IDS and is
irrelevant: the lookup is case-insensitive and no variant exists.

Native evidence (static, identical in original and v14 images):
  5A80D0 KKC_NiSeqResource::GetNiSeqResource: try{5A7750(name,1)}, FuncInfo AEED7C,
    one catch TGC_MissingResourceException& (B28008) -> 5A8115.
  59B4B0: try{59B1F0(name,1)}, FuncInfo AED848, one catch B28008 -> 59B502.
  425210 ("Unable to find") throws ThrowInfo AD9DA0 = TGC_MissingResourceException;
  4252A0 ("Not a") throws TGC_CorruptResourceException, NOT caught here.
  463B50, 49DA40, 46BB60 (the only callers) have no try blocks.
  The handlers read [[B39940]+F4] = UVar61 DataCompatibilityMode (the same cell
  5A7976 reads for Textures) and DataMissingResourceWarning through the B3994C
  table (the same lookup 5A6E90 inlines).

Files
-----
native_missing_catch.py  Unicorn oracle (phase8-ai/native_machine.py), 24 rows:
                         factory x mode x warning x present (x streaming).
                         Seams are listed in its docstring. The single-frame MSVC
                         catch dispatch is reproduced; the handler, continuation,
                         59B280 and every other instruction are original code.
fixtures.json            oracle output (static EH facts + rows + visited bytes).
findings.json            addresses, paths, pins, open gaps.

Rust (crates/aa-runtime/src/resource_depot.rs)
----------------------------------------------
Resources::sequence_5a80d0_with_policy(catalog,bank,ids,&mut impl TexturePolicy)
    -> Result<Option<Handle>>     found: Some; missing+mode1: None + diagnostic
Resources::sound_59b4b0_with_policy(catalog,bank,ids,streaming,&mut impl TexturePolicy)
    -> Result<Handle>             missing+mode1: default Sound + diagnostic
Resources/Application::missing_resource_diagnostics() -> the retained 404FC0
    records (channel, level, file, line, function, format, name; .message()).
The policy is the existing TexturePolicy: only its two current-variable reads
(data_compatibility_mode_current, data_missing_resource_warning_current) are
used, because the native reads are the same two cells. The old sequence_5a80d0
and sound_59b4b0 are kept; on a genuine miss they still fail, now naming the
path and the policy entry point.
Tests (resource_depot_missing_tests.rs, #[ignore], AA_GAME_ROOT):
  native_missing_catch_fixture_replay            every fixture row on real data
  census_missing_names_take_native_catch_paths   all census names
  actual_pre_world_missing_source_census         headless census (~18 min debug)

Caller changes still required (files owned by other sessions)
--------------------------------------------------------------
1. actor_animation_source.rs  Current::resource (line ~154):
     self.resources.sequence_5a80d0_with_policy(self.catalog,self.bank,&ids,&mut *self.texture_policy)
   (drop the .map(Some)). actor_animation_core already stores a None resource
   and 45EA60 already skips it, as 463B50/45EA60 do.
2. object_animation_set.rs  Current::resource (line ~84): same call. Current
   needs a texture policy field, so load_source_node/load take
   `texture_policy:&mut impl TexturePolicy`, and data_aspect.rs:359 passes its
   in-scope `texture_policy`. object_animation_core already returns early on
   None, matching 49DA40.
3. audio_data_pre_request.rs  Loading::resource (line ~191):
     self.resources.sound_59b4b0_with_policy(self.catalog,self.bank,&ids,streaming,&mut *self.texture_policy)
   which needs `texture_policy` threaded through State::prepare, and from
   data_object_request.rs prepare_graph_node (callers: prepare_object_inner and
   prepare_actor_animation_requests_with_textures pass their texture_policy;
   data_actor_request.rs:103 prepare_actor_request_graph needs it too).
4. aa-client may drain Application/Resources::missing_resource_diagnostics()
   to its log (level 10 under the shipped UVars).

Open
----
- DefaultAudioResource is read from the AVars declaration (Settings.app_variables)
  inside the installed fresh no-profile variable window, like
  ResourceCacheAudioRAMWeight and LanguageIDS. A runtime 412070 assignment of
  that AVar is not modelled.
- 404FC0's channel threshold/writer is not modelled; records are retained.
- Exception dispatch (__CxxFrameHandler/RaiseException) is reproduced by the
  oracle, not executed.
