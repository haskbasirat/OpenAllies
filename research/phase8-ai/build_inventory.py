"""Inventory of native AA.exe artificial-intelligence functions.

Read-only. Sources: reconstruction/research/native-aa/{functions,calls,
string-references}.tsv, the original AA.exe .rdata (vtable runs), Rust sources
under crates/ (address citations) and oracle fixture JSON under research/
(executed instruction VAs). Writes research/phase8-ai/inventory.json.

Module ranges are translation-unit estimates bounded by the nearest functions
that reference a .cpp source-path string; they are evidence, not symbols.
"ported" means a Rust source under crates/*/src cites the address. "executed"
means an oracle fixture recorded the function entry as executed original code.
"proven" means ported, executed, and that fixture is replayed by a Rust test.
These flags are mechanical; README.txt states the reviewed status of the
functions ported in phase8.
"""
from pathlib import Path
import bisect, collections, hashlib, json, re, struct

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
GAME = ROOT.parent
NATIVE = GAME / 'reconstruction/research/native-aa'
EXE = GAME / 'AA.exe'
sha = lambda b: hashlib.sha256(b).hexdigest()

# (module, start, end, evidence)
MODULES = [
    ('SAIData', 0x4a5000, 0x4b2830, 'Source/Data/SAIData.cpp loaders and goal/ego/recruit TGI keys (4A66E0..4B0560)'),
    ('Tai', 0x515a00, 0x539c40, 'Source/Sim/Tai/{AAI,CAI,Mind,UAI}.cpp between Zones.cpp (515970) and Solver/FloodFill.cpp (539C40)'),
    ('SAI', 0x79d300, 0x7d0a70, 'Source/Sai/{SAI,SAIGoalEngine,SAIMap,SAIPlayer,SAIRegion,Goals/*}.cpp after OfficialMapManager (79CE10)'),
    ('MGAIData', 0x95fd70, 0x960e00, 'MetaGame AI territory/attack/defend importance keys (95FD70..960950)'),
    ('MGAI', 0x962700, 0x96ad80, 'Source/Mgai/{MGAIPlayer,MGAIGoal}.cpp'),
]
# Mind-state metadata initializers (CRT) outside the module ranges.
EXTRA_RANGES = [('MindStateMetadata', 0x9cdf00, 0x9d0100, 'aai_/cai_/uai_ state metadata CRT initializers 9CDF30..9D00xx')]

# Reviewed phase8 status. Overrides the mechanical flags' interpretation for
# addresses touched in this round (citations of callers/seams are not ports).
PHASE8 = {
    '0096ce20': ('ported, oracle-proven (CRT qsort, VS2003)', 'aa_mind::sai::crt_qsort::qsort', 'native-goal-qsort.json'),
    '0096cdb0': ('ported, oracle-proven (CRT shortsort)', 'aa_mind::sai::crt_qsort', 'native-goal-qsort.json'),
    '007a30b0': ('ported, oracle-proven', 'aa_mind::sai::engine::base_priority_order', 'native-goal-qsort.json'),
    '007a7700': ('ported, oracle-proven (goal virtuals as services)', 'aa_mind::sai::engine::reevaluate_goals', 'native-goal-engine.json'),
    '007a93b0': ('ported, oracle-proven (7A9150/7A7E30 as services)', 'aa_mind::sai::engine::select_goals', 'native-goal-engine.json'),
    '007a3140': ('ported inline (array remove_if), oracle-proven in 7A93B0', 'aa_mind::sai::engine::select_goals', 'native-goal-engine.json'),
    '007a32f0': ('ported inline (list remove_if), oracle-proven in 7A7700', 'aa_mind::sai::engine::reevaluate_goals', 'native-goal-engine.json'),
    '007a2f90': ('ported inline predicate', 'aa_mind::sai::engine::select_goals', 'native-goal-engine.json'),
    '007a2fb0': ('ported inline predicate', 'aa_mind::sai::engine::reevaluate_goals', 'native-goal-engine.json'),
    '007a9150': ('ported, oracle-proven (goal/actor virtuals as services)', 'aa_mind::sai::assign::evaluate', 'native-goal-assign.json'),
    '007a7e30': ('ported, oracle-proven (goal/actor virtuals as services)', 'aa_mind::sai::assign::assign', 'native-goal-assign.json'),
    '0079df70': ('seam: fiber time-slice yield', None, 'native-goal-engine.json'),
    '00521060': ('ported, oracle-proven', 'aa_mind::cai_engage::can_add', 'native-cai-engage.json'),
    '00523dd0': ('ported, oracle-proven', 'aa_mind::cai_engage::add_target', 'native-cai-engage.json'),
    '00524c10': ('ported, oracle-proven', 'aa_mind::cai_engage::mind_add_target', 'native-cai-engage.json'),
    '0051efb0': ('ported, oracle-proven', 'aa_mind::cai_engage::remove_target', 'native-cai-engage.json'),
    '0051f040': ('ported, oracle-proven', 'aa_mind::cai_engage::clear_targets', 'native-cai-engage.json'),
    '00523ea0': ('ported, oracle-proven', 'aa_mind::cai_engage::refresh_targets', 'native-cai-engage.json'),
    '0051fe60': ('ported, oracle-proven', 'aa_mind::cai_engage::insert_sorted', 'native-cai-engage.json'),
    '005e2250': ('ported inline (list unlink)', 'aa_mind::cai_engage::remove_target', 'native-cai-engage.json'),
    '0051f9f0': ('ported, oracle-proven', 'aa_mind::cai_reactions::exhausted', 'native-cai-reactions.json'),
    '0051f920': ('ported, oracle-proven', 'aa_mind::cai_reactions::wavering', 'native-cai-reactions.json'),
    '005206a0': ('ported, oracle-proven', 'aa_mind::cai_reactions::kill_on_enter', 'native-cai-reactions.json'),
    '0052e830': ('ported, oracle-proven', 'aa_mind::cai_reactions::engage_reaction_delay', 'native-cai-reactions.json'),
    '00527240': ('ported, oracle-proven', 'aa_mind::cai_reactions::target_priority', 'native-cai-reactions.json'),
    '0052a810': ('ported in phase7 (aa_mind::activate); executed natively in the 51F9F0/51F920 oracle', 'aa_mind::activate', 'native-cai-reactions.json'),
    '005340c0': ('ported, oracle-proven (ability services as seams)', 'aa_mind::uai_kill::choose', 'native-uai-kill.json'),
    '00535240': ('ported, oracle-proven (5340C0/5307A0 as services)', 'aa_mind::uai_kill::think', 'native-uai-kill.json'),
    '00517f60': ('ported inline (enabled-ability iterator)', 'aa_mind::uai_kill::choose', 'native-uai-kill.json'),
    '00520320': ('service owned by the movement agent (CAI move to actor)', None, 'native-cai-reactions.json'),
    '005276b0': ('owned by the movement agent (CAI move Think)', None, None),
    '0052e9d0': ('owned by the movement agent (member return-to-slot)', None, None),
    '0052e900': ('owned by the movement agent (member return-to-slot)', None, None),
    '007acb00': ('owned by the movement agent (static region topology)', None, None),
    '007a5200': ('ported, oracle-proven', 'aa_mind::sai::goal_priority::finalize', 'native-goal-priority.json'),
    '007a3860': ('ported, oracle-proven', 'aa_mind::sai::goal_priority::affordable', 'native-goal-priority.json'),
    '0052a930': ('ported, oracle-proven (445400 ST0 bound as service)', 'aa_mind::rout_score::score', 'native-rout-score.json'),
    '0052b530': ('ported, oracle-proven with native 52A930', 'aa_mind::rout_select::select', 'native-rout-select.json'),
    '0052bb40': ('ported, oracle-proven with native 52A930/58AC70', 'aa_mind::recover_select::select', 'native-recover-select.json'),
    '0058ac70': ('ported, oracle-proven inside 52BB40', 'aa_mind::recover_select::SupplyComponent::active', 'native-recover-select.json'),
    '00521240': ('ported, oracle-proven (callees as services)', 'aa_mind::cai_rout::think', 'native-rout-think.json'),
    '0051e110': ('ported, oracle-proven', 'aa_mind::cai_states::retreat', 'native-state-thinks.json'),
    '0052df30': ('ported, oracle-proven', 'aa_mind::cai_states::deploy', 'native-state-thinks.json'),
    '0051de10': ('ported, oracle-proven', 'aa_mind::cai_states::regroup', 'native-state-thinks.json'),
    '0051e260': ('ported, oracle-proven', 'aa_mind::cai_states::exhausted', 'native-state-thinks.json'),
    '00523930': ('ported, oracle-proven', 'aa_mind::cai_states::chasing', 'native-state-thinks.json'),
    '00578140': ('seam: StateThinkWorld::organization_status (aa-actors Organization status)', None, 'native-state-thinks.json'),
    '00520d90': ('seam: StateThinkWorld::chase_continue (not ported)', None, 'native-state-thinks.json'),
    '0052a340': ('ported, oracle-proven', 'aa_mind::pop', 'native-mind-primitives.json'),
    '00529e60': ('ported, oracle-proven (diagnostic latch excluded)', 'aa_mind::think_base', 'native-mind-primitives.json'),
    '0052a000': ('ported, oracle-proven', 'aa_mind::is_current', 'native-mind-primitives.json'),
    '0052a1f0': ('ported, oracle-proven', 'aa_mind::within_arrival', 'native-mind-primitives.json'),
    '00529c30': ('trivial accessor (Mind+4 actor +4), modelled by callers', None, None),
    '00529f50': ('executed natively in 521240 oracle; RoutThinkWorld::mark_command_destination service', None, 'native-rout-think.json'),
    '0045d400': ('ported (inline form), oracle-proven via 7A5200/52A930', 'aa_mind::x87::unit_random', 'native-goal-priority.json'),
    '007bddd0': ('table ported, oracle-proven', 'aa_mind::sai::tables::GOAL_TYPES', 'native-sai-tables.json'),
    '007bdb70': ('table ported, oracle-proven', 'aa_mind::sai::tables::GOAL_TYPES', 'native-sai-tables.json'),
    '007bdcf0': ('table ported, oracle-proven', 'aa_mind::sai::tables::DIFFICULTY_LEVELS', 'native-sai-tables.json'),
    '007bdea0': ('table ported, oracle-proven', 'aa_mind::sai::tables::GOAL_ACTOR_PROXIMITY', 'native-sai-tables.json'),
    '007bda60': ('seam (enum entry registration recorded, not ported)', None, 'native-sai-tables.json'),
    '0079e4b0': ('seam: GoalWorld::sai_count (not ported)', None, 'native-goal-priority.json'),
    '0040eb50': ('seam: SAIDebug setting query (diagnostic)', None, 'native-goal-priority.json'),
    '00445400': ('executed natively in 52A930 oracle; RoutWorld::angle service (aa_trig::signed_angle is the stored variant)', None, 'native-rout-score.json'),
    '0085daa0': ('executed natively in 52B530 oracle over recovered tables; Rust uses aa_trig::lookup_sin', None, 'native-rout-select.json'),
    '0085da80': ('executed natively in 52B530 oracle over recovered tables; Rust uses aa_trig::lookup_cos', None, 'native-rout-select.json'),
    '004f02b0': ('executed natively in 521240 oracle (world RNG accessor); service', None, 'native-rout-think.json'),
    '0054efc0': ('seam: RoutWorld::zone (not ported)', None, 'native-rout-score.json'),
    '0054f8b0': ('seam: RoutWorld::friendly_zone (not ported)', None, 'native-rout-score.json'),
    '0052a690': ('seam: SelectWorld::reachable_delta (not ported)', None, 'native-rout-select.json'),
    '004f46e0': ('seam: find_open_position (spatial, not ported)', None, 'native-rout-select.json'),
    '00520440': ('seam: RoutThinkWorld::activate_move (CAI move activation, not ported)', None, 'native-rout-think.json'),
    '004b2e60': ('seam: SVar lookup (values are inputs)', None, 'native-rout-score.json'),
    '004af500': ('ported, oracle-proven on the shipped data (GoalPriority)', 'aa_mind::sai::ego_lists::goal_priority', 'native-saidata-ego.json'),
    '004b0560': ('ported, oracle-proven on the shipped data (KKC_SAIEgoData::LoadData)', 'aa_mind::sai::ego::load', 'native-saidata-ego.json'),
    '004ab9a0': ('ported, oracle-proven (ego constructor)', 'aa_mind::sai::ego::Ego::new', 'native-saidata-ego.json'),
    '004a8e00': ('ported, oracle-proven (ego reset)', 'aa_mind::sai::ego::Ego::new', 'native-saidata-ego.json'),
    '004a5c60': ('ported, oracle-proven (exploration goal defaults)', 'aa_mind::sai::ego::Ego::new', 'native-saidata-ego.json'),
    '004a5d60': ('ported, oracle-proven (GoalPriority slot constructor)', 'aa_mind::sai::ego::Ego::new', 'native-saidata-ego.json'),
    '004a5d80': ('ported, oracle-proven (GoalPriority slot equality)', 'aa_mind::sai::ego::Ego::goal_slot_is_default', 'native-saidata-ego.json'),
    '004ad980': ('ported, oracle-proven (Filters)', 'aa_mind::sai::ego_lists::filters', 'native-saidata-ego.json'),
    '004af230': ('ported, oracle-proven (ResourceNeed)', 'aa_mind::sai::ego_lists::resource_needs', 'native-saidata-ego.json'),
    '004aad80': ('ported, oracle-proven (ComponentValue)', 'aa_mind::sai::ego_lists::component_value', 'native-saidata-ego.json'),
    '004af8e0': ('ported, oracle-proven (ActorPriority)', 'aa_mind::sai::ego_lists::actor_priority', 'native-saidata-ego.json'),
    '004a66e0': ('ported, oracle-proven (exploration goals)', 'aa_mind::sai::ego_lists::exploration_goals', 'native-saidata-ego.json'),
    '004adf50': ('ported, oracle-proven (RecruitPreference)', 'aa_mind::sai::ego_lists::recruit_preference', 'native-saidata-ego.json'),
    '004ab090': ('ported, oracle-proven (SpecificBuildingBonus)', 'aa_mind::sai::ego_lists::building_bonus', 'native-saidata-ego.json'),
    '004aaeb0': ('ported, oracle-proven (SpecificRecruitRequest)', 'aa_mind::sai::ego_lists::recruit_request', 'native-saidata-ego.json'),
    '004a7c70': ('ported, oracle-proven (recruit request DataRef)', 'aa_mind::sai::ego_lists::recruit_request', 'native-saidata-ego.json'),
    '004afd50': ('ported, oracle-proven (GeneralPower)', 'aa_mind::sai::ego_lists::general_power', 'native-saidata-ego.json'),
    '004afb60': ('ported, oracle-proven (GeneralPowerActorRequirements)', 'aa_mind::sai::ego_lists::actor_requirements', 'native-saidata-ego.json'),
    '004aa1d0': ('ported, oracle-proven (GeneralPower constructor)', 'aa_mind::sai::ego::GeneralPower::new', 'native-saidata-ego.json'),
    '004a8700': ('ported inline (vector fill), oracle-proven', 'aa_mind::sai::ego::load', 'native-saidata-ego.json'),
    '004aedc0': ('ported, oracle-proven (formations)', 'aa_mind::sai::ego_lists::formations', 'native-saidata-ego.json'),
    '004af0c0': ('ported, oracle-proven (GoalFormation)', 'aa_mind::sai::ego_lists::goal_formation', 'native-saidata-ego.json'),
    '004a5c20': ('ported, oracle-proven (EconomicExcess qsort comparator)', 'aa_mind::sai::ego::load', 'native-saidata-ego.json'),
    '004a86d0': ('ported, oracle-proven (hash contains)', 'aa_mind::sai::native_hash::NativeHash::contains', 'native-saidata-ego.json'),
    '007ba3c0': ('ported, oracle-proven (hash insert)', 'aa_mind::sai::native_hash::NativeHash::insert', 'native-saidata-ego.json'),
    '00606750': ('ported, oracle-proven (hash rehash)', 'aa_mind::sai::native_hash::NativeHash::insert', 'native-saidata-ego.json'),
    '007a6430': ('ported, oracle-proven (hash find-or-insert)', 'aa_mind::sai::native_hash::NativeHash::find_or_insert', 'native-saidata-ego.json'),
    '0040d770': ('container growth (capacity only, not observable)', None, 'native-saidata-ego.json'),
    '004849f0': ('container growth (capacity only, not observable)', None, 'native-saidata-ego.json'),
    '004a6bb0': ('container growth (capacity only, not observable)', None, 'native-saidata-ego.json'),
    '004a6c40': ('ported inline (component vector growth constructs {0, 1})', 'aa_mind::sai::ego_lists::component_value', 'native-saidata-ego.json'),
    '0045f180': ('ported inline (vector resize zero-filled)', 'aa_mind::sai::ego_lists::general_power', 'native-saidata-ego.json'),
    '00766de0': ('ported inline (vector push_back)', 'aa_mind::sai::ego::Ego::new', 'native-saidata-ego.json'),
    '0047d960': ('ported, oracle-proven (DataItem base load)', 'aa_mind::sai::profile::base_load', 'native-saidata-profile.json'),
    '004aa430': ('ported, oracle-proven (SAI profile constructor)', 'aa_mind::sai::profile::SaiProfile::new', 'native-saidata-profile.json'),
    '004ae3e0': ('ported, oracle-proven on the shipped data (KKC_SAIData::LoadData)', 'aa_mind::sai::profile::load', 'native-saidata-profile.json'),
    '004ae340': ('ported (ego link; not executed by the oracle)', 'aa_mind::sai::profile::link_ego', None),
    '00473ad0': ('ported (DataRef queued on [B36530]+824), oracle-proven', 'aa_mind::sai::ego::DataRef', 'native-saidata-ego.json'),
    '00497220': ('ported (enum map lookup), oracle-proven', 'aa_mind::sai::tables::lookup', 'native-saidata-ego.json'),
    '0048d500': ('seam: DataManagers::find (aa-rules native registry)', None, 'native-saidata-ego.json'),
    '007a1b30': ('ported, oracle-proven (actor virtuals as services; route block = aa_navigation::route_cache::route_cost)', 'aa_mind::sai::travel::travel_cost', 'native-travel-time.json'),
    '007a1c10': ('ported, oracle-proven', 'aa_mind::sai::travel::travel_time', 'native-travel-time.json'),
    '004bdba0': ('seam: EgoServices::thing (aa-rules Thing registry)', None, 'native-saidata-ego.json'),
    '00463fa0': ('seam: EgoServices::property', None, 'native-saidata-ego.json'),
    '007a0aa0': ('cited only (SAI constructor seeding the SAI RNG from QPC), not ported', None, None),
    '007a0080': ('cited only (KKC_SAI::Think fibers/time budget), not ported', None, None),
    '007af220': ('cited only (KKC_SAICommandData::Send), not ported', None, None),
    '0051f190': ('ported, oracle-proven (4D3FA0 as service)', 'aa_mind::cai_rout::enter', 'native-rout-enter.json'),
    '0052c0d0': ('ported inline (pending command release)', 'aa_mind::cai_rout::enter', 'native-rout-enter.json'),
    '004d3fa0': ('seam: RoutEnterWorld::organization_halt (not ported)', None, 'native-rout-enter.json'),
    '007a7eb0': ('ported, oracle-proven', 'aa_mind::sai::assign::gather', 'native-goal-assign.json'),
    '007a6c70': ('ported, oracle-proven', 'aa_mind::sai::assign::assign_from_pool', 'native-goal-assign.json'),
    '007a3740': ('ported, oracle-proven', 'aa_mind::sai::assign::assign_required', 'native-goal-assign.json'),
    '007a35d0': ('ported, oracle-proven', 'aa_mind::sai::assign::evaluate_required', 'native-goal-assign.json'),
    '007a3e10': ('ported, oracle-proven', 'aa_mind::sai::assign::take_from_lower', 'native-goal-assign.json'),
    '007a3f30': ('ported, oracle-proven', 'aa_mind::sai::assign::take_from_higher', 'native-goal-assign.json'),
    '007a50e0': ('ported, oracle-proven', 'aa_mind::sai::assign::trade', 'native-goal-assign.json'),
    '007a33e0': ('ported, oracle-proven', 'aa_mind::sai::assign::swap', 'native-goal-assign.json'),
    '007afa40': ('ported, oracle-proven', 'aa_mind::sai::assign::release_from_pools', 'native-goal-assign.json'),
    '0056a1f0': ('ported, oracle-proven', 'aa_mind::sai::assign::ActorList::remove_first', 'native-goal-assign.json'),
    '007a6480': ('ported, oracle-proven (7A2FB0 and 7A10E0 predicates)', 'aa_mind::sai::assign::ActorList::remove_null', 'native-goal-assign.json'),
    '007a1120': ('ported inline (category mask test)', 'aa_mind::sai::assign::release_from_pools', 'native-goal-assign.json'),
    '004b9e60': ('ported (node push-front)', 'aa_mind::sai::assign::ActorList::push_front', 'native-goal-base.json'),
    '00473300': ('ported inline (node release; refcount modelled by caller ownership)', None, 'native-goal-assign.json'),
    '007a1030': ('ported inline (smart pointer assignment)', 'aa_mind::sai::assign::swap', 'native-goal-assign.json'),
    '007a3b00': ('ported, oracle-proven', 'aa_mind::sai::goal_base::priority_with_actors', 'native-goal-base.json'),
    '004a7130': ('ported, oracle-proven (CRT pow via aa_numeric::crt_pow)', 'aa_mind::sai::goal_base::travel_weight', 'native-goal-base.json'),
    '0096d890': ('aa_numeric::crt_pow::ci_pow (SSE2 body), executed natively in native-goal-base.json', None, 'native-goal-base.json'),
    '007a4fb0': ('ported, oracle-proven', 'aa_mind::sai::goal_base::swap_value', 'native-goal-base.json'),
    '007a5020': ('ported, oracle-proven', 'aa_mind::sai::goal_base::value', 'native-goal-base.json'),
    '007a5080': ('ported, oracle-proven', 'aa_mind::sai::goal_base::held_value', 'native-goal-base.json'),
    '0059e240': ('ported (count accessor)', 'aa_mind::sai::goal_base::actor_count', None),
    '007a5600': ('ported, oracle-proven', 'aa_mind::sai::goal_base::deselect', 'native-goal-base.json'),
    '007a3d30': ('ported, oracle-proven', 'aa_mind::sai::goal_base::release_all', 'native-goal-base.json'),
    '007afb00': ('ported, oracle-proven', 'aa_mind::sai::goal_base::return_to_pool', 'native-goal-base.json'),
    '007a6930': ('ported, oracle-proven', 'aa_mind::sai::goal_base::add_actor', 'native-goal-base.json'),
    '007a6a90': ('ported, oracle-proven', 'aa_mind::sai::goal_base::remove_actor', 'native-goal-base.json'),
    '007a67b0': ('ported, oracle-proven (7A5200 bound as service)', 'aa_mind::sai::goal_base::reevaluate', 'native-goal-base.json'),
    '007a0f80': ('ported, oracle-proven', 'aa_mind::sai::goal_base::set_owner', 'native-goal-base.json'),
    '007acf50': ('ported (7A0F80 wrapper)', 'aa_mind::sai::goal_base::set_owner', 'native-goal-base.json'),
    '007a10f0': ('seam (GTID registry lookup of the SAI actor game object)', None, 'native-goal-base.json'),
    '0079f390': ('seam: GoalBaseWorld::unit_check (not ported)', None, 'native-goal-base.json'),
    '004a7cd0': ('seam: GoalBaseWorld::apply_modifier (SAIData fuzzy modifier, not ported)', None, 'native-goal-base.json'),
    '00524d70': ('ported, oracle-proven (callees as services)', 'aa_mind::cai_sleep::think', 'native-sleep-think.json'),
    '00526a70': ('ported, oracle-proven (callees as services)', 'aa_mind::cai_engage_think::think', 'native-engage-think.json'),
    '005294f0': ('ported, oracle-proven (callees as services)', 'aa_mind::cai_guard::think', 'native-guard-think.json'),
    '004364c0': ('ported inline (organization member iterator, flag 0)', 'aa_mind::cai_engage_think', 'native-engage-think.json'),
    '00529c20': ('ported inline (leave an active state)', 'aa_mind::cai_guard', 'native-guard-think.json'),
    '00517f30': ('ported inline (GTID list contains)', 'aa_mind::cai_guard', 'native-guard-think.json'),
    '0051d920': ('seam: follow-state activation (not ported)', None, 'native-sleep-think.json'),
    '00527ce0': ('seam: GuardThinkWorld::guard_527ce0 (not ported)', None, 'native-guard-think.json'),
    '0051ee20': ('seam: GuardThinkWorld::guard_51ee20 (not ported)', None, 'native-guard-think.json'),
    '00522f60': ('seam: SleepThinkWorld::sleep_522f60 (not ported)', None, 'native-sleep-think.json'),
    '00523000': ('seam: SleepThinkWorld::sleep_523000 (not ported)', None, 'native-sleep-think.json'),
    '00522890': ('seam: SleepThinkWorld::member_arrived (not ported)', None, 'native-sleep-think.json'),
    '00520120': ('seam: SleepThinkWorld::organization_busy (not ported)', None, 'native-sleep-think.json'),
    '0052bf30': ('ported earlier as aa_mind::command::resume_pending; bound as a service in 524D70', 'aa_mind::command::resume_pending', 'native-sleep-think.json'),
    '00578470': ('seam: EngageThinkWorld::within_engage_range (not ported)', None, 'native-engage-think.json'),
    '00569300': ('seam: GuardThinkWorld::component_threats (denizen component list pruning)', None, 'native-guard-think.json'),
    '0051cc10': ('ported, oracle-proven (Mind-state Think; unported callees as services)', 'aa_mind::uai_thinks::aai_escort::think', 'native-aai-escort.json'),
    '0051c900': ('ported, oracle-proven (Mind-state Think; unported callees as services)', 'aa_mind::uai_thinks::aai_guard::think', 'native-aai-guard.json'),
    '00518d60': ('ported, oracle-proven (Mind-state Think; unported callees as services)', 'aa_mind::uai_thinks::aai_join::think', 'native-aai-join.json'),
    '00519750': ('ported, oracle-proven (Mind-state Think; unported callees as services)', 'aa_mind::uai_thinks::aai_landing::think', 'native-aai-landing.json'),
    '00519f70': ('ported, oracle-proven (Mind-state Think; unported callees as services)', 'aa_mind::uai_thinks::aai_queued::think', 'native-aai-queued.json'),
    '005180b0': ('ported, oracle-proven (Mind-state Think; unported callees as services)', 'aa_mind::uai_thinks::aai_recon::think', 'native-aai-recon.json'),
    '00519150': ('ported, oracle-proven (Mind-state Think; unported callees as services)', 'aa_mind::uai_thinks::aai_takeoff::think', 'native-aai-takeoff.json'),
    '005185d0': ('ported, oracle-proven (Mind-state Think; unported callees as services)', 'aa_mind::uai_thinks::aai_transport::think', 'native-aai-transport.json'),
    '005207f0': ('ported, oracle-proven (Mind-state Think; unported callees as services)', 'aa_mind::cai_thinks::bombard::think', 'native-cai-bombard.json'),
    '0051db80': ('ported, oracle-proven (Mind-state Think; unported callees as services)', 'aa_mind::cai_thinks::construct::think', 'native-cai-construct.json'),
    '00525d70': ('ported, oracle-proven (Mind-state Think; unported callees as services)', 'aa_mind::cai_thinks::explore::think', 'native-cai-explore.json'),
    '00521850': ('ported, oracle-proven (Mind-state Think; unported callees as services)', 'aa_mind::cai_thinks::join::think', 'native-cai-join.json'),
    '00520730': ('ported, oracle-proven (Mind-state Think; unported callees as services)', 'aa_mind::cai_thinks::kill::think', 'native-cai-kill.json'),
    '00527090': ('ported, oracle-proven (Mind-state Think; unported callees as services)', 'aa_mind::cai_thinks::pack::think', 'native-cai-pack.json'),
    '005267e0': ('ported, oracle-proven (Mind-state Think; unported callees as services)', 'aa_mind::cai_thinks::rampage::think', 'native-cai-rampage.json'),
    '0051de70': ('ported, oracle-proven (Mind-state Think; unported callees as services)', 'aa_mind::cai_thinks::reform::think', 'native-cai-reform.json'),
    '005239a0': ('ported, oracle-proven (Mind-state Think; unported callees as services)', 'aa_mind::cai_thinks::repair::think', 'native-cai-repair.json'),
    '00525930': ('ported, oracle-proven (Mind-state Think; unported callees as services)', 'aa_mind::cai_thinks::siege::think', 'native-cai-siege.json'),
    '00526e60': ('ported, oracle-proven (Mind-state Think; unported callees as services)', 'aa_mind::cai_thinks::unpack::think', 'native-cai-unpack.json'),
    '005353e0': ('ported, oracle-proven (Mind-state Think; unported callees as services)', 'aa_mind::uai_thinks::uai_bombard::think', 'native-uai-bombard.json'),
    '00536e10': ('ported, oracle-proven (Mind-state Think; unported callees as services)', 'aa_mind::uai_thinks::uai_engage::think', 'native-uai-engage.json'),
    '00531ad0': ('ported, oracle-proven (Mind-state Think; unported callees as services)', 'aa_mind::uai_thinks::uai_join::think', 'native-uai-join.json'),
    '00534000': ('ported, oracle-proven (Mind-state Think; unported callees as services)', 'aa_mind::uai_thinks::uai_reattach::think', 'native-uai-reattach.json'),
    '005359d0': ('ported, oracle-proven (Mind-state Think; unported callees as services)', 'aa_mind::uai_thinks::uai_repair::think', 'native-uai-repair.json'),
    '0052f950': ('ported, oracle-proven (Mind-state Think; unported callees as services)', 'aa_mind::uai_thinks::uai_sleep::think', 'native-uai-sleep.json'),
    '0052f440': ('ported, oracle-proven (Mind-state Think; unported callees as services)', 'aa_mind::uai_thinks::uai_unload::think', 'native-uai-unload.json'),
    '005372b0': ('ported, oracle-proven (Mind-state Think; unported callees as services)', 'aa_mind::uai_thinks::uai_unpack::think', 'native-uai-unpack.json'),
    '0051b8c0': ('ported, oracle-proven (Mind-state Think; unported callees as services)', 'aa_mind::uai_thinks::aai_attack::think', 'native-aai-attack.json'),
    '007a0c20': ('ported, oracle-proven (SAI actor classification; 79E300/79E080/4CEFF0/79F6E0/7A0390/7A0A40 as services)', 'aa_mind::sai::actor_notify::notify', 'native-actor-notify.json'),
    '007ac0d0': ('ported, oracle-proven (KKC_SAIMap::DynamicMapAnalysis; actor virtuals, 4D0180, 4CEDA0, 7B9320, yield and reschedule as services)', 'aa_mind::sai::map_analysis::dynamic_map_analysis', 'native-map-analysis.json'),
    '007aa5b0': ('ported, oracle-proven (influence reset)', 'aa_mind::sai::map_analysis::dynamic_map_analysis', 'native-map-analysis.json'),
    '007ae350': ('ported, oracle-proven (learning multiplier)', 'aa_mind::sai::map_analysis', 'native-map-analysis.json'),
    '007ad890': ('ported, oracle-proven (learning prune)', 'aa_mind::sai::map_analysis', 'native-map-analysis.json'),
    '007adda0': ('ported, oracle-proven (hash erase)', 'aa_mind::sai::native_hash::NativeHash::erase', 'native-map-analysis.json'),
    '004a5e40': ('ported, oracle-proven (ego learning scale)', 'aa_mind::sai::map_analysis', 'native-map-analysis.json'),
    '007aba70': ('ported, oracle-proven (settlement spread)', 'aa_mind::sai::map_analysis', 'native-map-analysis.json'),
    '007ab940': ('ported, oracle-proven (mobile spread)', 'aa_mind::sai::map_analysis', 'native-map-analysis.json'),
    '007aa620': ('ported, oracle-proven (recursive propagation)', 'aa_mind::sai::map_analysis', 'native-map-analysis.json'),
    '007bb780': ('ported, oracle-proven (influence accumulate +8/+14)', 'aa_mind::sai::map_analysis', 'native-map-analysis.json'),
    '007bb820': ('ported, oracle-proven (influence accumulate +20)', 'aa_mind::sai::map_analysis', 'native-map-analysis.json'),
    '007b9050': ('ported inline (region entry by player slot)', 'aa_mind::sai::map_analysis', 'native-map-analysis.json'),
    '007a9c60': ('ported inline (static actor-list iterator)', 'aa_mind::sai::map_analysis', 'native-map-analysis.json'),
    '007a9c80': ('ported inline (mobile actor-list iterator)', 'aa_mind::sai::map_analysis', 'native-map-analysis.json'),
    '007b9180': ('ported inline (iterator begin)', 'aa_mind::sai::map_analysis', 'native-map-analysis.json'),
    '007bb480': ('ported inline (iterator end test, skips stale GTIDs)', 'aa_mind::sai::map_analysis', 'native-map-analysis.json'),
    '007bac00': ('ported inline (iterator advance)', 'aa_mind::sai::map_analysis', 'native-map-analysis.json'),
    '0045d720': ('ported inline (terrain == 0)', 'aa_mind::sai::map_analysis', 'native-map-analysis.json'),
    '005c37e0': ('ported inline (hash iteration begin)', 'aa_mind::sai::map_analysis', 'native-map-analysis.json'),
    '004d0180': ('seam: MapWorld::combat_value (actor combat value)', None, 'native-map-analysis.json'),
    '004ceda0': ('seam: MapWorld::organization_actor', None, 'native-map-analysis.json'),
    '007b9320': ('seam: MapWorld::region_terrain (first rectangle tile terrain)', None, 'native-map-analysis.json'),
    '0079eff0': ('seam: MapWorld::schedule (SAI scheduler)', None, 'native-map-analysis.json'),
    '00524b80': ('ported, oracle-proven (regiment Mind member added)', 'aa_mind::cai_member_added::member_added', 'native-member-added.json'),
    '004d0900': ('ported inline (list node push)', 'aa_mind::cai_member_added::member_added', 'native-member-added.json'),
    '00534a40': ('bound: element_move_continuation::collision_plan -> aa_navigation::uai_collision::avoid_static (movement-proven)', 'aa_mind::element_move_continuation::collision_plan', None),
    '00536ad0': ('bound: element_move_continuation::stop_distance -> aa_navigation::uai_collision::avoid_thing (movement-proven)', 'aa_mind::element_move_continuation::stop_distance', None),
}

KIND_HINTS = [
    (re.compile(r'KKC_(\w+)::(\w+)'), None),
]


def tsv(path):
    with open(path, encoding='utf-8', errors='replace') as f:
        head = f.readline().rstrip('\n').split('\t')
        for line in f:
            yield dict(zip(head, line.rstrip('\n').split('\t')))


def main():
    functions = {}
    for row in tsv(NATIVE / 'functions.tsv'):
        try:
            a = int(row['address'], 16)
        except ValueError:
            continue
        functions[a] = {'name': row['name'], 'size': int(row['bodyBytes'] or 0), 'thunk': row['thunk'] == 'true'}
    starts = sorted(functions)

    strings = collections.defaultdict(list)
    for row in tsv(NATIVE / 'string-references.tsv'):
        try:
            a = int(row['functionAddress'], 16)
        except ValueError:
            continue
        v = row['value']
        if v not in strings[a]:
            strings[a].append(v)

    callers = collections.defaultdict(set)
    callees = collections.defaultdict(set)
    for row in tsv(NATIVE / 'calls.tsv'):
        try:
            c = int(row['caller'], 16)
            e = int(row['callee'], 16)
        except ValueError:
            continue
        callers[e].add(c)
        callees[c].add(e)

    def module_of(a):
        for name, s, e, _ in MODULES + EXTRA_RANGES:
            if s <= a < e:
                return name
        return None

    ai = {a for a in starts if module_of(a)}

    # Source-file labels: nearest .cpp-referencing function at/before and after.
    cpp = {a: next((s for s in strings[a] if s.endswith('.cpp')), None) for a in ai}
    cpp_points = sorted((a, f) for a, f in cpp.items() if f)

    def source_file(a):
        i = bisect.bisect_right(cpp_points, (a, '￿')) - 1
        before = cpp_points[i][1] if i >= 0 else None
        after = cpp_points[i + 1][1] if i + 1 < len(cpp_points) else None
        if cpp.get(a):
            return cpp[a], 'direct'
        if before and before == after:
            return before, 'bracketed'
        return (before or after), 'nearest'

    # Vtables: runs of >=2 consecutive .rdata dwords that are function starts.
    b = EXE.read_bytes()
    pe = struct.unpack_from('<I', b, 0x3c)[0]
    n = struct.unpack_from('<H', b, pe + 6)[0]
    op = pe + 24
    sec = op + struct.unpack_from('<H', b, pe + 20)[0]
    rdata = None
    for i in range(n):
        name = b[sec + i * 40:sec + i * 40 + 8].rstrip(b'\0')
        vs, va, rs, ro = struct.unpack_from('<IIII', b, sec + i * 40 + 8)
        if name == b'.rdata':
            rdata = (0x400000 + va, b[ro:ro + rs])
    base, data = rdata
    fset = set(starts)
    vt_members = collections.defaultdict(list)  # function -> [(vtable, slot)]
    vtables = {}
    runs = {}
    i = 0
    words = struct.unpack_from('<%dI' % (len(data) // 4), data)
    while i < len(words):
        if words[i] in fset and 0x401000 <= words[i] < 0x9f0000:
            j = i
            while j < len(words) and words[j] in fset and 0x401000 <= words[j] < 0x9f0000:
                j += 1
            if j - i >= 2:
                runs[base + i * 4] = list(words[i:j])
            i = j
        else:
            i += 1
    # Shared stubs (empty/return-constant bodies reused by many classes) do not
    # make a vtable AI-related; a vtable needs a specific AI member.
    appearances = collections.Counter(e for es in runs.values() for e in set(es))
    for vt, entries in runs.items():
        if any(e in ai and appearances[e] <= 8 for e in entries):
            vtables[vt] = entries
            for slot, e in enumerate(entries):
                vt_members[e].append((vt, slot * 4))

    # Vtable labels from member names (KKC_X::Y strings) or describe strings.
    describe = re.compile(r'^([A-Z][A-Za-z ]+) \((?:%s|undecided|dead|resources|region|no target|completed|failed|unselected)')
    vt_label = {}
    for vt, entries in vtables.items():
        classes = collections.Counter()
        for e in entries:
            for s in strings.get(e, []):
                m = re.match(r'KKC_(\w+)::\w+$', s)
                if m:
                    classes[m.group(1)] += 1
                m = describe.match(s)
                if m and module_of(e) == 'SAI':
                    classes['SAIGoal:' + m.group(1)] += 1
        if classes:
            vt_label[vt] = classes.most_common(1)[0][0]

    # Rust citations.
    cited = collections.defaultdict(set)
    hexrun = re.compile(r'[0-9A-Fa-f]{6,}')
    for p in sorted((ROOT / 'crates').glob('*/src/**/*.rs')):
        text = p.read_text(encoding='utf-8', errors='replace')
        rel = p.relative_to(ROOT).as_posix()
        for m in hexrun.finditer(text):
            run = m.group(0)
            if len(run) > 7:
                continue
            for k in range(0, len(run) - 5):
                try:
                    a = int(run[k:k + 6], 16)
                except ValueError:
                    continue
                if a in ai:
                    cited[a].add(rel)

    # Oracle fixtures: executed instruction VAs and whether a Rust test replays them.
    rust_text = '\n'.join(p.read_text(encoding='utf-8', errors='replace')
                          for p in (ROOT / 'crates').glob('*/**/*.rs'))
    executed = collections.defaultdict(set)
    replayed = collections.defaultdict(set)
    for p in sorted((ROOT / 'research').rglob('*.json')):
        if p.stat().st_size > 400_000_000:
            continue
        try:
            text = p.read_text(encoding='utf-8')
        except Exception:
            continue
        vas = set(int(v, 16) for v in re.findall(r'"va"\s*:\s*"0x([0-9a-fA-F]+)"', text))
        vas |= set(int(v, 16) for v in re.findall(r'"va"\s*:\s*"([0-9a-fA-F]{6,8})"', text))
        hit = vas & ai
        if not hit:
            continue
        rel = p.relative_to(ROOT).as_posix()
        is_replayed = p.name in rust_text
        for a in hit:
            executed[a].add(rel)
            if is_replayed:
                replayed[a].add(rel)

    # SAI goal classes: the describe virtual (+4C, used by the 7A5200 debug
    # CSV path) is the unique function referencing each class's describe
    # string; vtable start = slot address - 4C.
    describe_strings = {
        'ATTACK_REGION': 'Attack Region (', 'ATTACK_ACTOR': 'Attack Actor (', 'DEFEND_REGION': 'Defend Region (',
        'DEFEND_ACTOR': 'Defend Actor (', 'AIR_ATTACK': 'Air attack (', 'RECOVER': 'Recover (', 'REPAIR': 'Repair (',
        'EXPLORE': 'Explore (', 'BUILD': 'Build (', 'CONSTRUCT': 'Construct (', 'PACK': 'Pack (', 'UNPACK': 'Unpack (',
        'RECRUIT': 'Recruit (', 'UPGRADE': 'Upgrade (', 'GIVE_RESOURCES': 'Give (resources', 'GIVE_ACTOR': 'Give (%s',
        'DESTROY': 'Destroy (', 'ATTACH_HERO': 'Attach Hero (', 'MOVE_HERO': 'Move Hero (', 'RESEARCH': 'Research (',
        'BOMBARD': 'Bombard (', 'ATTACH': 'Attach (',
    }
    slot_labels = {0x4: 'goal type (7A5200)', 0x48: 'settlement reserve exempt (7A3860)', 0x4c: 'describe (debug)',
                   0x78: 'base priority (7A5200)'}
    goal_classes = {}
    for goal, prefix in describe_strings.items():
        cands = [a for a in ai if module_of(a) == 'SAI' and any(x.startswith(prefix) for x in strings.get(a, []))]
        for d in cands:
            for i in range(0, len(words)):
                if words[i] == d:
                    start = base + i * 4 - 0x4c
                    entries = list(words[(start - base) // 4:(start - base) // 4 + 34])
                    goal_classes.setdefault(goal, []).append({
                        'vtable': hex(start), 'describe': '%08x' % d,
                        'slots': {hex(4 * k): '%08x' % e for k, e in enumerate(entries)},
                        'labelled': {hex(k): '%08x' % entries[k // 4] for k in slot_labels},
                    })
    # Mind states: phase7 recovered (vtable, metadata) pairs per Mind kind and
    # metadata names by executing the CRT initializers. Slot +8 is Think
    # (TAIState base vtable A32450 holds KKC_TAIState::Think 529E60 there).
    mind_states = []
    meta_path = ROOT / 'research/phase7-mind/native-metadata.json'
    if meta_path.exists():
        meta = json.loads(meta_path.read_text(encoding='utf-8'))
        names = {m['metadata_va']: m for m in meta['metadata']}
        for kind, layouts in meta['state_layouts'].items():
            for st in layouts:
                vt = int(st['vtable_va'], 16)
                i0 = (vt - base) // 4
                entries = list(words[i0:i0 + 16])
                m = names.get(st['metadata_va'], {})
                mind_states.append({'mind': kind, 'name': m.get('name'), 'state_offset': st['offset'],
                                    'vtable': hex(vt), 'metadata': st['metadata_va'],
                                    'discard_on_suspend': m.get('discard_on_suspend'),
                                    'think_slot_c': '%08x' % entries[3],
                                    'slots': ['%08x' % e for e in entries]})
    # Code addresses referenced from AI vtables but absent from functions.tsv
    # (Ghidra did not create functions for some vtable-only targets).
    vt_only = set()
    lists = [c['slots'].values() for cls in goal_classes.values() for c in cls] + [st['slots'] for st in mind_states]
    for es in lists:
            for e in es:
                a = int(e, 16) if isinstance(e, str) else e
                if module_of(a) and a not in functions and 0x401000 <= a < 0x9f0000:
                    vt_only.add(a)
    known = sorted(set(starts) | vt_only)

    rows = []
    for a in sorted(vt_only):
        nxt = known[known.index(a) + 1] if known.index(a) + 1 < len(known) else a
        rows.append({
            'address': '%08x' % a, 'module': module_of(a), 'source_file': None,
            'source_file_evidence': 'vtable-only', 'size': nxt - a, 'thunk': False,
            'role': None, 'strings': [], 'vtables': [], 'callers': 0, 'callees': None,
            'not_in_functions_tsv': True, 'ported': False, 'rust_citations': [],
            'oracle_executed': False, 'oracle_fixtures': [], 'proven': False,
        })
    for a in sorted(ai):
        f = functions[a]
        src, how = source_file(a)
        own = [s for s in strings.get(a, [])]
        kkc = sorted({m.group(0) for s in own for m in [re.match(r'KKC_\w+::\w+', s)] if m})
        role = kkc[0] if kkc else None
        vts = [{'vtable': hex(vt), 'slot': hex(slot), 'class': vt_label.get(vt)} for vt, slot in vt_members.get(a, [])]
        if not role:
            labelled = [v for v in vts if v['class']]
            if labelled:
                role = '%s vslot %s' % (labelled[0]['class'], labelled[0]['slot'])
        if not role:
            keys = [s for s in own if re.match(r'^[A-Za-z_][A-Za-z0-9_]+$', s)]
            if keys:
                role = 'reads keys: ' + ', '.join(keys[:6])
        if not role and own:
            role = 'strings: ' + ' | '.join(x[:50] for x in own[:3])
        rows.append({
            'address': '%08x' % a,
            'module': module_of(a),
            'source_file': src.replace('\\\\', '/').lstrip('./') if src else None,
            'source_file_evidence': how,
            'size': f['size'],
            'thunk': f['thunk'],
            'role': role,
            'strings': own[:12],
            'vtables': vts,
            'callers': len(callers.get(a, ())),
            'callees': len(callees.get(a, ())),
            'ported': bool(cited.get(a)),
            'rust_citations': sorted(cited.get(a, ())),
            'oracle_executed': bool(executed.get(a)),
            'oracle_fixtures': sorted(executed.get(a, ())),
            'proven': bool(cited.get(a)) and bool(replayed.get(a)),
        })
        if '%08x' % a in PHASE8:
            status, rust, fixture = PHASE8['%08x' % a]
            rows[-1]['phase8'] = {'status': status, 'rust': rust, 'fixture': fixture}

    summary = collections.OrderedDict()
    for name, s, e, ev in MODULES + EXTRA_RANGES:
        sel = [r for r in rows if r['module'] == name]
        summary[name] = {
            'range': [hex(s), hex(e)], 'evidence': ev, 'functions': len(sel),
            'bytes': sum(r['size'] for r in sel),
            'vtable_only_not_in_functions_tsv': sum(bool(r.get('not_in_functions_tsv')) for r in sel),
            'ported': sum(r['ported'] for r in sel),
            'oracle_executed': sum(r['oracle_executed'] for r in sel),
            'proven': sum(r['proven'] for r in sel),
        }
    report = {
        'schema': 1,
        'method': __doc__,
        'original_sha256': sha(b),
        'producer_sha256': sha(Path(__file__).read_bytes()),
        'summary': summary,
        'vtables': [{'address': hex(vt), 'class': vt_label.get(vt), 'entries': ['%08x' % e for e in es]}
                    for vt, es in sorted(vtables.items())],
        'phase8_external_dependencies': {a: {'status': v[0], 'rust': v[1], 'fixture': v[2]}
                                         for a, v in sorted(PHASE8.items()) if int(a, 16) not in ai},
        'mind_states': mind_states,
        'sai_goal_classes': goal_classes,
        'sai_goal_slot_labels': {hex(k): v for k, v in slot_labels.items()},
        'functions': sorted(rows, key=lambda r: r['address']),
    }
    out = HERE / 'inventory.json'
    out.write_text(json.dumps(report, indent=1) + '\n', encoding='utf-8')
    print(json.dumps(summary, indent=1))


if __name__ == '__main__':
    main()
