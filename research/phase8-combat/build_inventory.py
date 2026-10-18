"""Build inventory.json: native AA.exe combat functions and their port status.

Curated from string references (settings-index.json, Body/Organization/UAI
/CAI source paths), calls.tsv neighbourhoods of the combat roots and the
existing phase6/phase7/phase8 evidence. Every address is checked against the
Ghidra function inventory; Rust citations are discovered by searching crates/.
Status vocabulary:
  proven        ported in Rust and replayed against original-code Unicorn fixtures
  proven-other  proven, but owned by another crate (listed for completeness)
  seam          deliberately a required world service at this boundary
  fragment      a proven part of a larger native function (documented scope)
  not-ported    identified, not ported in Rust yet
"""
from pathlib import Path
import json
import re

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
GAME = ROOT.parent
FUNCS = GAME / 'reconstruction/research/native-aa/functions.tsv'

P6C, P6W, P6CR, P6E = 'research/phase6-combat', 'research/phase6-weapon', 'research/phase6-credit', 'research/phase6-experience'
P7A, P7P = 'research/phase7-ability', 'research/phase7-performer'
P8 = 'research/phase8-combat'
MOR = P8 + '/native-morale-fixtures.json'
EXP = P8 + '/native-experience-amount-fixtures.json'
ROE = P8 + '/native-roe-fixtures.json'
RSP = P8 + '/native-damage-response-fixtures.json'
REA = P8 + '/native-combat-reaction-fixtures.json'
CAP = P8 + '/native-capture-attempt-fixtures.json'

E = []


def add(address, role, area, status, rust=None, oracle=None, note=None):
    E.append(dict(address=address, role=role, area=area, status=status, rust=rust, oracle=oracle, note=note))


# --- Hit resolution, damage, death (Body) -----------------------------------
add('004ca400', 'Weapon/DamageEffect hit: power, offense, defense, type factor, minimum', 'hit resolution', 'proven', 'aa-combat::hit', P6W + '/native-weapon-fixtures.json')
add('004c8330', 'Context target eligibility (actor virtual +B4)', 'hit resolution', 'proven', 'aa-combat::World::resolve_target / morale::effect_organization', P6W + ' + ' + MOR)
add('0057dbf0', 'Offensive factor (organization leader or modifier)', 'hit resolution', 'proven', 'aa-combat::World::offensive_factor', P6W + '/native-weapon-fixtures.json')
add('00501530', 'Modifier offense preference (+58/+5C/1)', 'hit resolution', 'proven', 'aa-combat::offensive_modifier; aa-body::properties', P6W + '/native-weapon-fixtures.json')
add('00501650', 'First matching damage-type factor', 'hit resolution', 'proven', 'aa-body::damage::first_damage_factor', P6C + '/damage-native-oracle.json')
add('004cf110', 'Actor modifier context (actor+78 -> +1C)', 'hit resolution', 'proven', 'world service; executed in phase6 oracles', P6C + '/damage-native-oracle.json')
add('005625c0', 'Body DealDamage: propagation, history, health, responses, capture, death queue', 'damage', 'proven', 'aa-body::damage::deal_damage', P6C + '/damage-native-oracle.json')
add('00561e50', 'Body source-aware SetHealth and directional health FX', 'damage', 'proven', 'aa-body::Body::set_health_from_source', P6C + '/damage-native-oracle.json')
add('00561be0', 'Damage record copy used by wall propagation', 'damage', 'proven', 'aa-body::damage', P6C + '/damage-native-oracle.json')
add('00409230', '2D normalize used by directional health FX', 'damage', 'proven', 'aa-body', P6C + '/damage-native-oracle.json')
add('00562ba0', 'Body plain heal', 'damage', 'proven', 'aa-body::Body::heal_plain', 'research/phase4-body/body-native-oracle.json')
add('00562200', 'Body set health and maximum', 'damage', 'proven', 'aa-body::Body::set_health_and_maximum', 'research/phase4-body/body-native-oracle.json')
add('00561d40', 'Body maximum update with ratio', 'damage', 'proven', 'aa-body::Body::update_resolved_maximum', 'research/phase4-body/body-native-oracle.json')
add('00561960', 'Body field initialization', 'damage', 'proven', 'aa-body::Body::from_resolved', 'research/phase4-body/body-native-oracle.json')
add('005617e0', 'Corpse sink fraction', 'death', 'proven', 'aa-body::Body::sink_fraction_f32', 'research/phase4-body/body-native-oracle.json')
add('00561c20', 'Body CheckDeath dispatch', 'death', 'proven', 'aa-body::Body::check_death', 'research/phase4-body/body-native-oracle.json')
add('00562380', 'Body StartDying: death ability, corpse, sink, decay timer, allegiance removal', 'death', 'proven', 'aa-body::dying::start_dying', P6C + '/dying-native-oracle.json')
add('004f1a90', 'World death queue push (list head)', 'death', 'proven', 'aa-combat::death::DeathQueue::push', RSP)
add('00584e70', 'Wall member killed: queue every segment or the actor', 'death', 'proven', 'aa-combat::death::wall_member_killed', RSP)
add('004f65b0', 'World phase change: LIFO death drain (CheckDeath) and phase writes', 'death', 'fragment', 'aa-combat::death::change_phase', RSP,
    'Removal-list processing 4F6634..4F6743 is a required world callback; only the empty-list case is proven.')
add('00584cd0', 'Outer/wall component last-damage record copy', 'damage', 'proven', 'aa-combat::death::outer_damaged', RSP)
add('004d7960', 'Actor BeginDeath (called by CheckDeath)', 'death', 'seam', 'aa-body Host callback', None)
add('004f4c40', 'World actor removal (removal list)', 'death', 'not-ported', None, None)
add('005818d0', 'Properties periodic tick: expiry, healing, periodic damage', 'damage', 'proven', 'aa-combat::periodic::tick', P6W + '/native-periodic-fixtures.json')
add('005034f0', 'Property removal', 'damage', 'proven', 'aa-combat::periodic::remove_property', P6W + '/native-periodic-fixtures.json')
add('005032d0', 'Duplicate property selection', 'damage', 'proven', 'aa-combat::periodic::remove_property', P6W + '/native-periodic-fixtures.json')
add('004c0a80', 'Damage type presentation load', 'damage', 'proven', 'aa-combat::types::load', P6W + '/native-damage-type-fixtures.json')
add('00562d10', 'Body shortfall damage: RNG draw, event-kind-4 DealDamage', 'damage', 'proven', 'aa-combat::shortfall::damage', RSP)
add('00578660', 'Organization TickShortfallDamage member loop', 'damage', 'proven', 'aa-combat::shortfall::tick', RSP)
add('0050d0b0', 'Damage presentation queue (UI damage numbers)', 'presentation', 'seam', 'aa-body::damage Action::PresentationDamaged', None)
add('0056ae70', 'Component88 damaged dispatch (569A80/568D80)', 'damage', 'not-ported', None, None, 'aa-body Action::Component88Damaged remains a required world action.')
add('00569a80', 'Component88 damage predicate (recursive, range test)', 'damage', 'not-ported', None, None)
add('00568d80', 'Component88 damage response', 'damage', 'not-ported', None, None)
add('004d6660', 'Actor recovery state (state 0x14) refresh after damage', 'damage', 'not-ported', None, None, 'aa-body Action::ResetRecovery remains a required world action.')
# --- Capture ------------------------------------------------------------------
add('004d1120', 'Capture eligibility (segments, NOT_CAPTURABLE, +94/+D4, captureable)', 'capture', 'proven', 'aa-combat::capture::eligible', RSP)
add('004ceda0', 'Ancestor via wall-segment owner / outer chain', 'capture', 'proven', 'executed inside 4D1120 oracle', RSP)
add('004579d0', 'Component +34 GTID registry lookup', 'capture', 'proven', 'aa-combat::capture::World::resolve', RSP)
add('004d5230', 'Capture attempt: owner change, announcements, SAI, statistics, reward', 'capture', 'proven', 'aa-combat::capture::attempt', CAP)
add('004cebe0', 'Capture reward: 5719E0 experience and Mind +188', 'capture', 'proven', 'aa-combat::morale::capture_reward', MOR)
add('0054bea0', 'Capture notification manager (B36520)', 'capture', 'seam', 'capture::AttemptWorld::capture_notification', None)
add('00585a20', 'Wall segments ownership transfer', 'capture', 'seam', 'capture::AttemptWorld::transfer_wall', None)
add('0056acb0', 'Component88 ownership transfer', 'capture', 'seam', 'capture::AttemptWorld::transfer_component88', None)
add('0058ee00', 'Actor statistics classification', 'capture', 'seam', 'capture::AttemptWorld::classification', P6CR)
add('0058f010', 'Capture statistics on new owner', 'capture', 'seam', 'capture::AttemptWorld::capture_statistics', None)
add('004d8450', 'KKC_GActor::ChangeKingdom (actor virtual +160): ownership change and its economy side effects', 'capture', 'not-ported', None, None,
    'Seam capture::AttemptWorld::set_allegiance; assigned to this area for the next round.')
add('004cdfd0', 'Commander experience income (shares commander::State::experience)', 'cadence', 'proven-other', 'aa-economy::tick::add_commander_experience', 'research/phase8-economy/native-tick-fixtures.json')
add('004e24b0', 'Faction/general ability affordability (Commander XP)', 'cadence', 'proven-other', 'aa-economy::transfers::can_afford_commander_ability', 'research/phase8-economy/native-transfer-fixtures.json')
add('0058e870', 'Statistic add (binary32)', 'experience', 'proven', 'aa-body::credit::increment_stat', P6CR)
# --- Experience / credit ------------------------------------------------------
add('004d57c0', 'Kill crediting (HandleKillCrediting)', 'experience', 'proven', 'aa-body::credit::credit_kill', P6CR)
add('00571820', 'Experience value ratio victim/source*0.5+0.5', 'experience', 'proven', 'aa-body::experience_amounts::ratio', EXP)
add('00571960', 'Kill experience (clamped)', 'experience', 'proven', 'aa-body::experience_amounts::clamped(Kill); credit.rs', EXP)
add('005719e0', 'Capture experience (clamped)', 'experience', 'proven', 'aa-body::experience_amounts::clamped(Capture)', EXP)
add('005718e0', 'Rout experience (clamped)', 'experience', 'proven', 'aa-body::experience_amounts::clamped(Rout)', EXP)
add('005718b0', 'Engagement experience (unclamped extended)', 'experience', 'proven', 'aa-body::experience_amounts::engagement', EXP)
add('004d01e0', 'Actor value (component sum)', 'experience', 'proven', 'aa-body::experience::actor_value', P6E)
add('00578c00', 'Regiment component value', 'experience', 'proven', 'aa-body::experience', P6E)
add('004ce770', 'Award experience (member/leader routing)', 'experience', 'proven', 'aa-body::experience::award', P6E)
add('00578710', 'Regiment member experience iteration', 'experience', 'proven', 'aa-body::experience::award', P6E)
add('00571a60', 'Experience apply and promotion', 'experience', 'proven', 'aa-body::promotion', 'research/phase7-promotion')
add('00579600', 'Engagement experience tick', 'experience', 'proven', 'aa-combat::morale::engagement_experience', MOR)
# --- Morale / suppression ----------------------------------------------------
for a, r, f in [('00578140', 'Morale state (0 exhausted, 1 wavering, 2 steady)', 'state'),
                ('005786d0', 'Morale gain (clamp to maximum, clear exhausted)', 'gain'),
                ('00579660', 'Morale loss with gates (mode 6, destructed, flag, immunity, Mind busy)', 'lose'),
                ('00579750', 'Signed morale modify (trigger effect)', 'modify'),
                ('00578f10', 'Slayer morale gain (kill)', 'slayer'),
                ('004ceb40', 'Slayer dispatch (killer membership organization)', 'slayer_organization'),
                ('00579010', 'Breaker morale gain (rout)', 'breaker'),
                ('00579ab0', 'Retreat morale penalty', 'retreat_penalty'),
                ('0057a0a0', 'Morale tick: replenish/supply bonus, engagement loss', 'tick'),
                ('005785d0', 'Attack efficiency approach to desired', 'attack_efficiency'),
                ('004ca050', 'MORALE_GAIN/MORALE_LOSS ability effect', 'effect_organization/apply_effect'),
                ('004ce6c0', 'Organization actor of an element', 'organization_actor'),
                ('004d3fa0', 'Rout handler: message, breaker morale and rout experience for attackers', 'rout')]:
    add(a, r, 'morale', 'proven', 'aa-combat::morale::' + f, REA if a == '004d3fa0' else MOR)
add('00501500', 'Modifier multiply +44 (morale replenish)', 'morale', 'proven', 'aa-body::properties::ModifierState::multiply', MOR)
add('00501510', 'Modifier divide +48 (engagement loss)', 'morale', 'proven', 'aa-body::properties::ModifierState::divide', MOR)
add('004d1080', 'Actor +104 bit2 test', 'morale', 'proven', 'morale::Actor::flags104', MOR)
add('004cfe90', 'Actor engaged (+118 nonnull)', 'morale', 'proven', 'morale::Actor::engaged', MOR)
add('004d64f0', 'Actor state transition (state 5 mind_exhausted)', 'morale', 'proven-other', 'aa-actors::state', None, 'Seam here; aa-actors owns it.')
add('004d49b0', 'Current state descriptor of a group', 'morale', 'seam', 'morale::World::state_group', None)
add('0051f9f0', 'CAI morale exhausted reaction', 'morale', 'seam', 'morale::World::mind_exhausted', None)
add('0051f920', 'CAI morale wavering reaction', 'morale', 'seam', 'morale::World::mind_wavering', None)
add('0057cd70', 'Organization member removal (MoraleDeathFactor loss, leader/primary reselection)', 'morale', 'not-ported', None, None)
add('0057c8e0', 'Organization TickMaintenance orchestration', 'morale', 'not-ported', None, None, 'Calls the proven 57A0A0, 579600, 5785D0, 578660.')
add('005788b0', 'Organization aggregate sums (+68/+6C)', 'morale', 'not-ported', None, None)
add('00579110', 'Organization supply-regeneration gate', 'morale', 'not-ported', None, None)
add('00543800', 'Trigger EffectActorModifyMorale (calls 579750)', 'morale', 'not-ported', None, None, 'Owned by aa-triggers.')
# --- ROE / reactive engagement / target acquisition --------------------------
add('00570f70', 'ROE primary damage record on membership component', 'target acquisition', 'proven', 'aa-combat::roe::record', ROE)
add('00570a70', 'ROE history prune', 'target acquisition', 'proven', 'aa-combat::roe::prune', ROE)
add('00570e30', 'ROE damage rate', 'target acquisition', 'proven', 'aa-combat::roe::rate', ROE)
add('00570ea0', 'ROE history push (vector)', 'target acquisition', 'proven', 'aa-combat::roe::record', ROE)
add('005707e0', 'ROE history vector reallocation', 'target acquisition', 'proven', 'Vec growth (capacity not semantic)', ROE)
add('00579490', 'Organization OnElementDamaged: reactive engage target', 'target acquisition', 'proven', 'aa-combat::engagement::element_damaged', RSP)
add('00578e90', 'Organization Mind free (+64/+7C)', 'target acquisition', 'proven', 'aa-combat::engagement', RSP)
add('00578470', 'Attacker within operating radius (definition +304)', 'target acquisition', 'proven', 'aa-combat::engagement::within_operating_radius', RSP)
add('004ce640', 'Element attached to organization', 'target acquisition', 'proven', 'engagement::World::element_attached', RSP)
add('00432ed0', 'Actor +100 bit4', 'target acquisition', 'proven', 'engagement', RSP)
add('005790f0', 'Actor +100 bit15', 'target acquisition', 'proven', 'engagement', RSP)
add('004d6190', 'Attacked-by list (BombardTargetTimeout) and attacker engagement', 'target acquisition', 'proven', 'aa-combat::reactions::attacked_by', REA)
add('004d3a70', 'Attacker engagement list and engaged message', 'target acquisition', 'proven', 'aa-combat::reactions::record_engagement', REA)
add('004d1b40', 'Attacked-by node constructor', 'target acquisition', 'proven', 'reactions::AttackedBy', REA)
add('00524c10', 'CAI Engage AddTarget', 'target acquisition', 'seam', 'engagement::World::add_engage_target', None, 'CAI state machine is Mind (aa-mind) territory.')
add('00523dd0', 'CAI Engage AddTarget (variant)', 'target acquisition', 'not-ported', None, None)
add('00523ea0', 'CAI Engage AddTarget (variant)', 'target acquisition', 'not-ported', None, None)
add('0051efb0', 'CAI Engage RemoveTarget', 'target acquisition', 'not-ported', None, None)
add('0051f040', 'CAI Engage ClearTargetList', 'target acquisition', 'not-ported', None, None)
add('005206a0', 'CAI Kill OnEnter', 'target acquisition', 'not-ported', None, None)
add('00527240', 'KillTargetIsHighestPriority evaluation', 'target acquisition', 'not-ported', None, None)
add('0052e830', 'EngageReactionTimeRange', 'target acquisition', 'not-ported', None, None)
add('00535240', 'UAI Kill state Think', 'target acquisition', 'not-ported', None, None)
add('005340c0', 'UAI decision using ROE rate and AbilityScoreAttackerBonus', 'target acquisition', 'not-ported', None, None, 'Consumer of the proven 570E30.')
add('00536840', 'UAI Kill/Engage enemy collision', 'target acquisition', 'proven-other', 'aa-mind::element_move_continuation', None)
add('00533c30', 'Moving target replan', 'target acquisition', 'proven-other', 'aa-mind::moving_target', 'research/phase7-moving-target')
add('00521e10', 'CAI retreat (MoraleRetreatPenalty consumer)', 'morale', 'not-ported', None, None)
add('00521240', 'CAI Rout Think', 'morale', 'not-ported', None, None, 'Its rout score 52A930 is proven in aa-mind::rout_score.')
# --- Weapon selection, cadence, range, projectiles ---------------------------
add('005307a0', 'Element ability decision result (weapon selection)', 'weapon selection', 'proven-other', 'aa-mind::ability_decision', None)
add('0052ed30', 'Element decision utility', 'weapon selection', 'proven-other', 'aa-mind::decision_score', None)
add('0052d820', 'Ability wait', 'weapon selection', 'proven-other', 'aa-mind::ability_motion', 'research/phase7-ability-motion')
add('0052ef40', 'Ability pursue', 'weapon selection', 'proven-other', 'aa-mind::ability_motion', 'research/phase7-ability-motion')
add('004be6c0', 'Local ability utility weights', 'weapon selection', 'proven', 'aa-combat::weights::maximum', P7P + '/native-weight-fixtures.json')
add('0057e670', 'Maximum other-mask utility', 'weapon selection', 'proven', 'aa-combat::weights::maximum_other', P7P + '/native-weight-fixtures.json')
add('004c8f20', 'Ability CanUse admission', 'weapon selection', 'proven', 'aa-combat::admission::can_use', P7A + '/native-admission-fixtures.json')
add('004c8740', 'Ability target eligibility', 'weapon selection', 'proven', 'aa-combat::admission::eligible', P7A + '/native-admission-fixtures.json')
add('004c8a40', 'Ability availability (summons, mana, recharge)', 'weapon selection', 'proven', 'aa-combat::admission::available', P7A + '/native-admission-fixtures.json')
add('004c89f0', 'Summoned list cleanup', 'weapon selection', 'proven', 'aa-combat::admission::available', P7A + '/native-admission-fixtures.json')
add('004c5840', 'Target classification flags', 'weapon selection', 'proven', 'aa-combat::admission::target_flags', P7A + '/native-admission-fixtures.json')
add('004c5a20', 'Target filter match', 'weapon selection', 'proven', 'aa-combat::admission::target_matches', P7A + '/native-admission-fixtures.json')
add('004c83e0', 'Recharge remaining (reload)', 'cadence', 'proven', 'aa-combat::admission::recharge_remaining', P7A + '/native-admission-fixtures.json')
add('004c8270', 'Effective range with fuzzy range', 'range', 'proven', 'aa-combat::admission::effective_range', 'research/phase7-ability-motion/native-range-fixtures.json')
add('004be910', 'Modified projectile/bombard range', 'range', 'proven', 'aa-combat::admission (range_value)', 'research/phase7-ability-motion/native-range-fixtures.json')
add('0057ec80', 'Maximum enabled ability range', 'range', 'proven', 'aa-combat::admission::maximum_range', 'research/phase7-moving-target/native-maximum-fixtures.json')
for a, r in [('0046f100', 'Shape range test dispatcher'), ('0046eb70', 'Point/circle/box range'), ('0046e2d0', 'Rectangle inflate range'),
             ('004ee400', 'Actor range descriptor'), ('004ee800', 'Actor/actor range'), ('004ee7c0', 'Point/actor range'),
             ('004ea030', 'Ordinary target shape getter')]:
    add(a, r, 'range', 'proven-other', 'aa-spatial::range', 'research/phase7-range/native-range.json')
for a, r in [('004ea0f0', 'Soft range descriptor'), ('004ea020', 'Ordinary soft shape getter')]:
    add(a, r, 'soft range', 'proven-other', 'aa-spatial::soft_range', 'research/phase7-soft-range/native-soft-range.json')
add('0052fe60', 'Arrival soft-range branch', 'soft range', 'fragment', 'aa-spatial::soft_range::source_first_in_range', 'research/phase7-soft-range/native-soft-range.json')
add('005774a0', 'Mover soft-range fragment', 'soft range', 'fragment', 'aa-spatial::soft_range::source_receiver_in_range', 'research/phase7-soft-range/native-mover-range.json')
for a, r, f in [('0057f290', 'Performer start attack', 'start'), ('0057ed20', 'Performer begin animation/use deadlines', 'begin'),
                ('0057e360', 'Performer context fill', 'fill_context'), ('0057dc90', 'Performer finish', 'finish'),
                ('0057e8c0', 'Performer use frame', 'use_frame'), ('0057f040', 'Performer timed event (use/end/reload)', 'timed_event')]:
    add(a, r, 'cadence', 'proven', 'aa-combat::performer::' + f, P7P + '/native-performer-fixtures.json')
add('0057f4d0', 'Performer constructor', 'cadence', 'proven', 'aa-combat::constructors::create', P7P + '/native-constructor-fixtures.json')
add('0057db90', 'Performer InitCreate', 'cadence', 'proven', 'aa-combat::constructors::init_create', P7P + '/native-constructor-fixtures.json')
add('0057e030', 'Performer ability delivery (impulse, mana)', 'hit resolution', 'proven', 'aa-combat::execution::dispatch_from_performer', P7A + '/native-dispatch-fixtures.json')
add('0057dd90', 'Performer target/turret intention', 'cadence', 'seam', 'performer::World::track_target', None)
add('004c8650', 'Ability instance constructor', 'cadence', 'proven', 'aa-combat::ability::Ability::new', P7P + '/native-constructor-fixtures.json')
add('004c86d0', 'Ability instance constructor (local)', 'cadence', 'proven', 'aa-combat::ability::Ability::new', P7P + '/native-constructor-fixtures.json')
add('004c8300', 'Ability context initialization', 'hit resolution', 'proven', 'aa-combat::performer::Context::new', REA)
add('004c9550', 'Ability use: phases, payment, area, projectile, attenuation, effects', 'hit resolution', 'proven', 'aa-combat::execution::use_ability', P7A + '/native-execution-fixtures.json')
add('004c9c90', 'Area target traversal and max-target sorting', 'area damage', 'proven', 'aa-combat::execution::use_area', P7A + '/native-execution-fixtures.json')
add('004c9eb0', 'Ability try-use (CanUse then Use at now)', 'hit resolution', 'proven', 'aa-combat::reactions::try_use', REA)
add('00501770', 'Modifier OnAttacked defender abilities', 'hit resolution', 'proven', 'aa-combat::reactions::defender_abilities', REA)
add('004c8cb0', 'Projectile launch (factory handoff)', 'projectiles', 'proven', 'aa-combat::launch::launch', P7A + '/native-launch-fixtures.json')
add('004be780', 'Projectile spawn position', 'projectiles', 'proven', 'aa-combat::launch::spawn_position', P7A + '/native-launch-fixtures.json')
add('00580bc0', 'Projectile start (duration, prediction)', 'projectiles', 'proven', 'aa-combat::projectile::start', P7A + '/native-projectile-fixtures.json')
add('00580920', 'Projectile advance (travel, arrival)', 'projectiles', 'proven', 'aa-combat::projectile::advance', P7A + '/native-projectile-fixtures.json')
add('00580660', 'Projectile impact (phase2 delivery)', 'projectiles', 'proven', 'aa-combat::projectile::impact', P7A + '/native-projectile-fixtures.json')
add('00580ed0', 'Projectile constructor', 'projectiles', 'proven', 'aa-combat::projectile::Projectile::new', P7P + '/native-constructor-fixtures.json')
add('004c8140', 'Ability copy into projectile', 'projectiles', 'proven', 'aa-combat::projectile::start', P7A + '/native-projectile-fixtures.json')
for a, r, f in [('004eeee0', 'Target hit position', 'hit_position'), ('004eee60', 'Ground hit position', 'ground_position'),
                ('0046e150', 'Shape projection', 'shape_projection'), ('0046e4f0', 'Placed shape projection', 'placed_projection')]:
    add(a, r, 'projectiles', 'proven', 'aa-combat::surface::' + f, 'research/phase7-surface/native-surface-fixtures.json')
for a, r in [('004e2590', 'Ability payment'), ('004cdd90', 'Faction XP debit'), ('004e1690', 'Stockpile debit'), ('004bec50', 'placeholder')]:
    if r != 'placeholder':
        add(a, r, 'cadence', 'proven', 'aa-combat::payment::pay', 'research/phase7-payment/native-payment-fixtures.json')
for a, r in [('004c8500', 'Ability FX request'), ('004294e0', 'FX manager request'), ('00428570', 'FX instance constructor')]:
    add(a, r, 'presentation', 'proven', 'aa-combat::fx::emit', 'research/phase7-fx/native-fx-fixtures.json')
for a, r in [('004e0f00', 'Commander replacement'), ('004ce1d0', 'Commander constructor'), ('004ce0b0', 'Commander load'), ('004c8b40', 'Ability load')]:
    add(a, r, 'cadence', 'proven', 'aa-combat::commander', 'research/phase7-commander/native-commander-fixtures.json')
# --- Combat value / AI evaluation (not ported) --------------------------------
add('0045e090', 'Combat value computation (CombatValue* settings)', 'AI evaluation', 'not-ported', None, None)
add('004bb590', 'Combat value CSV dump (diagnostic)', 'AI evaluation', 'not-ported', None, None, 'Diagnostic only.')
add('004bbf90', 'Mean attack/defense values', 'AI evaluation', 'not-ported', None, None)

# --- Round 2 (lead assignment 2026-10-14) -------------------------------------
ORG = P8 + '/native-org-tick-fixtures.json'
REM = P8 + '/native-member-removal-fixtures.json'
SIE = P8 + '/native-siege-fixtures.json'
DEN = P8 + '/native-denizen-fixtures.json'
OWN = P8 + '/native-change-kingdom-fixtures.json'
NUM = P8 + '/native-damage-number-fixtures.json'
TICK = 'research/phase8-tick/native-removal-request.json'
OVERRIDE = {
    '004f65b0': ('proven-other', 'aa-world::World::transition', TICK,
                 'One drain: aa-world owns 4F65B0; phase8-combat drain fixtures are cross-evidence.'),
    '004f4c40': ('proven-other', 'aa-world (removal path)', TICK, None),
    '004d6660': ('proven', 'aa-combat::siege::refresh_under_siege', SIE, 'Sets state 20 under_siege.'),
    '0056ae70': ('proven', 'aa-combat::denizen::on_damaged', DEN, None),
    '00569a80': ('proven', 'aa-combat::denizen::intruder_in_range', DEN, None),
    '00568d80': ('proven', 'aa-combat::denizen::on_intruder', DEN, None),
    '0057c8e0': ('proven', 'aa-combat::organization::tick_maintenance', ORG, None),
    '005788b0': ('proven', 'aa-combat::organization::combat_value', ORG, None),
    '00579110': ('proven', 'aa-combat::organization::regeneration_gate', ORG, None),
    '0057cd70': ('proven', 'aa-combat::organization::remove_member', REM, None),
    '004d8450': ('proven', 'aa-combat::ownership::change_kingdom', OWN,
                 'Kingdom add/remove actor and recruit singles bind to aa-economy::kingdom_registry.'),
    '0050d0b0': ('proven', 'aa-combat::presentation::queue_damage_number', NUM, None),
    '0045e090': ('not-ported', None, None,
                 'Blocked: needs an exact MSVC CRT pow (96D890: SSE2 path 977A10 or x87 path 96D8ED); '
                 'Unicorn transcendental results are not hardware ground truth. AI evaluation input.'),
    '004bbf90': ('not-ported', None, None, 'Blocked with 45E090 (combat-value means).'),
    '004d7960': ('seam', 'organization::RemovalWorld::begin_owner_death; aa-body Host', None, None),
}
AI_OWNED = {'00535240', '005340c0', '005206a0', '00523dd0', '00523ea0', '0051efb0', '0051f040',
            '00527240', '0052e830', '00524c10', '0051f9f0', '0051f920', '00521e10', '00521240'}
for a, r, area, st, rust, ora in [
    ('004d0440', 'Recently damaged (UnderSiegeRecentlyDamagedSeconds)', 'damage', 'proven', 'aa-combat::siege::recently_damaged', SIE),
    ('004364c0', 'Organization member iterator (attached-only flag)', 'morale', 'proven', 'executed in denizen/removal oracles', DEN),
    ('004d0380', 'Relation predicate (+7C/+60, flags8 bit11, virtual +84)', 'target acquisition', 'proven', 'aa-combat::denizen::on_intruder', DEN),
    ('00517f30', 'GTID list contains', 'target acquisition', 'proven', 'aa-combat::denizen::on_intruder', DEN),
    ('004d1970', 'GTID list head insert (4D0900 node)', 'target acquisition', 'proven', 'aa-combat::denizen::on_intruder', DEN),
    ('004cf1b0', 'Actor GTID handle', 'target acquisition', 'proven', 'executed in denizen/presentation oracles', DEN),
    ('0057dc40', 'Member health-weighted combat value', 'morale', 'proven', 'aa-combat::organization::combat_value', ORG),
    ('004d7da0', 'State 40 targeted', 'morale', 'proven', 'aa-combat::organization::targeted_state', ORG),
    ('004d7ce0', 'State 19 out_of_supply sync', 'morale', 'proven', 'aa-combat::organization::supply_state', ORG),
    ('0057c800', 'Follow leader', 'morale', 'proven', 'aa-combat::organization::follow_leader', ORG),
    ('004515f0', 'XY equality', 'morale', 'proven', 'aa-combat::organization::follow_leader', ORG),
    ('0058b150', 'Watched target finished (+40 == +3C)', 'morale', 'proven', 'organization::World::watched_target', ORG),
    ('005147c0', 'Marker gate (word8 != -1)', 'morale', 'proven', 'aa-combat::organization::tick_maintenance', ORG),
    ('00578540', 'Owner-granted property detach', 'morale', 'proven', 'aa-combat::organization::remove_member', REM),
    ('0045ec60', 'Definition +2D8 property list', 'morale', 'proven', 'organization::RemovalWorld', REM),
    ('0045d580', 'Definition morale value +2C0', 'morale', 'proven', 'organization::RemovalWorld::morale_value', REM),
    ('004d1050', 'Actor +100 bit26 / +104 bit4', 'morale', 'proven', 'aa-combat::organization::remove_member', REM),
    ('0050d070', 'Damage-number vector append', 'presentation', 'proven', 'aa-combat::presentation', NUM),
    ('0057b5a0', 'Organization regeneration', 'morale', 'seam', 'organization::World::regenerate', None),
    ('0057aeb0', 'Organization regroup attempt', 'morale', 'seam', 'organization::World::regroup_check', None),
    ('004d0a80', 'In-supply predicate', 'morale', 'seam', 'organization::World::in_supply', None),
    ('0045d520', 'Entrench delay (definition +2EC, / modifier +40 via 5014D0)', 'morale', 'seam', 'organization::World::entrench_time', None),
    ('00515f40', 'Organization flare', 'presentation', 'seam', 'organization::World::flare', None),
    ('0057c680', 'Move organization to leader', 'morale', 'seam', 'organization::World::move_to_leader', None),
    ('0057a640', 'Recompute +D8 minimum', 'morale', 'seam', 'organization::RemovalWorld::recompute_6c', None),
    ('005797e0', 'Slot refresh', 'morale', 'seam', 'organization::RemovalWorld::slot_refresh', None),
    ('00570670', 'Membership detach', 'morale', 'seam', 'organization::RemovalWorld::detach', None),
    ('004cef70', 'Property removal by source (581810)', 'morale', 'seam', 'organization::RemovalWorld::remove_property', None),
    ('00585160', 'Wall component predicate', 'damage', 'seam', 'siege::World::wall_predicate', None),
    ('00569540', 'Denizen predicate', 'damage', 'seam', 'siege::World::component88_predicate', None),
    ('00586370', 'Wall component update', 'damage', 'seam', 'siege::World::wall_update', None),
    ('004e44e0', 'GKingdom::RemoveActor', 'capture', 'proven-other', 'aa-economy::kingdom_registry::remove_actor', None),
    ('004e6180', 'GKingdom::AddActor', 'capture', 'proven-other', 'aa-economy::kingdom_registry::add_actor', None),
    ('004e2a60', 'Kingdom recruit single find', 'capture', 'proven-other', 'aa-economy::kingdom_registry::RecruitItemTable::find_single', None),
    ('004e2cb0', 'Kingdom recruit single remove', 'capture', 'proven-other', 'aa-economy::kingdom_registry::RecruitItemTable::remove_single', None),
    ('004e4c30', 'Kingdom recruit single add', 'capture', 'proven-other', 'aa-economy::kingdom_registry::RecruitItemTable::add_single', None),
    ('0058d950', 'Relation statistic add', 'experience', 'proven-other', 'aa-economy::statistics::RelationStatistics::add', None),
    ('004f6760', 'World removal request', 'death', 'proven-other', 'aa-world::World::request_removal', TICK),
]:
    add(a, r, area, st, rust, ora)


# --- Round 3 (lead assignment 2026-10-14): CRT pow, combat value, rest -------
POW = P8 + '/native-crt-pow-fixtures.json'
CV = P8 + '/native-combat-value-fixtures.json'
STT = P8 + '/native-statistics-tracking-fixtures.json'
REG = P8 + '/native-regeneration-fixtures.json'
ENT = P8 + '/native-entrench-delay-fixtures.json'
SPD = P8 + '/native-org-speed-fixtures.json'
PRW = P8 + '/native-property-wrappers-fixtures.json'
OVERRIDE.update({
    '0045e090': ('proven', 'aa-combat::combat_value::ability_value', CV,
                 'Both CRT pow calls run natively in the oracle; the port uses aa-numeric::crt_pow.'),
    '004bbf90': ('proven', 'aa-combat::combat_value::mean_values', CV,
                 'Empty trimmed ranges (native NaN means) are rejected explicitly.'),
    '004bb590': ('proven', 'aa-combat::combat_value::actor_data_values', CV,
                 'CSV open/header/close are DefinitionWorld seams (diagnostic file output).'),
    '004cef70': ('proven', 'aa-body::property_api::remove_property', PRW,
                 '0x581810 body is aa-body property_effects::remove (phase7 fixtures).'),
    '0045d520': ('proven', 'aa-combat::organization::entrench_delay', ENT, None),
    '0057b5a0': ('proven', 'aa-combat::regeneration::regenerate', REG,
                 'Member spawn 57AD30, heal 562BA0 and presentation 50C380 are world services.'),
    '0057aeb0': ('proven', 'aa-combat::regeneration::regroup', REG, None),
    '0057a640': ('proven', 'aa-combat::organization::recompute_speed', SPD, None),
    '0058f010': ('proven', 'aa-combat::statistics_tracking::track', STT,
                 'On aa-economy Statistics (add_score); class word actor+13C.'),
    '0058ee00': ('proven-other', 'aa-economy::statistics::structure_class', STT,
                 'Executed natively inside the 58F010 oracle.'),
    '005797e0': ('proven-other', 'aa-movement::formation (ComputeSlotPosition)', None, None),
    '0057c680': ('proven-other', 'aa-actors::element_position::set_position', None, None),
    '004d0a80': ('proven-other', 'aa-economy::supply::in_supply', None, None),
    '004d49b0': ('proven-other', 'aa-actors::state::group', None, None),
    '00521240': ('proven-other', 'aa-mind::cai_rout', None, None),
    '0051f920': ('proven-other', 'aa-mind::cai_reactions (wavering)', None, None),
    '0051f9f0': ('proven-other', 'aa-mind::cai_reactions (exhausted)', None, None),
    '00521e10': ('proven-other', 'aa-mind::action_mask::regiment', None,
                 'aa-mind identifies 521E10 as the regiment command mask (MoraleRetreatPenalty context).'),
})
for a, r, area, st, rust, ora in [
    ('0096d890', '_CIpow dispatch (SSE2 flag B4B340, MXCSR, x87 CW)', 'AI evaluation', 'proven',
     'aa-numeric::crt_pow::select_path / ci_pow', POW),
    ('00977a10', '_CIpow SSE2 entry (operand stores)', 'AI evaluation', 'proven', 'aa-numeric::crt_pow::ci_pow', POW),
    ('00977a29', 'CRT pow SSE2 body (tables AD0BA0..AD44C8)', 'AI evaluation', 'proven',
     'aa-numeric::crt_pow::pow_sse2 (generated by lift_crt_pow.py)', POW),
    ('0096d8ed', '_CIpow x87 body', 'AI evaluation', 'seam', 'aa-numeric::crt_pow::CrtPowError::X87PathUnsupported', None),
    ('00977111', '___libm_error_support (default _matherr 84D110)', 'AI evaluation', 'proven',
     'aa-numeric::crt_pow::libm_error_support', POW),
    ('004618b0', 'Thing definition combat value (+1E4/+30C/+36C, recursive)', 'AI evaluation', 'proven',
     'aa-combat::combat_value::definition_value', CV),
    ('0057e430', 'Performer combat value (+60)', 'AI evaluation', 'proven', 'aa-combat::combat_value::performer_value', CV),
    ('004be3f0', 'Ability power getter (501A30/501AA0)', 'AI evaluation', 'proven', 'aa-combat::combat_value', CV),
    ('004be680', 'Local ability first entry +10', 'AI evaluation', 'proven', 'aa-combat::combat_value', CV),
    ('004be4b0', 'Local recharge getter (501560)', 'AI evaluation', 'proven', 'aa-combat::combat_value', CV),
    ('0045e6d0', 'Thing extent value (+468/+478/2.0001)', 'AI evaluation', 'proven', 'aa-combat::combat_value::Extent::value', CV),
    ('00869d30', 'Bisection with comparator (descending 4BB020)', 'AI evaluation', 'proven', 'aa-combat::combat_value (insert)', CV),
    ('004bb850', 'Vector insert at index', 'AI evaluation', 'proven', 'aa-combat::combat_value (insert)', CV),
    ('00501a30', 'Modifier melee power (+8, +4, clamp +1A4)', 'AI evaluation', 'proven', 'aa-body::properties::ModifierState::power', CV),
    ('00501aa0', 'Modifier ranged power (+C, +4, clamp +1A4)', 'AI evaluation', 'proven', 'aa-body::properties::ModifierState::power', CV),
    ('0058efe0', 'Statistics untrack', 'capture', 'proven', 'aa-combat::statistics_tracking::untrack', STT),
    ('0058ea80', 'Statistics classified predicate (+80, !+D8, +A4)', 'capture', 'proven', 'aa-combat::statistics_tracking::classified', STT),
    ('0058eba0', 'Statistics leave class', 'capture', 'proven', 'aa-combat::statistics_tracking', STT),
    ('0058e8b0', 'Statistics enter class', 'capture', 'proven', 'aa-combat::statistics_tracking', STT),
    ('0058e890', 'Statistics score maximum', 'capture', 'proven', 'aa-combat::statistics_tracking', STT),
    ('004cef10', 'Actor property admission (apply property)', 'damage', 'proven',
     'aa-body::property_api::apply_property (property_admission::add)', 'research/phase7-properties/native-admission.json'),
    ('005617c0', 'Body full health predicate', 'morale', 'proven', 'aa-combat::regeneration::BodyHealth::full', REG),
    ('0045d5b0', 'Definition regeneration +2CC (5014E0)', 'morale', 'proven', 'aa-combat::regeneration', REG),
    ('0057ad30', 'Organization member spawn at slot (4FBF40)', 'morale', 'seam', 'regeneration::World::spawn', None),
    ('0050c380', 'Regeneration presentation', 'presentation', 'seam', 'regeneration::World::present_regeneration', None),
]:
    add(a, r, area, st, rust, ora)


# --- Round 4 (lead assignment 2026-10-15): ability definitions, BeginDeath,
# walls/denizens, membership, member spawn, turret intention ----------------
BD = P8 + '/native-begin-death-fixtures.json'
WALL = P8 + '/native-wall-fixtures.json'
SPAWN = P8 + '/native-member-spawn-fixtures.json'
TUR = P8 + '/native-turret-intention-fixtures.json'
ABL = P8 + '/native-ability-effect-fixtures.json'
OVERRIDE.update({
    '004d7960': ('proven', 'aa-combat::begin_death::begin_death', BD,
                 'Orchestration ported; credit, zone release, announcements, component deaths, wall member, '
                 'StartDying and removal stay begin_death::World services.'),
    '00585a20': ('proven', 'aa-combat::walls::transfer_wall', WALL, None),
    '0056acb0': ('proven', 'aa-combat::walls::transfer_component_88', WALL, None),
    '00570670': ('proven', 'aa-combat::organization::detach_membership', WALL, None),
    '00585160': ('proven', 'aa-combat::walls::wall_complete', WALL, None),
    '00569540': ('proven', 'aa-combat::walls::intruder_near_wall', WALL,
                 '585280 within-wall geometry on the wall actor +98 is an IntruderWorld seam (wall/geometry owner).'),
    '00586370': ('proven', 'aa-combat::walls::propagate_under_siege', WALL, None),
    '0057ad30': ('proven', 'aa-combat::regeneration::spawn_member', SPAWN,
                 'Actor creation 4FBF40 is the SpawnWorld::create_actor seam (actor-creation owner).'),
    '0057dd90': ('proven', 'aa-combat::performer::track_target', TUR,
                 'aa-mind uai_kill/element_move seams may bind this too.'),
    '0054bea0': ('proven-other', 'aa-triggers::predicates::queue_captured', None,
                 'Owner aa-triggers: trigger manager B36520 +50 queue prepend of a 54B820 Captured payload '
                 '(+4 captured +14, +8 source +14, +C source if virtual +A8 else virtual +108, its +14). Not combat.'),
    '00515f40': ('seam', 'organization::World::flare', None,
                 'Owner zone/detection solver (Sim/Zones.cpp; phase8-movement cluster Solver/ZoneSolver.cpp): '
                 'KKC_Zone::Move by world position (552640 clamp, cell shift [B36534]+30 +4, 515970). Not combat.'),
})
for a, r, area, st, rust, ora in [
    ('00585ae0', 'Wall actor flag 13 refresh (517FA0)', 'damage', 'proven', 'aa-combat::walls::refresh_flag13', WALL),
    ('00569300', 'Component88 intruder list front cleanup', 'damage', 'proven', 'aa-combat::walls::intruder_near_wall', WALL),
    ('0057dea0', 'Performer turret intention clear', 'cadence', 'proven', 'aa-combat::performer::clear_turret_intentions', TUR),
    ('004c81e0', 'Ability turret index ([+8]+54)', 'cadence', 'proven', 'performer::TrackWorld::ability_turret', TUR),
    ('0045dd30', 'Definition turret +1F8[i] byte +31', 'cadence', 'proven', 'performer::TrackWorld::turret_tracks_target', TUR),
    ('004bef80', 'Ability effect dispatch (type, 4CA880 construction, LoadData)', 'ability definition', 'proven',
     'aa-rules::abilities::native_ability_effects', ABL),
    ('004bee20', 'Ability flags and main power from effects', 'ability definition', 'proven',
     'aa-rules::abilities (native_definition_effects)', ABL),
    ('004ca880', 'Ability effect construction by kind', 'ability definition', 'proven', 'aa-rules::abilities::LoadedEffect', ABL),
    ('00497af0', 'Ability effect type enum registration', 'ability definition', 'proven', 'aa-rules::abilities::effect_kind', ABL),
    ('00497a40', 'FXC type enum registration', 'ability definition', 'proven', 'aa-rules::abilities::FXC_TYPES', ABL),
    ('004c1060', 'Modifier type enum registration', 'ability definition', 'proven', 'aa-rules::abilities::MODIFIER_TYPES', ABL),
    ('004c11e0', 'Modifier attribute enum registration', 'ability definition', 'proven', 'aa-rules::terrain_speed::ATTRIBUTES', ABL),
    ('004cc1d0', 'Heal type enum (table constructor)', 'ability definition', 'proven', 'aa-rules::abilities::heal_type', ABL),
    ('004ca010', 'Morale effect LoadData (+14 amount)', 'ability definition', 'proven', 'aa-rules::abilities::EffectData::Morale', ABL),
    ('004cab50', 'Heal effect LoadData (+14 amount, +18 heal type)', 'ability definition', 'proven', 'aa-rules::abilities::EffectData::Heal', ABL),
    ('004ccc70', 'Damage effect LoadData (+14 damage type, +18 amount)', 'ability definition', 'proven', 'aa-rules::abilities::EffectData::Damage', ABL),
    ('004cbbf0', 'Enchantment effect LoadData (EnchantmentProperty child)', 'ability definition', 'proven',
     'aa-rules::abilities::EffectData::Enchantment', ABL),
    ('004c9fd0', 'Base effect LoadData (CAPTURE, DESCRIBE_ECONOMY)', 'ability definition', 'proven', 'aa-rules::abilities::EffectData::Plain', ABL),
    ('004ca190', 'Steal resources effect LoadData (ResourcesAmount)', 'ability definition', 'proven',
     'aa-rules::abilities::EffectData::StealResources', ABL),
    ('004cc890', 'Summon effect LoadData (object_ids, deferred 4CC730)', 'ability definition', 'not-ported', None, None),
    ('004c4f90', 'EnchantmentProperty LoadData (technology, duration, type, permanent VFX)', 'ability definition', 'proven',
     'aa-rules::abilities::EnchantmentProperty', ABL),
    ('004c4030', 'Property modifier list loading (FLAG/ADD/RESIST_DAMAGE/PERIODIC_DAMAGE)', 'ability definition', 'proven',
     'aa-rules::abilities::EnchantmentModifier', ABL),
    ('004fec70', 'ADD modifier LoadData (amount, attribute < 13, repair)', 'ability definition', 'proven', 'aa-rules::abilities::add_modifier', ABL),
    ('00501350', 'PERIODIC_DAMAGE modifier LoadData', 'ability definition', 'proven',
     'aa-rules::abilities::EnchantmentModifier::PeriodicDamage', ABL),
    ('00428a30', 'FXC permanent-VFX search (ChainImmediate/Chain recursion)', 'ability definition', 'proven',
     'aa-rules::abilities::permanent_vfx_present', ABL),
    ('0042c5e0', 'CreateVFX LoadData (+64 = duration != 0)', 'ability definition', 'proven', 'aa-rules::abilities::FxcFacts', ABL),
    ('00494f30', 'VFX LoadData lifetime (+24 permanent)', 'ability definition', 'proven', 'aa-rules::abilities::FxcFacts', ABL),
    ('004bf990', 'Ability definition LoadData (required_properties tokens at 4C00AB)', 'ability definition', 'fragment',
     'aa-rules::abilities::required_property_tokens / LoadedAbility::required_properties',
     'aa-rules lib test original_ability_effects_resolve (Data.rwd + v14 catalog)'),
]:
    add(a, r, area, st, rust, ora)


# --- Round 5 (lead assignment 2026-10-15): special operations (Commander /
# Faction abilities), SUMMON resolution, modifier kinds, required_properties,
# within-wall geometry. Statuses are updated by the round's ports; the data
# census is special-ops-census.json (special_ops_census.py). ------------------
SPO = 'special operations'
R5 = {}


def r5(address, role, area, status, rust=None, oracle=None, note=None):
    R5[address] = True
    E[:] = [e for e in E if e['address'] != address]
    add(address, role, area, status, rust, oracle, note)


# Activation: order -> Faction::UseAbility -> admission -> use.
r5('0061e190', 'KKC_FactionUseAbilityOrder execute (TGC_RequestDecreeOrder)', SPO, 'seam',
   'commander::use_ability caller', None, 'Owner: orders/net agent (Orders.h decree path).')
r5('004cde00', 'KKC_Faction::UseAbility: find ability, context kind 3, admission, use', SPO, 'not-ported')
r5('004c9ef0', 'Ability use at world time ([B36534]+E0) -> 4C9550', SPO, 'not-ported')
r5('006073c0', 'Special-op button availability (UI; calls 4E24B0)', SPO, 'seam', None, None, 'UI owner.')
# Effect Apply virtuals (vtable +8, called by 4C9550 / 4C9C90).
r5('004ca050', 'MoraleEffect Apply (4C8330, 4CE6C0, 5786D0/579660)', SPO, 'proven',
   'aa-combat::morale::effect_organization + apply_effect', MOR)
r5('004ca400', 'DamageEffect Apply (hit)', SPO, 'proven', 'aa-combat::execution (crate::hit)',
   P7A + '/native-execution-fixtures.json')
r5('004ca0b0', 'HealEffect Apply (4CF110/501B70 modifier, 562BA0)', SPO, 'not-ported')
r5('004ca130', 'EnchantmentEffect Apply (581280 reject, 581700 add)', SPO, 'not-ported')
r5('004ca260', 'CaptureEffect Apply (4D5230 with record kind 1 reason 7)', SPO, 'not-ported')
r5('004ca680', 'StealResourcesEffect Apply (target kingdom, min(stock, amount))', SPO, 'proven-other',
   'aa-economy::transfers::steal_resources', None)
r5('004cc050', 'DescribeEconomyEffect Apply (4E7200 text, 40BF70 message 0xF)', SPO, 'not-ported')
r5('004cca70', 'SummonEffect Apply: position/facing, object choice, open position', SPO, 'not-ported')
r5('004cb110', 'SummonEffect SummonTransportedActor', SPO, 'not-ported')
r5('004cc470', 'SummonEffect SummonActor', SPO, 'not-ported')
r5('004cc2c0', 'SummonEffect SummonObject', SPO, 'not-ported')
r5('004cb5b0', 'Summon failure message (ability_effect_summon_failed)', SPO, 'not-ported')
r5('00554b30', 'Open-position search (radius 12)', SPO, 'seam', None, None, 'Placement/map owner.')
r5('004e7200', 'Kingdom economy description text', SPO, 'seam', None, None, 'Economy/UI owner.')
r5('0040bf70', 'Player message post (B399B0)', SPO, 'seam', None, None, 'UI/message owner.')
# Property side of enchantments and buffs.
r5('00581280', 'Properties: find property', SPO, 'not-ported')
r5('00581700', 'Properties: add property (source, allegiance)', SPO, 'not-ported')
r5('00501b70', 'Modifier heal amount', SPO, 'not-ported')
# Definition loading for special operations.
r5('0048bc25', 'FactionData ability list (4BF990 with actor 0)', SPO, 'fragment',
   'aa-rules::commander::native_commander_constructor_inputs', None)
r5('004cc890', 'SummonEffect LoadData (object ids, property, controllable, unique flags)', SPO, 'not-ported')
r5('004caa00', 'Summon deferred-resolution object (473AD0 registration)', SPO, 'not-ported')
r5('004cc730', 'Summon deferred resolution (427BD0, transport/property validation)', SPO, 'not-ported')
r5('004fed70', 'MULTIPLY modifier LoadData', SPO, 'not-ported')
r5('005010a0', 'ENCHANTMENT_IMMUNITY modifier LoadData', SPO, 'not-ported')
r5('00501150', 'ABILITY_EFFECT modifier LoadData (nested effect via 4CA880)', SPO, 'not-ported')
r5('004fee90', 'PERIODIC_HEAL modifier LoadData', SPO, 'not-ported')
r5('00501410', 'REVERSE_ABILITY modifier LoadData', SPO, 'not-ported')
r5('004feec0', 'RESOURCE_PRODUCTION modifier LoadData', SPO, 'not-ported')
r5('004feaf0', 'Modifier base LoadData (type +8)', SPO, 'not-ported')
r5('004c42d0', 'ModifierProperty LoadData (required_properties via 473AD0)', SPO, 'not-ported')
r5('00585280', 'Wall within-distance geometry (component88 intruders)', 'damage', 'not-ported')

# Round 5 results (statuses after the ports).
SOE = P8 + '/native-special-ops-effects-fixtures.json'
SUM = P8 + '/native-summon-fixtures.json'
SOL = P8 + '/native-special-ops-loading-fixtures.json'
GEO = P8 + '/native-wall-geometry-fixtures.json'
MEX = P8 + '/native-member-extras-fixtures.json'
FACTION_TEST = 'aa-rules lib test original_special_operations_resolve (24 Factions, 88 abilities)'
OVERRIDE.update({
    '004cde00': ('proven', 'aa-combat::special_ops::use_ability', SOE, None),
    '004c9ef0': ('proven', 'aa-combat::special_ops::use_ability (FactionWorld::use_ability at world time)', SOE, None),
    '004ca0b0': ('proven', 'aa-combat::special_ops::apply_heal', SOE, None),
    '004ca130': ('proven', 'aa-combat::special_ops::apply_enchantment', SOE, None),
    '004ca260': ('proven', 'aa-combat::special_ops::apply_capture (CAPTURE_EFFECT_RECORD)', SOE,
                 '4D5230 itself is capture::attempt (round 2).'),
    '004cc050': ('proven', 'aa-combat::special_ops::apply_describe_economy', SOE,
                 'Text/localisation/post 4E7200/4B9F40/40BF70 stay the DescribeWorld seam (UI/economy owners).'),
    '004cca70': ('proven', 'aa-combat::summon::apply', SUM, None),
    '004cb110': ('proven', 'aa-combat::summon (summon_transported, transport_entry)', SUM,
                 'Transport payload 58BA90/4C2290 and the move command are SummonWorld seams (transport/orders owners).'),
    '004cc470': ('proven', 'aa-combat::summon (summon_actor)', SUM,
                 'Creation 4FA030/4FA150 and the attack command 4CA330/4D9B60 are SummonWorld seams.'),
    '004cc2c0': ('proven', 'aa-combat::summon (summon_object)', SUM, 'Creation 4F9C10 is a SummonWorld seam.'),
    '004cb5b0': ('seam', 'summon::SummonWorld::summon_failed', None, 'UI message 0x11 (ability_effect_summon_failed).'),
    '00581280': ('proven', 'aa-body::property_api::find_instance (via find_actor_property)', SOE, None),
    '00581700': ('proven', 'aa-combat::special_ops::add_enchantment', SOE,
                 '581630 insertion is aa-body property_effects::insert_with (proven).'),
    '00501b70': ('proven', 'aa-combat::special_ops::heal_modifier', SOE, None),
    '0048bc25': ('fragment', 'aa-rules::abilities::native_faction_ability_effects', FACTION_TEST, None),
    '004cc890': ('proven', 'aa-rules::abilities::SummonEffectData (native_summon_effect)', SOL, None),
    '004caa00': ('proven', 'aa-rules::abilities (deferred object, executed in the oracle)', SOL, None),
    '004cc730': ('proven', 'aa-rules::abilities::summon_resolution_checks', SOL,
                 'A recruit actor with a non-flying transport depends on the deferred-list order: explicit Unsupported.'),
    '004fed70': ('proven', 'aa-rules::abilities::multiply_modifier', SOL, None),
    '005010a0': ('proven', 'aa-rules::abilities::EnchantmentModifier::EnchantmentImmunity', SOL, None),
    '00501150': ('proven', 'aa-rules::abilities::EnchantmentModifier::AbilityEffect (ability_effect_kind)', SOL, None),
    '004fee90': ('proven', 'aa-rules::abilities::EnchantmentModifier::PeriodicHeal', SOL, None),
    '00501410': ('proven', 'aa-rules::abilities::EnchantmentModifier::ReverseAbility', SOL, None),
    '004feec0': ('proven', 'aa-rules::abilities::resource_production', SOL, None),
    '004feaf0': ('proven', 'aa-rules::abilities (modifier type at +8)', SOL, None),
    '004c42d0': ('fragment', 'aa-rules::abilities::EnchantmentProperty::required_properties', SOL,
                 'Only the required_properties deferral (473AD0, vtable A23C38) is in scope.'),
    '004bf990': ('fragment', 'aa-rules::abilities::required_property_tokens / native_required_properties', SOL,
                 'Its required_properties block 4C00AB..4C023D executed natively (ability_required section).'),
    '00585280': ('proven', 'aa-combat::walls::intruder_within_wall', GEO, None),
})
for a, r, area, st, rust, ora in [
    ('004c4e10', 'ModifierProperty required_properties resolver (deferred, 463FA0)', SPO, 'proven',
     'aa-rules::abilities::native_required_properties', SOL),
    ('00424fa0', 'Tokenizer constructor (",", flags 0)', SPO, 'proven', 'aa-rules::abilities::required_property_tokens', SOL),
    ('00412480', 'Tokenizer next token (whitespace, delimiters, quotes)', SPO, 'proven',
     'aa-rules::abilities::required_property_tokens', SOL),
    ('004c89a0', 'Ability summoned GTID list head insert', SPO, 'proven', 'aa-combat::summon (add_summoned)', SUM),
    ('004ca7b0', 'Summoned FX event 4 over actor or organization members', SPO, 'proven', 'aa-combat::summon (summoned_fx)', SUM),
    ('00503140', 'Enchantment instance constructor (vtable A2EBE0)', SPO, 'proven',
     'special_ops::EnchantmentWorld::insert_enchantment (fields)', SOE),
    ('005031b0', 'Enchantment instance expiry (vtable A2EBE0 slot 0)', SPO, 'proven', 'aa-combat::special_ops::enchantment_expired', SOE),
    ('004cef80', 'Actor property lookup (581280 -> 5032D0)', SPO, 'proven', 'aa-body::property_api::find_actor_property', SOE),
    ('005851a0', 'Wall point-in-polygon test', 'damage', 'proven', 'aa-combat::walls::point_within_wall', GEO),
    ('005705a0', 'Membership attach (owner, slot)', 'morale', 'proven', 'aa-combat::organization::attach_membership', MEX),
    ('005784c0', 'Property propagation to a new member', 'morale', 'proven', 'aa-combat::organization::propagate_properties', MEX),
    ('0057dcf0', 'Performer ability gating by rank', 'cadence', 'proven', 'aa-combat::performer::gate_by_rank', MEX),
    ('004be4f0', 'Local ability rank window', 'cadence', 'proven', 'aa-combat::performer::rank_allows', MEX),
    ('004be4d0', 'Local ability has rank limits', 'cadence', 'proven', 'aa-combat::performer::rank_allows', MEX),
    ('004f9c10', 'World object creation', SPO, 'seam', 'summon::SummonWorld::create', None),
    ('004fa030', 'World actor creation', SPO, 'seam', 'summon::SummonWorld::create', None),
    ('004fa150', 'World organization creation', SPO, 'seam', 'summon::SummonWorld::create', None),
    ('0058ba90', 'Transport component payload load', SPO, 'seam', 'summon::SummonWorld::load_transport', None),
    ('004d9b60', 'Command issue', SPO, 'seam', 'summon::SummonWorld::order_attack / order_move', None),
    ('004f02b0', 'World RNG access (with 444A90 direction table)', SPO, 'seam', 'summon::SummonWorld::random_direction', None),
    ('00502b30', 'ModifierState AddPropertyModifiers (kinds 3,4,6,7,8,9 tables)', SPO, 'not-ported', None, None),
    ('00502c60', 'ModifierState RemovePropertyModifiers', SPO, 'not-ported', None, None),
    ('004fe6f0', 'Immunity added: purge hostile enchantments of that type', SPO, 'not-ported', None, None),
    ('00502ad0', 'Ability-effect modifier list add (+8C by ability type)', SPO, 'not-ported', None, None),
    ('00502560', 'Periodic damage table add (+98, 5015C0)', SPO, 'not-ported', None, None),
    ('00502640', 'Reverse ability add (+B0, 4C86D0)', SPO, 'not-ported', None, None),
    ('00501f60', 'Resource production table add/subtract (+80)', SPO, 'not-ported', None, None),
]:
    r5(a, r, area, st, rust, ora)

# --- Round 6 (lead approval 2026-10-15): ModifierState table kinds ----------
MTB = P8 + '/native-modifier-tables-fixtures.json'
for a, r, rust in [
    ('00502b30', 'ModifierState AddPropertyModifiers (table kinds 3,4,6,7,8,9; 0,1,2,5 via 502A70)',
     'aa-body::modifier_tables::add_modifier (property_flags dispatch)'),
    ('00502c60', 'ModifierState RemovePropertyModifiers', 'aa-body::modifier_tables::remove_modifier'),
    ('004fe6f0', 'Immunity added: purge hostile enchantments of that type',
     'aa-body::modifier_tables::purge_hostile_enchantments'),
    ('00502ad0', 'Ability-effect list prepend (+8C by AbilityType index)', 'aa-body::modifier_tables::add_modifier'),
    ('00502560', 'Periodic damage record (+98)', 'aa-body::modifier_tables::add_modifier'),
    ('00502640', 'Reverse ability add (+B0)', 'aa-body::modifier_tables::add_modifier'),
    ('00501f60', 'Resource production add/subtract (+80)', 'aa-body::modifier_tables (production)'),
]:
    r5(a, r, SPO, 'proven', rust, MTB)
for a, r, rust in [
    ('005017d0', 'Immunity type removal (+68)', 'aa-body::modifier_tables::remove_modifier'),
    ('00501ef0', 'Ability-effect list unlink', 'aa-body::modifier_tables::remove_modifier'),
    ('00501820', 'Periodic damage record removal', 'aa-body::modifier_tables::remove_modifier'),
    ('005018c0', 'Periodic heal removal', 'aa-body::modifier_tables::remove_modifier'),
    ('005023b0', 'Reverse ability removal (4CE080)', 'aa-body::modifier_tables::remove_modifier'),
    ('005015c0', 'Periodic damage record constructor', 'aa-body::modifier_tables::add_modifier'),
    ('00502700', 'Ability-effect list growth (5021E0)', 'aa-body::modifier_tables::add_modifier'),
    ('005033e0', 'Enchantment instance allegiance resolve (+18)', 'property_flags (InstanceIdentity / ActorInstance)'),
    ('00501690', 'Periodic damage type getter (tick)', 'aa-combat::periodic::periodic_from_tables'),
    ('005016b0', 'Periodic damage amount getter (tick)', 'aa-combat::periodic::periodic_from_tables'),
    ('005016d0', 'Periodic damage source getter (tick)', 'aa-combat::periodic::periodic_from_tables'),
    ('005016f0', 'Periodic damage allegiance getter (tick)', 'aa-combat::periodic::periodic_from_tables'),
    ('00501710', 'Periodic heal amount getter (tick)', 'aa-combat::periodic::periodic_from_tables'),
]:
    r5(a, r, SPO, 'proven', rust, MTB)
r5('004c86d0', 'Ability constructor without local definition (reverse abilities)', SPO, 'proven-other',
   'aa-combat::ability::Ability::new (bind property_flags::Effects::create_reverse_ability)', MTB)

r5('00554b30', 'Open-position search (radius 12)', SPO, 'proven-other', 'aa-spatial::placement::find_placement_within', None)
r5('00552640', 'Map interior clamp', SPO, 'proven-other', 'aa-movement::falling::clamp_to_interior', None)
r5('004f02b0', 'World RNG access (with 444A90 direction table)', SPO, 'proven-other',
   'aa-movement::formation::random_direction with the GWorld+0xD4 draw (aa-runtime simulation)', None)


def main():
    sizes = {}
    names = {}
    for line in FUNCS.read_text().splitlines()[1:]:
        p = line.split('\t')
        sizes[p[0]] = int(p[3])
        names[p[0]] = p[1]
    sources = {}
    for path in (ROOT / 'crates').rglob('*.rs'):
        text = path.read_text(encoding='utf8', errors='replace').lower()
        sources[str(path.relative_to(ROOT)).replace('\\', '/')] = text
    rows = []
    for e in E:
        if e['address'] in OVERRIDE:
            st, rust, ora, note = OVERRIDE[e['address']]
            e = {**e, 'status': st, 'rust': rust, 'oracle': ora, 'note': note or e['note']}
        if e['address'] in AI_OWNED and e['status'] in ('not-ported', 'seam'):
            e = {**e, 'owner': 'AI agent (aa-mind), per lead routing; combat-side services exposed here'}
        if e['status'] == 'skip' or e['role'] == 'placeholder':
            continue
        a = e['address']
        extra = {}
        if a in sizes:
            extra = {'ghidra_name': names[a], 'body_bytes': sizes[a]}
        else:
            value = int(a, 16)
            container = max((k for k in sizes if int(k, 16) <= value), key=lambda k: int(k, 16))
            inside = value < int(container, 16) + sizes[container]
            extra = {'ghidra_name': None, 'ghidra_container': container if inside else None,
                     'note_entry': ('entry inside a Ghidra-merged function' if inside else
                                    'not in the Ghidra function inventory (virtual-table-only entry)') +
                                   '; its instructions are pinned by the cited oracle'}
        short = a.lstrip('0')
        cited = sorted(p for p, t in sources.items() if re.search(r'(?<![0-9])' + short + r'(?![0-9a-f])', t))
        rows.append({**e, **extra, 'cited_in': cited})
    counts = {}
    for r in rows:
        counts[r['status']] = counts.get(r['status'], 0) + 1
    doc = {'schema': 1, 'scope': __doc__, 'counts': counts, 'functions': sorted(rows, key=lambda r: (r['area'], r['address']))}
    (HERE / 'inventory.json').write_text(json.dumps(doc, indent=1) + '\n')
    print(json.dumps(counts))


if __name__ == '__main__':
    main()
