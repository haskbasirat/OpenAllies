"""Build research/phase8-metagame/inventory.json.

Every function in the metagame code range 0x92B000..0x96BB00 of the original
AA.exe (plus listed out-of-range helpers) with:
  source_file      .cpp named by an assert/log string the function references,
                   or inferred from the nearest attributed neighbours when both
                   sides agree (MSVC links object files contiguously; inferred,
                   not proven), else null
  strings          up to 8 referenced strings (string-references.tsv)
  role/ported/proven/notes from ANNOTATIONS below (manual, from pseudocode +
                   instruction reading); null when not analysed yet.
"""
import csv, json, hashlib
from pathlib import Path
HERE = Path(__file__).resolve().parent
NATIVE = HERE.parents[2] / 'reconstruction/research/native-aa'
LO, HI = 0x932000, 0x96bb00   # 0x92B000..0x931FFF is libpng/zlib (strings), excluded

B = 'crates/aa-metagame/src/battle.rs'
R = 'crates/aa-metagame/src/rng.rs'
D = 'crates/aa-metagame/src/data.rs'
FX = 'research/phase8-metagame/native-battle.json'
# address: (role, ported, proven, notes)
ANNOTATIONS = {
    0x85d840: ('engine LCG draw (state*0x41C64E6D+0x3039; returns bits 16..47)', R, FX, 'used on MGState+0x18'),
    0x938990: ('MGState: address of RNG state word (this+0x18)', R, FX, None),
    0x9389b0: ('MGState: kingdom at index (+0x1C list)', B, FX, None),
    0x9389c0: ('MGState: index of kingdom in +0x1C list (-1 if absent)', B, FX, None),
    0x938b70: ('MGState: kingdom whose actor handle (+0xC) resolves to the given kingdom actor', B, FX, 'BattleContext::kingdom_of_actor'),
    0x93ea90: ('army upgrade cost: sum of shop division costs of the army divisions', B, FX, None),
    0x93eb40: ('army definition MetaGameComponent data +4 (movement)', None, None, 'used by 956380 attacker-win cleanup'),
    0x93eb50: ('component: second-to-last entry of +0x24 list', None, None, None),
    0x93eb70: ('army: x87 sum of division damage (+0x34), native order', B, FX, None),
    0x93ebd0: ('army quick-battle strength = division sum + MetaGameComponent damage', B, FX, None),
    0x93ec40: ('comparator weakest (strict >, ties take later army)', B, FX, None),
    0x93eca0: ('comparator strongest (strict <, ties take later army)', B, FX, None),
    0x93f080: ('comparator least expensive (cost a < cost b keeps a)', B, FX, None),
    0x93f110: ('comparator most expensive (cost a > cost b keeps a)', B, FX, None),
    0x94d690: ('filtered territory-army iterator: seek first match', B, FX, None),
    0x945250: ('filtered territory-army iterator: advance', B, FX, None),
    0x954d80: ('territory: fold side armies with a comparator (destroy selection)', B, FX, None),
    0x954eb0: ('select weakest army of side', B, FX, None),
    0x954ed0: ('select strongest army of side', B, FX, None),
    0x954ef0: ('select most expensive army of side', B, FX, None),
    0x954f10: ('select least expensive army of side', B, FX, None),
    0x954f30: ('territory: capture capturable armies (vtable+0x160 change owner)', None, None, 'attacker win path'),
    0x954fd0: ('territory defender strength (armies the owner does not regard as enemies)', B, FX, None),
    0x955320: ('filter: owner regards army kingdom as enemy (vtable+0x140); true when unowned', B, FX, None),
    0x9553a0: ('filter: owner vtable+0x138 relation (ally-like); false when unowned', B, FX, None),
    0x955420: ('territory: n-th attacker-side army (enemies of the owner)', B, FX, None),
    0x9554f0: ('territory: n-th defender-side army (non-enemies of the owner)', B, FX, None),
    0x955e50: ('territory: begin attacker-side iteration (filter 955320)', B, FX, None),
    0x955eb0: ('territory: defending kingdom (owner 0.5, +1 per vtable+0x138 army, strict argmax)', B, FX, None),
    0x956280: ('territory: remove army from list (decrements +0x1C when the owner regards it as enemy)', None, None, 'seamed in oracle'),
    0x956350: ('kill+remove army if non-null', None, None, None),
    0x956380: ('attacker win: destroy defender-side (vtable+0x138) armies whose MG data +4 (movement) is 0', None, None, None),
    0x9564f0: ('destroy behaviour random: draw % side count, nth army, kill+remove', B, FX, 'selection part ported; removal not'),
    0x95c6a0: ('enum table insert (value, name)', None, None, 'seamed'),
    0x95c7b0: ('destroy behaviour enum registration (none..all = 0..7)', B, FX, None),
    0x95ccc0: ('DynamicCampaign data load', D, None, 'subset: see data.rs'),
    0x95f270: ('quick battle technology bonus of a kingdom', B, FX, None),
    0x95f2f0: ('quick battle odds clamp(base - defender + attacker)', B, FX, None),
    0x95f370: ('quick battle attack odds of a territory', B, FX, None),
    0x95f480: ('KKC_QuickBattleSolver::SolveBattle', B, FX, None),
    0x961810: ('shop: division entry by identity', B, FX, None),
    0x961840: ('shop: technology entry by identity', B, FX, None),
    0x961870: ('shop: actor entry by identity', None, None, None),
    0x966f40: ('destroy behaviour first', B, FX, 'selection = nth_army(side, 0); not separately executed'),
    0x966f70: ('destroy behaviour weakest', B, FX, 'selection via 954EB0'),
    0x966f90: ('destroy behaviour strongest', B, FX, 'selection via 954ED0'),
    0x966fb0: ('destroy behaviour least expensive', B, FX, 'selection via 954F10'),
    0x966fd0: ('destroy behaviour most expensive', B, FX, 'selection via 954EF0'),
    0x9670d0: ('apply destroy behaviour to a side (switch; 7=all loop)', None, None, 'needs removal port'),
    0x967290: ('MGBattleResolver: transfer territory / defending destroy behaviour', None, None, None),
    0x967390: ('KKC_MGBattleResolver::ProcessAttackerWin', None, None, None),
    0x9673f0: ('KKC_MGBattleResolver::ProcessDefenderWin (attacking_destroy_behavior)', None, None, None),
    0x9674f0: ('KKC_MGBattleResolver::ProcessCheat', None, None, None),
    0x967530: ('KKC_MGBattleResolver::Process', None, None, None),
}
Q = 'crates/aa-metagame/src/qsort.rs'
QX = 'research/phase8-metagame/native-qsort.json'
DT = 'crates/aa-metagame/tests/real_data.rs + loader_validation.rs (not oracle)'
ANNOTATIONS.update({
    0x96ce20: ('CRT _qsort (VS2003), unstable; used for shop lists', Q, QX, None),
    0x96cdb0: ('CRT shortsort (first maximum to the end)', Q, QX, None),
    0x961790: ('shop comparator: ascending signed cost', D, QX, None),
    0x962290: ('Shop load: Technology, Actor, Division lists, then qsort all/actors/divisions/technologies', D, QX, 'order proven on the shipped shops; lookups ported, not oracle'),
    0x961d90: ('Shop Technology list (cost then ids; empty -> NULL ref)', D, None, DT),
    0x961b10: ('Shop Actor list (4BDBA0 category 0x28 METAGAME_ACTOR)', D, None, DT),
    0x962010: ('Shop Division list (empty -> NULL ref)', D, None, DT),
    0x95c840: ('campaign Kingdom list: pass 1 load all, pass 2 liberates_to', D, None, DT),
    0x95e320: ('kingdom territory liberates_to pass (first-match kingdom)', D, None, DT),
    0x95eb00: ('campaign Kingdom load (ids, starting_resources, capital via map, mgai, stockpiles, territories, objective, shop)', D, None, DT),
    0x95e740: ('kingdom Territory records + Actor lists (actor Things)', D, None, DT),
    0x95c5d0: ('validation: a territory owned by two kingdoms (order i, record, k)', D, None, DT),
    0x95c9b0: ('campaign Team list', D, None, DT),
    0x95e540: ('Team load (kingdom first-match lookup)', D, None, DT),
    0x95cad0: ('campaign Event list', D, None, DT),
    0x968000: ('Event load (round, round_probability, kingdom_turn_ids, effects)', D, None, 'no shipped events'),
    0x967a40: ('event effect type enum (add_tech 0x101, add_actor 0x11001, remove_tech 0x101, remove_actor 1, change_territory 0x10011)', D, None, None),
    0x967c10: ('event Effect load (flag-gated required/optional lookups)', D, None, 'no shipped effects'),
    0x968250: ('Objective load (single required child, capital_captures)', D, None, DT),
    0x958700: ('territory type enum (unknown, normal, neutral, background, alias)', D, None, DT),
    0x9587e0: ('land/water node enum (node_land, node_water)', D, None, DT),
    0x959a50: ('actor_spacing_type enum (simple, circle, square)', D, None, DT),
    0x958ae0: ('MGMap territory load (type, points, spacing, object, normal-only rmc/production/minimap_alpha, slots)', D, None, DT),
    0x9583c0: ('territory AddLink (rmc attacker case-sensitive check)', D, None, DT),
    0x9625e0: ('territory Link load (intermediate points, rmc_attacker_ids for normal owners)', D, None, DT),
    0x95d930: ('MGMap load (blend check, width/height, offset/scale, two passes, alias pairing)', D, None, DT),
    0x95d5c0: ('validation: links to alias territories', D, None, DT),
    0x9579a0: ('territory offset (location/bounds/link points += offset)', D, None, DT),
    0x957a20: ('territory scale (x,y of location/bounds/slots/link points *= scale)', D, None, DT),
    0x93b250: ('MetaGameComponent load (army definition)', D, None, DT),
    0x941240: ('MGDivision load', D, None, DT),
    0x960510: ('MGAI load', D, None, DT),
    0x960120: ('MGAI per-map load (DefaultTerritory/Capital/FriendlyCapital/EnemyCapital + Territory list)', D, None, DT),
    0x95fd70: ('MGAI territory record load', D, None, DT),
    0x9536e0: ('built-in MGCommand registration (12 commands, indices 0..11)', D, None, DT),
    0x949b60: ('MGCommand data load', D, None, 'not in functions.tsv; vtable 0xAC9364'),
    0x954570: ('KKC_MGCommand::Process dispatch by command index 0..11', None, None, None),
    0x951d70: ('MGCommand validation: flag checks then per-command validators; returns error code', None, None, 'next port target'),
    0x952ec0: ('command error code -> message (5 not_owned, 6 cannot_move, 7 cannot_move_to, 8 enemy_territory_required, 9 friendly_territory_required, 0xB existing_pending_battle, 0xC no_attacks_left, 0xD must_pullout, 0x14 insufficient_resources, 0x15 purchase_unowned_capital)', None, None, None),
    0x956c20: ('UI: validate (951D70) and issue a metagame command', None, None, None),
    0x954b30: ('territory: owner regards kingdom as enemy (vtable+140), true when unowned', None, None, 'used by command flag 0x20'),
    0x954c00: ('territory: owner does not regard kingdom as enemy, false when unowned', None, None, 'used by command flag 0x40'),
    0x939b90: ('KKC_MGState::InitCreate: MG kingdoms, territories (owned then unowned), capitals, events, props', None, None, 'next port target'),
    0x938ee0: ('KKC_MGState save writer (strategic save format)', None, None, 'next port target'),
    0x95a100: ('MG kingdom state constructor (+0x38 resources = starting_resources)', None, None, None),
    0x954a90: ('MG territory constructor (owned record)', None, None, None),
    0x954ae0: ('MG territory constructor (unowned map territory)', None, None, None),
    0x955620: ('territory: set owner (+0x24)', None, None, None),
    0x95a6b0: ('victory test: enemies whose capital condition holds >= objective capital_captures', None, None, 'uses 95A350 (unanalysed)'),
    0x938e10: ('end-of-game check over kingdoms (sets MGState+0x50)', None, None, None),
    0x93a320: ('find territory with a pending battle', None, None, None),
    0x952790: ('KKC_MGCommand::ProcessEndTurn', None, None, None),
    0x954220: ('KKC_MGCommand::ProcessQuickBattle (SolveBattle -> ResolveBattle)', None, None, None),
    0x954130: ('KKC_MGCommand::ResolveBattle (history entry 95EFD0)', None, None, None),
})
S = 'crates/aa-metagame/src/state.rs'
IX = 'research/phase8-metagame/native-init.json'
ANNOTATIONS.update({
    0x939b90: ('KKC_MGState::InitCreate: MG kingdoms, owned territories + armies, unowned territories, capitals, events, props, current = first', S, IX, 'positions/visuals are seams; events not shipped'),
    0x939690: ('KKC_MGState constructor (RNG 0x8E4B12A5, 25-bucket territory hash, vectors)', S, IX, 'used as the oracle start state'),
    0x939440: ('KKC_MGState destructor', None, None, None),
    0x95a100: ('MG kingdom constructor (+0x38 resources = starting_resources)', S, IX, None),
    0x954a90: ('MG territory constructor (owned record)', S, IX, None),
    0x954ae0: ('MG territory constructor (unowned map territory)', S, IX, None),
    0x955620: ('territory: set owner (959F20 leave, 95A850 append, 954CF0 recount)', S, IX, 'old-owner path not exercised by InitCreate'),
    0x954cf0: ('territory: recount +0x1C (all armies when unowned)', S, IX, None),
    0x95a850: ('MG kingdom: append owned territory', S, IX, None),
    0x95a690: ('MG kingdom owns map territory (95A640 case-sensitive ids compare)', S, IX, None),
    0x955d50: ('territory: create the record actors (955B70 each)', S, IX, 'actor creation 4FA030 is a seam'),
    0x955b70: ('territory: place one army (TerritoryPositionSolver, 4FA030 create, 940100 register)', S, IX, 'positions not ported (needs aa-trig)'),
    0x940100: ('army component: enter territory (956280 leave old, 956140 add, 93F9F0 path)', S, IX, 'leave-old path not exercised'),
    0x956140: ('territory: add army ref at first free slot (slot-ordered), +0x1C++ when enemy/unowned', S, IX, None),
    0x93f9f0: ('army component: path append / truncate after revisit', S, IX, 'truncate path not exercised'),
    0x954a10: ('territory: first free display slot', S, IX, None),
    0x9548e0: ('army ref constructor (+4 handle, +8 slot, +C army snapshot 9405E0)', None, None, None),
    0x9405e0: ('army snapshot record (owner kingdom name, definition, path defs)', None, None, 'used by save'),
    0x959420: ('TerritoryPositionSolver: slot position (actor slots + spacing layout)', None, IX, 'positions recorded; port needs aa-trig'),
    0x959560: ('circle spacing layout (6 per ring, trig lookups)', None, IX, None),
    0x959b20: ('actor_spacing_type enum table constructor', None, IX, 'static init used by oracles'),
    0x9c8620: ('static init: cos lookup table (4096)', None, IX, 'aa-trig'),
    0x9c8640: ('static init: sin lookup table (4096)', None, IX, 'aa-trig'),
    0x93a3b0: ('KKC_MGState vtable+0x24 (likely load/reader)', None, None, 'vtable 0xAC77CC'),
    0x93a990: ('KKC_MGState vtable+0x18', None, None, 'calls SolveBattle'),
})
C = 'crates/aa-metagame/src/commands.rs'
CX = 'research/phase8-metagame/native-commands.json'
PX = 'research/phase8-metagame/native-process.json'
VX = 'research/phase8-metagame/native-victory.json'
MX = 'research/phase8-metagame/native-moves.json'
ANNOTATIONS.update({
    0x951d70: ('MGCommand validation (presence 950D30, flags 0x20/0x40/0x800, per-command validators)', C, CX, 'path result (93FDC0) is an input'),
    0x950d30: ('command presence checks by flag groups', C, CX, None),
    0x949820: ('command data flag test', C, CX, None),
    0x9512b0: ('validate move (codes 4,2,5,0xA,6,7)', C, CX, 'army vtable+0x134 is a seam input'),
    0x951480: ('validate attack (move checks + 0xB pending battle, 0xC attacks left)', C, CX, None),
    0x950db0: ('validate startbattle (0xE, 0xD)', C, CX, None),
    0x950df0: ('validate quickbattle (0xF, 0xE, 0xD)', C, CX, None),
    0x951710: ('validate endturn (4, 0x10)', C, CX, None),
    0x951770: ('validate purchasetech (4, 0x11, 0x14)', C, CX, None),
    0x951800: ('validate purchaseactor (4, 0x12, 0x14, 0x15 capital lost)', C, CX, None),
    0x951890: ('validate upgradeactor (4, 2, 0x13, 0x14; prerequisites vs current divisions)', C, CX, None),
    0x9519d0: ('validate upgradeactorall (4, 2, 0x14)', C, CX, None),
    0x950ef0: ('validate take (cheat gate 40FCA0)', C, CX, 'cheats off'),
    0x93a320: ('pending battle (first territory with attackers, defenders and strength > 0; alias -> partner)', C, CX, None),
    0x93f7a0: ('upgrade-all division list (shop order, repeated passes)', C, PX, None),
    0x93f970: ('upgrade-all cost', C, CX, None),
    0x9510f0: ('KKC_MGCommand constructor', C, CX, 'used by oracles'),
    0x950cf0: ('command territory setter (alias -> partner)', C, CX, None),
    0x954570: ('KKC_MGCommand::Process dispatch', C, PX, 'endturn/quickbattle/take in turn/resolve modules'),
    0x9543c0: ('ProcessMove -> 9537B0', C, PX, None),
    0x954400: ('ProcessAttack -> 9537B0', C, PX, None),
    0x9537b0: ('move/attack: path, waypoint queue 93F4B0, movement message', C, PX, 'messages are seams (category recorded)'),
    0x93f4b0: ('replace waypoint queue with a path', C, PX, None),
    0x954440: ('ProcessStartBattle (+0x44=1, +0x46=0)', C, PX, 'quickbattle branch unsupported'),
    0x950e40: ('ProcessPlayBattle (+0x45=1)', C, PX, None),
    0x950e70: ('ProcessCancelBattle (+0x44=0, +0x46=0)', C, PX, None),
    0x9528b0: ('ProcessPurchaseTech (pay, add if absent 95A770)', C, PX, None),
    0x95a770: ('MG kingdom add technology if absent', C, PX, None),
    0x952a00: ('ProcessPurchaseActor (pay, 955B70 at capital, owner = capital owner, +0x4C)', C, PX, 'actor creation 4FA030 seam'),
    0x952d10: ('ProcessUpgradeActor (952B90 each, 951AA0)', C, PX, None),
    0x952d90: ('ProcessUpgradeActorAll (93F7A0, 952B90 each, 951AA0)', C, PX, None),
    0x952b90: ('upgrade helper: add division if absent, pay only when added', C, PX, None),
    0x93f750: ('component add division (unique)', C, PX, None),
    0x951aa0: ('ChangeGActor: strictly lower division_matching_preference type whose divisions are all present', C, PX, 'respawn 955DD0 seams; waypoints cleared'),
    0x955dd0: ('respawn army actor with a new type (snapshot 9402E0, 955820, 4F6760, 93F620)', C, PX, 'engine seams'),
    0x956280: ('territory: remove army ref (+0x1C update)', S, MX, None),
    0x959f20: ('MG kingdom: remove LAST matching owned territory', S, VX, None),
    0x95a6b0: ('victory test', S, VX, None),
    0x95a350: ('capital lost test', S, VX, None),
})
PA = 'crates/aa-metagame/src/path.rs'
PX = 'research/phase8-metagame/native-path.json'
ANNOTATIONS.update({
    0x93fdc0: ('army path: alias target -> partner, 93FCC0 then fallback 93FAD0', PA, PX, None),
    0x93fcc0: ('army path search with budget (0 if current hostile or fought, else movement - moves used)', PA, PX, None),
    0x93fad0: ('army path fallback from path[0] with full movement', PA, PX, None),
    0x93eae0: ('moves used (link costs over the turn path)', PA, PX, None),
    0x957910: ('territory def: link to a target def', PA, PX, None),
    0x962310: ('link cost: 1 unless the link owner is an alias', PA, PX, None),
    0x961550: ('path search object setup (start, goal, kingdom actor, budget)', PA, PX, None),
    0x961430: ('path search: node store 961230 + open heap', PA, PX, None),
    0x961230: ('path node store constructor (vtable ACDD68)', PA, PX, None),
    0x961350: ('path search run; accept only result 1 with count <= budget (or -1)', PA, PX, None),
    0x960fb0: ('greedy best-first search loop (parent fixed at discovery, goal before blocked test)', PA, PX, None),
    0x93ed00: ('path expansion (heap key f32(link cost + f32(h)))', PA, PX, None),
    0x960d00: ('blocked node test (kingdom, not goal, hostile or unowned)', PA, PX, None),
    0x960d60: ('trivial case: start == goal or direct link', PA, PX, None),
    0x960e00: ('depth limit test', PA, PX, None),
    0x960e20: ('link cost query', PA, PX, None),
    0x960e70: ('heuristic (|dx|+|dy|) * 0x3D888889, 0 at goal', PA, PX, None),
    0x960f30: ('preconditions (budget 0 needs start == goal; both defs need links)', PA, PX, None),
    0x960db0: ('path reversal', PA, PX, None),
    0x53f350: ('heap sift-up (moves while new <= parent)', PA, PX, None),
    0x53f410: ('heap sift-down (right child unless left < right)', PA, PX, None),
    0x53e7b0: ('heap constructor (max 0 = unlimited)', PA, PX, None),
})
ANNOTATIONS.update({
    0x966ff0: ('KKC_MGBattleResolver constructor (+4 territory, +8 attacker won)', 'crates/aa-metagame/src/resolve.rs', 'research/phase8-metagame/native-resolve.json', None),
    0x967530: ('KKC_MGBattleResolver::Process (attackers fought++, battles counter, win/loss)', 'crates/aa-metagame/src/resolve.rs', 'research/phase8-metagame/native-resolve.json', None),
    0x967390: ('ProcessAttackerWin -> 967290(winner, 1)', 'crates/aa-metagame/src/resolve.rs', 'research/phase8-metagame/native-resolve.json', None),
    0x9673f0: ('ProcessDefenderWin (attacking destroy behaviour, attacker retreat orders)', 'crates/aa-metagame/src/resolve.rs', 'research/phase8-metagame/native-resolve.json', None),
    0x967290: ('attacker win: liberation, capture 954F30, immobile cleanup 956380, defending destroy behaviour, retreat 9671A0, owner change', 'crates/aa-metagame/src/resolve.rs', 'research/phase8-metagame/native-resolve.json', None),
    0x9670d0: ('apply destroy behaviour to a side (1-6 unless skip, 7 all loop)', 'crates/aa-metagame/src/resolve.rs', 'research/phase8-metagame/native-resolve.json', None),
    0x9671a0: ('retreat survivors to 967040 target or destroy all defenders', 'crates/aa-metagame/src/resolve.rs', 'research/phase8-metagame/native-resolve.json', None),
    0x967040: ('retreat target: linked territory of a friend of the old owner with the strictly largest strength', 'crates/aa-metagame/src/resolve.rs', 'research/phase8-metagame/native-resolve.json', None),
    0x956380: ('kill immobile defender-side (vtable+0x138) armies in reverse order', 'crates/aa-metagame/src/resolve.rs', 'research/phase8-metagame/native-resolve.json', None),
    0x954f30: ('capture capturable armies (vtable+0x160), recount against old owner', 'crates/aa-metagame/src/resolve.rs', 'research/phase8-metagame/native-resolve.json', None),
    0x956350: ('kill (4D7960 seam) and remove an army', 'crates/aa-metagame/src/resolve.rs', 'research/phase8-metagame/native-resolve.json', None),
    0x93f550: ('replace the waypoint queue with one territory', 'crates/aa-metagame/src/resolve.rs', 'research/phase8-metagame/native-resolve.json', None),
    0x93eb50: ('previous territory of the turn path', 'crates/aa-metagame/src/resolve.rs', 'research/phase8-metagame/native-resolve.json', None),
    0x940020: ('deferred move step: pop waypoint, leave/enter territory, path', 'crates/aa-metagame/src/resolve.rs', 'research/phase8-metagame/native-resolve.json', None),
    0x954130: ('KKC_MGCommand::ResolveBattle (messages seam, Process, 938E10, +0x46, battle record)', 'crates/aa-metagame/src/resolve.rs', 'research/phase8-metagame/native-resolve.json', None),
    0x954220: ('KKC_MGCommand::ProcessQuickBattle (93A320, SolveBattle, ResolveBattle)', 'crates/aa-metagame/src/resolve.rs', 'research/phase8-metagame/native-resolve.json', None),
    0x938e10: ('game-over check (95A6B0 per kingdom; engine victory seams; +0x50)', 'crates/aa-metagame/src/resolve.rs', 'research/phase8-metagame/native-resolve.json', None),
    0x938980: ('set MGState+0x46', 'crates/aa-metagame/src/resolve.rs', 'research/phase8-metagame/native-resolve.json', None),
    0x939410: ('append battle record to MGState+0x60', 'crates/aa-metagame/src/resolve.rs', 'research/phase8-metagame/native-resolve.json', None),
    0x95efd0: ('battle record constructor (territory, attacker, defender, won)', 'crates/aa-metagame/src/resolve.rs', 'research/phase8-metagame/native-resolve.json', None),
    0x952790: ('KKC_MGCommand::ProcessEndTurn (message seam, 9391F0)', 'crates/aa-metagame/src/turn.rs', 'research/phase8-metagame/native-turn.json', None),
    0x9391f0: ('advance to the next kingdom that can play (at most kingdom count steps)', 'crates/aa-metagame/src/turn.rs', 'research/phase8-metagame/native-turn.json', None),
    0x938da0: ('current kingdom can play (capital held or a mobile army)', 'crates/aa-metagame/src/turn.rs', 'research/phase8-metagame/native-turn.json', None),
    0x938c90: ('advance one kingdom: results cleared, battles reset, auto-captures, per-army reset, round/income/events, flags cleared', 'crates/aa-metagame/src/turn.rs', 'research/phase8-metagame/native-turn.json', None),
    0x938a80: ('clear the battle records', 'crates/aa-metagame/src/turn.rs', 'research/phase8-metagame/native-turn.json', None),
    0x9556d0: ('auto-capture an undefended territory with attackers; per-army turn reset 93FE10', 'crates/aa-metagame/src/turn.rs', 'research/phase8-metagame/native-turn.json', None),
    0x93fe10: ('army turn reset (fought 0, path = [current])', 'crates/aa-metagame/src/turn.rs', 'research/phase8-metagame/native-turn.json', None),
    0x959fb0: ('income: resources += production of owned territories', 'crates/aa-metagame/src/turn.rs', 'research/phase8-metagame/native-turn.json', None),
    0x95add0: ('campaign event roll (RNG vs round_probability)', None, None, 'Unsupported: shipped campaigns have no events'),
    0x95be20: ('campaign event fire (effects 95B220..95B9B0)', None, None, 'Unsupported: shipped campaigns have no events'),
})
ANNOTATIONS.update({
    0x938ee0: ('KKC_MGState::Save (METAGAME chunk body; via 4F2DB0 -> 859150)', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', 'chunk framing 859150/859030 not ported; Unsupported at native NULL dereferences (from the listing)'),
    0x93a3b0: ('KKC_MGState::Load (version checks, kingdoms, territories, current by wcscmp, records; events read and dropped)', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', 'territory record (+08) not restored, as natively'),
    0x939930: ('METAGAME load target: 939690 constructor, published in B49FA0 (as 4FC600)', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', None),
    0x95a470: ('MG kingdom Save', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', None),
    0x95aac0: ('MG kingdom Load (technologies keep null entries, help popups drop them)', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', None),
    0x95a1c0: ('MG kingdom constructor (0x40)', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', None),
    0x955160: ('MG territory Save (thiscall (MGState, ctx), ret 8)', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', None),
    0x9565e0: ('MG territory Load (owner 938BC0/95A850/954CF0, army refs)', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', 'owned-territory lists rebuilt in territory order'),
    0x954a40: ('MG territory constructor (0x28, index)', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', None),
    0x956570: ('MG territory: clear the army reference list', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', None),
    0x938bc0: ('MGState: MG kingdom by kingdom data (first case-insensitive match)', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', None),
    0x954970: ('territory army reference Save (vtable ACBDCC slot 0)', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', None),
    0x9549c0: ('territory army reference Load (vtable ACBDCC slot 1)', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', None),
    0x954860: ('territory army reference constructor', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', None),
    0x93f1a0: ('army snapshot (0x28) Save', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', None),
    0x9409e0: ('army snapshot (0x28) Load (map lookups through B49FA0)', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', 'unknown Thing (4BDBA0) silently null'),
    0x93f590: ('army snapshot constructor', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', None),
    0x9402e0: ('army snapshot capture (owner engine name, Thing IDS, path definitions)', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', None),
    0x95b020: ('campaign event state Save', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', 'no shipped campaign has events (synthetic events in the oracle)'),
    0x95b0f0: ('campaign event state Load (result never stored by 93A3B0)', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', None),
    0x95ad90: ('campaign event state constructor (0x14)', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', None),
    0x95f000: ('battle record Save', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', None),
    0x95f100: ('battle record Load (lookups through B49FA0)', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', None),
    0x95ef90: ('battle record default constructor', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', None),
    0x93eea0: ('army MetaGame component record Save (writes -1 for a null territory)', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', None),
    0x940740: ('army MetaGame component record Load (territory indices via 9389F0, unchecked)', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', 'DYNAMIC_THINGS vs METAGAME load order unverified'),
    0x95aa00: ('IDS reader KKC_MGHelpPopupData (empty -> null, unknown -> throw 41DFB0)', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', None),
    0x940680: ('IDS reader KKC_MGDivisionData (empty -> null, unknown -> throw 41DFB0)', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', None),
    0x939a20: ('campaign registry lookup by IDS', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', None),
    0x95c0d0: ('campaign: kingdom data by IDS', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', None),
    0x95c130: ('campaign: kingdom data lookup (record kingdoms)', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', None),
    0x95d900: ('campaign map: territory definition by IDS (hash 405030 + __wcsicmp)', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', None),
    0x938970: ('MGState: campaign map ([[+0x08]+0x18])', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', None),
    0x939b60: ('MGState: territory by map IDS (hash +0x34)', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', None),
    0x9389f0: ('MGState: territory at index (no bounds check)', 'crates/aa-metagame/src/save.rs', 'research/phase8-metagame/native-save.json', None),
    0x948ef0: ('KKC_MetaGameRTSLauncher::RequestGame: order sequence up to Launch 40DF00 (RTS seam)', 'crates/aa-metagame/src/launch.rs', 'research/phase8-metagame/native-launch.json', 'RMC game settings (+0x90) carried by RMC id only'),
    0x947560: ('participant builder: attackers (955E50) then defenders (955E80)', 'crates/aa-metagame/src/launch.rs', 'research/phase8-metagame/native-launch.json', 'NULL dereferences -> Unsupported (315 oracle cases)'),
    0x946010: ('participant find-or-create by (kingdom data, RMC kingdom id), add army', 'crates/aa-metagame/src/launch.rs', 'research/phase8-metagame/native-launch.json', None),
    0x945200: ('participant record helper', 'crates/aa-metagame/src/launch.rs', 'research/phase8-metagame/native-launch.json', None),
    0x945bd0: ('participant record (0x2C) constructor', 'crates/aa-metagame/src/launch.rs', 'research/phase8-metagame/native-launch.json', None),
    0x945ef0: ('participant record: add army', 'crates/aa-metagame/src/launch.rs', 'research/phase8-metagame/native-launch.json', None),
    0x955e80: ('territory: begin defender-side iteration (filter 9553A0)', 'crates/aa-metagame/src/launch.rs', 'research/phase8-metagame/native-launch.json', None),
    0x946810: ('SetPlayerKingdom(player, "") for every session player', 'crates/aa-metagame/src/launch.rs', 'research/phase8-metagame/native-launch.json', None),
    0x9459f0: ('side stockpile sums (binary32 element adds 4CCD80)', 'crates/aa-metagame/src/launch.rs', 'research/phase8-metagame/native-launch.json', None),
    0x9466c0: ('start properties of a participant', 'crates/aa-metagame/src/launch.rs', 'research/phase8-metagame/native-launch.json', None),
    0x945c90: ('start properties: owned technologies', 'crates/aa-metagame/src/launch.rs', 'research/phase8-metagame/native-launch.json', None),
    0x945d10: ('start properties: division technologies of the side', 'crates/aa-metagame/src/launch.rs', 'research/phase8-metagame/native-launch.json', None),
    0x948bd0: ('additional actors: send per participant', 'crates/aa-metagame/src/launch.rs', 'research/phase8-metagame/native-launch.json', None),
    0x9488b0: ('additional actors: (rmc id, actor) rows, armies then divisions', 'crates/aa-metagame/src/launch.rs', 'research/phase8-metagame/native-launch.json', None),
    0x948770: ('additional actors helper', 'crates/aa-metagame/src/launch.rs', 'research/phase8-metagame/native-launch.json', None),
    0x954be0: ('friendly side test of a kingdom on the battle territory (!954B70)', 'crates/aa-metagame/src/launch.rs', 'research/phase8-metagame/native-launch.json', None),
    0x954b70: ('hostile side test of a kingdom on the battle territory', 'crates/aa-metagame/src/launch.rs', 'research/phase8-metagame/native-launch.json', None),
    0x93a990: ('KKC_MGState vtable+0x18: RTS result decision (winner name, local-side rule, SolveBattle draw) then 954130', 'crates/aa-metagame/src/launch.rs', 'research/phase8-metagame/native-launch.json', 'composition with 954130 not oracle-proven end to end'),
    0x506370: ('RMC settings: slider midpoints (integer (max-min)/2+min)', 'crates/aa-metagame/src/launch.rs', 'research/phase8-metagame/native-launch.json', 'proven on synthetic sliders; RMC loader 4A4EB0 not ported'),
    0x933170: ('MG kingdom -> kingdom actor (used by 93A990)', 'crates/aa-metagame/src/launch.rs', 'research/phase8-metagame/native-launch.json', None),
    0x95f220: ('KKC_QuickBattleSolver wrapper (SolveBattle draw for 93A990)', 'crates/aa-metagame/src/launch.rs', 'research/phase8-metagame/native-launch.json', None),
    0x945600: ('TGC_SetKingdomMergeIntoIDSOrder sender (seam)', None, 'research/phase8-metagame/native-launch.json', 'order decoded at the seam'),
    0x945540: ('TGC_SetKingdomColorOrder sender (seam)', None, 'research/phase8-metagame/native-launch.json', 'order decoded at the seam'),
    0x9456c0: ('TGC_AddKingdomStartPropertyOrder sender (seam)', None, 'research/phase8-metagame/native-launch.json', 'order decoded at the seam'),
    0x945900: ('TGC_SetKingdomBonusStockpileOrder sender (seam)', None, 'research/phase8-metagame/native-launch.json', 'order decoded at the seam'),
    0x945780: ('TGC_ClearKingdomAdditionalActorsOrder sender (seam)', None, 'research/phase8-metagame/native-launch.json', 'order decoded at the seam'),
    0x945840: ('TGC_AddKingdomAdditionalActorOrder sender (seam)', None, 'research/phase8-metagame/native-launch.json', 'order decoded at the seam'),
})
ANNOTATIONS.update({
    0x963430: ('KKC_MGAIPlayer constructor (controller, MG kingdom; vtable ACE11C)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x964d30: ('KKC_MGAIPlayer::PlayTurn (stages: analysis, goals, purchase, attack, moves; yields 944CE0)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', 'driven by controller 945090 (not ported)'),
    0x962b70: ('DoMapAnalysis', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x9699e0: ('analysis: 96AA90 on every territory analysis', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x96aa90: ('territory analysis update', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x969cb0: ('MGAI analysis constructor (0x34)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x96a780: ('territory analysis constructor (0x28)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x95fdd0: ('MGAI map record: first record whose map is the campaign map', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x95ff10: ('MGAI territory record: first whose definition IDS equals (wcscmp)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x96a230: ('proximity (BFS 96A1A0/96A090/969C20/969BC0/969B40/969B60)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x96a1a0: ('proximity BFS', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x96a090: ('proximity BFS step', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x969c20: ('proximity BFS helper', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x969bc0: ('proximity BFS helper', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x969b40: ('proximity BFS queue helper', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x969b60: ('proximity BFS queue helper', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x96cc4c: ('CRT _ftol2 (proximity)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x96a4c0: ('influence', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x96a330: ('influence per kingdom (binary32)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x96a5a0: ('influence accumulate (kingdom index 9389C0, binary32)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x4a6280: ('integer pow (and inlined copies in the influence code)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x969e60: ('border / front flags', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x9699b0: ('importance', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x96a980: ('importance per territory (with 96A5D0)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x96a5d0: ('importance with an uninitialised operator-new block (border flags set on analyses)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', 'block contents are an oracle input'),
    0x96a9c0: ('threat', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x9635a0: ('SetGoalPriorities', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x9632e0: ('goal threshold / army assignment (threshold, goals, armies)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x969430: ('Attack goal Reset', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x9697d0: ('AttackUnoccupied goal Reset', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x969600: ('Defend goal Reset', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x9683b0: ('goal item list maintenance', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x869d30: ('bisection insert (descending priority)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x9682a0: ('goal item comparator', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x969180: ('attack goal item constructor (vtable ACF758)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', 'serves Attack and AttackUnoccupied goals'),
    0x969130: ('attack item priority f32(b*(1-f) + f*a)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x9690c0: ('defend goal item constructor (vtable ACF750)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x968930: ('defend item priority (-> 968720)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x968720: ('defence value (both item kinds)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x968c50: ('attack distance value: importance*(min-1)/(N-1), no_attack_distance as path budget', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', 'no_attack_distance == 1 divides by zero: Unsupported'),
    0x968a50: ('share of the armies distance-weighted strength', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x968940: ('goal item value helper (stale handle dereference without null test)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', 'stale handles: Unsupported'),
    0x969010: ('priority reduction by adding an army (binary32)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x968f70: ('goal item: assign army with reduction', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x969250: ('items with priority >= threshold', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x969380: ('best reduction over eligible items (strictly greater)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x9692b0: ('give the army to the eligible item with the largest reduction', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x9682e0: ('largest priority (strictly greater wins)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x968eb0: ('goal iterator', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x968520: ('goal iterator', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x968560: ('goal iterator', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x968400: ('goal iterator', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x968420: ('goal iterator', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x968440: ('goal iterator', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x968ed0: ('goal iterator', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x963b70: ('purchasing', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', 'empty or zero-cost shops: Unsupported (native divides by zero / loops)'),
    0x962f00: ('purchase decision (capital lost / no upgrade possible paths)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x962db0: ('some kingdom army lacks a shop division (upgrade possible)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x963900: ('random affordable missing shop division for an army', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x962cb0: ('shuffle (app RNG [[B36528]+0x2C])', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x938ac0: ('MGState: territories owned by kingdoms the kingdom regards as enemies (with 959F00)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', 'gates army purchases'),
    0x959f00: ('MG kingdom: owned territory count', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x959ff0: ('MG kingdom: owns technology (+0x20 list)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x93e9e0: ('army helper (purchasing)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x963fd0: ('attack decision', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x955650: ('territory: another neighbour hostile to the owner', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x955090: ('territory: binary32 strength sum of its armies', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x9649a0: ('SetGoalPriorities when MGState+0x46 == 1', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x9649c0: ('Move: one step toward every item for every assigned army', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x962b20: ('4-field record constructor (arg, [item]+0x14 or 0, arg, moved pointer) used by the MGAI', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x963030: ('DoMovement: issue orders and wait for arrival (tick limit)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x962a40: ('EndTurn (waits after a resolved battle)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x93f030: ('army remaining movement (93FCC0 budget rules)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x9685a0: ('Attack goal constructor (vtable ACF620)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x9686a0: ('AttackUnoccupied goal constructor (vtable ACF630)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x968620: ('Defend goal constructor (vtable ACF628)', 'crates/aa-metagame/src/mgai.rs', 'research/phase8-metagame/native-mgai-c*.json', None),
    0x944ce0: ('MGAI controller yield (SAI fiber scheduler 79DFF0)', None, 'research/phase8-metagame/native-mgai-c*.json', 'seam: may change the state'),
    0x944f50: ('MGAI controller command queue', None, 'research/phase8-metagame/native-mgai-c*.json', 'seam: issued commands are the MGAI output'),
    0x944f90: ('MGAI controller: one player per MG kingdom', None, None, 'not ported'),
    0x945090: ('MGAI controller: drives PlayTurn', None, None, 'not ported'),
})
ANNOTATIONS.update({
    0x5b5620: ('TGI localized getter: required field (throws Node doesnt contain field.), a #key is replaced by the string table 4B9F40 at load time', 'crates/aa-metagame/src/data/fields.rs', None, 'keys kept; string-table lookup not ported; checked on shipped data (real_data), not oracle'),
    0x47d960: ('data base loader: "loading duplicate node with ids", required localized name, loaded flag +0x14', 'crates/aa-metagame/src/data.rs', None, 'checked on shipped data, not oracle'),
})
ANNOTATIONS.update({
    0x959420: ('KKC_TerritoryPositionSolver::Solve (owner at +8 never read)', 'crates/aa-metagame/src/positions.rs', 'research/phase8-metagame/native-positions.json', None),
    0x959220: ('TerritoryPositionSolver constructor', 'crates/aa-metagame/src/positions.rs', 'research/phase8-metagame/native-positions.json', None),
    0x959390: ('SIMPLE layout (8 offsets from 959760)', 'crates/aa-metagame/src/positions.rs', 'research/phase8-metagame/native-positions.json', None),
    0x959560: ('CIRCLE layout (ring base 6)', 'crates/aa-metagame/src/positions.rs', 'research/phase8-metagame/native-positions.json', None),
    0x959240: ('SQUARE layout (ring base 8, jump table 95937C)', 'crates/aa-metagame/src/positions.rs', 'research/phase8-metagame/native-positions.json', 'default branch (side > 3) unreachable'),
    0x959760: ('SIMPLE layout offsets', 'crates/aa-metagame/src/positions.rs', 'research/phase8-metagame/native-positions.json', None),
    0x954a10: ('territory: first free army slot', 'crates/aa-metagame/src/positions.rs', 'research/phase8-metagame/native-positions.json', None),
    0x954cc0: ('territory: army slot', 'crates/aa-metagame/src/positions.rs', 'research/phase8-metagame/native-positions.json', None),
    0x954c60: ('territory: army slot helper', 'crates/aa-metagame/src/positions.rs', 'research/phase8-metagame/native-positions.json', None),
    0x9578c0: ('territory def: actor slot (+0x60)', 'crates/aa-metagame/src/positions.rs', 'research/phase8-metagame/native-positions.json', None),
    0x9552a0: ('territory reference position (-> 957DD0)', 'crates/aa-metagame/src/positions.rs', 'research/phase8-metagame/native-positions.json', None),
    0x957dd0: ('territory def position (alias: x from bounds by partner x; z from visual +0x28)', 'crates/aa-metagame/src/positions.rs', 'research/phase8-metagame/native-positions.json', 'visual z is an engine input'),
    0x955b70: ('place one army: solve at -1, z = 2.0, facing (0x3F34FDF4, 0xBF34FDF4), create 4FA030', 'crates/aa-metagame/src/positions.rs', 'research/phase8-metagame/native-positions.json', 'placements proven at PC53 only'),
    0x955820: ('respawn with the stored slot', 'crates/aa-metagame/src/positions.rs', 'research/phase8-metagame/native-positions.json', None),
    0x9626d0: ('realign on alias wrap (93EBF0 actor distance)', 'crates/aa-metagame/src/positions.rs', 'research/phase8-metagame/native-positions.json', None),
    0x962880: ('move waypoints (alias partner by 3D distance, solve at -1, link intermediate points)', 'crates/aa-metagame/src/positions.rs', 'research/phase8-metagame/native-positions.json', None),
    0x85daa0: ('sin lookup (4096 entries)', 'crates/aa-metagame/src/positions.rs', 'research/phase8-metagame/native-positions.json', 'aa_trig::lookup_sin'),
    0x85da80: ('cos lookup (4096 entries)', 'crates/aa-metagame/src/positions.rs', 'research/phase8-metagame/native-positions.json', 'aa_trig::lookup_cos'),
    0x9c8620: ('static initializer: cos/sin table object B39A48', None, 'research/phase8-metagame/native-positions.json', 'tables equal the exact binary32 roundings (mpmath, margin >= 282x the FSIN/FCOS error bound; trig-tables-exact.json) and aa-trig'),
    0x9c8640: ('static initializer: table object B39A60', None, 'research/phase8-metagame/native-positions.json', 'tables equal the exact binary32 roundings (mpmath, margin >= 282x the FSIN/FCOS error bound; trig-tables-exact.json) and aa-trig'),
})
ANNOTATIONS.update({
    0x8584a0: ('chunk file constructor (vtable AB4E38)', 'crates/aa-metagame/src/chunk_file.rs', 'research/phase8-metagame/native-chunks.json', None),
    0x859690: ('chunk file open (+0x34 base 0, +0x38 size, 858540, 859380)', 'crates/aa-metagame/src/chunk_file.rs', 'research/phase8-metagame/native-chunks.json', None),
    0x859380: ('read header and directory (new file: 859090 writes the header)', 'crates/aa-metagame/src/chunk_file.rs', 'research/phase8-metagame/native-chunks.json', None),
    0x859150: ('open a chunk for writing (name, compress flag)', 'crates/aa-metagame/src/chunk_file.rs', 'research/phase8-metagame/native-chunks.json', 'compressed chunks Unsupported'),
    0x858e40: ('chunk write', 'crates/aa-metagame/src/chunk_file.rs', 'research/phase8-metagame/native-chunks.json', None),
    0x858db0: ('chunk size update', 'crates/aa-metagame/src/chunk_file.rs', 'research/phase8-metagame/native-chunks.json', None),
    0x859030: ('open a chunk for reading (name, required)', 'crates/aa-metagame/src/chunk_file.rs', 'research/phase8-metagame/native-chunks.json', None),
    0x858c90: ('chunk read', 'crates/aa-metagame/src/chunk_file.rs', 'research/phase8-metagame/native-chunks.json', None),
    0x858f50: ('chunk seek', 'crates/aa-metagame/src/chunk_file.rs', 'research/phase8-metagame/native-chunks.json', None),
    0x858ed0: ('file seek', 'crates/aa-metagame/src/chunk_file.rs', 'research/phase8-metagame/native-chunks.json', None),
    0x859330: ('close', 'crates/aa-metagame/src/chunk_file.rs', 'research/phase8-metagame/native-chunks.json', None),
    0x859090: ('write header and directory', 'crates/aa-metagame/src/chunk_file.rs', 'research/phase8-metagame/native-chunks.json', 'flag bits 1..63 are uninitialised stack natively; written as 0'),
    0x8582b0: ('directory record', 'crates/aa-metagame/src/chunk_file.rs', 'research/phase8-metagame/native-chunks.json', None),
    0x8580e0: ('chunk descriptor', 'crates/aa-metagame/src/chunk_file.rs', 'research/phase8-metagame/native-chunks.json', None),
    0x858ba0: ('chunk error "File: %s Chunk: %s Offset: %ld - " + reason (throw)', 'crates/aa-metagame/src/chunk_file.rs', 'research/phase8-metagame/native-chunks.json', None),
    0x4fc600: ('map load: chunk order (METAGAME after KINGDOMS, before USED_REQUIRED_OBJECTS / STATIC_THINGS / DYNAMIC_THINGS)', 'crates/aa-metagame/src/chunk_file.rs', None, 'from the listing; METAGAME only for game type 4'),
    0x4f2db0: ('map save: chunk order', 'crates/aa-metagame/src/chunk_file.rs', None, 'from the listing'),
    0x60a140: ('MGState +0x48 writer (game type 4): 4BEDF0 name of the last world kingdom with +0x19C == 1, else empty', 'crates/aa-metagame/src/state.rs', None, 'callers 6125A0, 61E620; engine side, not ported (rts_winner is an MgState input)'),
    0x95ad60: ('event state constructor (95BE70 + 95AD60 in 939B90)', 'crates/aa-metagame/src/state.rs', 'research/phase8-metagame/native-turn.json', None),
    0x95be70: ('InitCreate: one event state per campaign event', 'crates/aa-metagame/src/state.rs', 'research/phase8-metagame/native-turn.json', None),
    0x95bc80: ('event effects in order (add_tech 95B630, add_actor 95B220, remove_tech 95B7F0, remove_actor 95B400, change_territory 95B9B0) and event message', 'crates/aa-metagame/src/turn.rs', 'research/phase8-metagame/native-turn.json', 'synthetic events (no shipped campaign has any)'),
    0x95b220: ('effect add_actor (955B70 place)', 'crates/aa-metagame/src/turn.rs', 'research/phase8-metagame/native-turn.json', None),
    0x95b400: ('effect remove_actor', 'crates/aa-metagame/src/turn.rs', 'research/phase8-metagame/native-turn.json', None),
    0x95b630: ('effect add_tech', 'crates/aa-metagame/src/turn.rs', 'research/phase8-metagame/native-turn.json', None),
    0x95b7f0: ('effect remove_tech', 'crates/aa-metagame/src/turn.rs', 'research/phase8-metagame/native-turn.json', None),
    0x95b9b0: ('effect change_territory', 'crates/aa-metagame/src/turn.rs', 'research/phase8-metagame/native-turn.json', None),
    0x95aed0: ('effect territory (given or one draw)', 'crates/aa-metagame/src/turn.rs', 'research/phase8-metagame/native-turn.json', None),
    0x95ae80: ('effect army type (given or one draw)', 'crates/aa-metagame/src/turn.rs', 'research/phase8-metagame/native-turn.json', None),
    0x95af40: ('effect technology (given or one draw)', 'crates/aa-metagame/src/turn.rs', 'research/phase8-metagame/native-turn.json', None),
    0x95add0: ('event roll (new round): fixed round or one draw vs round_probability; kingdom_turn or one draw over the kingdoms', 'crates/aa-metagame/src/turn.rs', 'research/phase8-metagame/native-turn.json', 'synthetic events'),
    0x95be20: ('event fire: triggered and the current kingdom is the event kingdom -> 95BC80', 'crates/aa-metagame/src/turn.rs', 'research/phase8-metagame/native-turn.json', 'synthetic events'),
    0x944d60: ('MGAI controller constructor (vtable AC8E74, 0x20 bytes)', 'crates/aa-metagame/src/mgai/controller.rs', 'research/phase8-metagame/native-mgai-controller-c*.json', None),
    0x944f90: ('MGAI controller: one KKC_MGAIPlayer per MG kingdom', 'crates/aa-metagame/src/mgai/controller.rs', 'research/phase8-metagame/native-mgai-controller-c*.json', None),
    0x945090: ('MGAI controller fiber body: players, PlayTurn of the current kingdom, turn-ended resets', 'crates/aa-metagame/src/mgai/controller.rs', 'research/phase8-metagame/native-mgai-controller-c*.json', None),
    0x944d20: ('MGAI controller: delete the players', 'crates/aa-metagame/src/mgai/controller.rs', 'research/phase8-metagame/native-mgai-controller-c*.json', None),
    0x944e60: ('MGAI controller: flush the command queue into orders (950470)', 'crates/aa-metagame/src/mgai/controller.rs', 'research/phase8-metagame/native-mgai-controller-c*.json', 'order submission 950770/9506D0 is a seam'),
    0x9505f0: ('order execution: issuer, 9525A0 (951D70 == OK), 954570', 'crates/aa-metagame/src/mgai/controller.rs', 'research/phase8-metagame/native-mgai-controller-c*.json', 'issuer resolution 74B3B0/765E00 is engine'),
    0x9525a0: ('order execution: validation gate (951D70 == OK)', 'crates/aa-metagame/src/mgai/controller.rs', 'research/phase8-metagame/native-mgai-controller-c*.json', None),
    0x4a5420: ('KKC_RMCData::Load: fields, map sizes, sub-loaders, [Heroes] (+0x80/+0x84 random heroes min/max)', 'crates/aa-metagame/src/rmc.rs', None, 'ported; all 63 shipped RMCs load (rmc-data.json); no native oracle'),
    0x4a0b60: ('RMCData constructor (vtable A15058)', 'crates/aa-metagame/src/rmc.rs', None, 'not oracle-proven'),
    0x4a19d0: ('RMC LoadRules (game_type, world rule defaults)', 'crates/aa-metagame/src/rmc.rs', None, 'not oracle-proven'),
    0x4a4410: ('RMC terrain sliders (+0x50)', 'crates/aa-metagame/src/rmc.rs', None, 'not oracle-proven'),
    0x4a1db0: ('RMC terrain noise (+0x5C)', 'crates/aa-metagame/src/rmc.rs', None, 'not oracle-proven'),
    0x4a4eb0: ('RMC kingdoms: kingdom infos, major +0xF8 / independent +0x104 slots, relations, KingdomsMax', 'crates/aa-metagame/src/rmc.rs', None, 'not oracle-proven; slots agree with data.rs RmcDefinition on every metagame RMC'),
    0x4a4600: ('RMC object groups in node order (DoodadGroup +0x68, ActorGroup +0x74, apply_to_all_major_kingdoms expansion)', 'crates/aa-metagame/src/rmc.rs', None, 'not oracle-proven'),
    0x4a2150: ('RMC group restrictions: exact group or prefix*; with no loaded match the group itself is compared ("Unknown group prefix" only on mismatch)', 'crates/aa-metagame/src/rmc.rs', None, 'self-match fallback read from the listing; needed by 8 shipped RMCs'),
    0x4a1cd0: ('RMC teams', 'crates/aa-metagame/src/rmc.rs', None, 'not oracle-proven'),
})
EXTRA = [0x85d840, 0x5065d0]
# Coarse areas, inferred from the assert/log anchors and referenced strings.
AREAS = [
    (0x932000, 0x9387a0, 'ui: MetaGameAppState/MetaGameInterface, decrees/orders'),
    (0x9387a0, 0x93b250, 'sim: KKC_MGState (InitCreate 939B90, save writer 938EE0, kingdom/territory lookups)'),
    (0x93b250, 0x941140, 'data+sim: MetaGameComponent load 93B250, data managers 93C0D0..93E2B0, army component runtime 93E8F0..93F5xx'),
    (0x941140, 0x941960, 'data: MGDivision load/tooltip'),
    (0x941960, 0x943330, 'ui: ChooseDynamicCampaignInterface'),
    (0x943330, 0x9505f0, 'ui: options, purchasing, start-battle dialog, RTS launcher 948EF0, history/help popups'),
    (0x9505f0, 0x950e40, 'ui: MetaGameOrders (TellMGCommand order)'),
    (0x950e40, 0x954570, 'sim: KKC_MGCommand (purchase, upgrade, move/attack, end turn, battles)'),
    (0x954570, 0x9583c0, 'sim: MG territory state (army lists, filters, quick-battle helpers)'),
    (0x9583c0, 0x95a000, 'data+sim: territory definition load, TerritoryPositionSolver'),
    (0x95a000, 0x95c000, 'sim+data: MG kingdom state (ctor 95A100), help popup data'),
    (0x95c000, 0x95ef00, 'data: DynamicCampaign/Kingdom/Team/Territory loaders'),
    (0x95ef00, 0x95f7c0, 'sim: history entry, KKC_QuickBattleSolver'),
    (0x95f7c0, 0x960950, 'data: help popup, MGAI parameters'),
    (0x960950, 0x962a40, 'data: map table props, shop lists, territory links'),
    (0x962a40, 0x965280, 'ai: KKC_MGAIPlayer'),
    (0x965280, 0x966e00, 'ui: history dialog, command buttons'),
    (0x966e00, 0x967a40, 'sim: KKC_MGBattleResolver, destroy behaviours'),
    (0x967a40, 0x968f70, 'data+sim: campaign events/effects, objectives'),
    (0x968f70, 0x96a000, 'ai: KKC_MGAIGoal'),
    (0x96a000, 0x96bb00, 'ui: object/player displays'),
]
def area(a):
    for lo, hi, name in AREAS:
        if lo <= a < hi:
            return name
    return None

def main():
    funcs = {}
    with open(NATIVE / 'functions.tsv', newline='') as fh:
        for row in csv.DictReader(fh, delimiter='\t'):
            a = int(row['address'], 16)
            if LO <= a < HI or a in EXTRA:
                funcs[a] = int(row['bodyBytes'])
    strings = {}
    with open(NATIVE / 'string-references.tsv', newline='', encoding='utf-8', errors='replace') as fh:
        for row in csv.DictReader(fh, delimiter='\t'):
            if not row['functionAddress']:
                continue
            a = int(row['functionAddress'], 16)
            if a in funcs:
                strings.setdefault(a, [])
                if row['value'] not in strings[a]:
                    strings[a].append(row['value'])
    anchor = {}
    for a, values in strings.items():
        files = sorted({v for v in values if v.startswith('.\\\\Source') or v.startswith('.\\Source')})
        if len(files) == 1:
            anchor[a] = files[0].replace('\\\\', '\\')
    order = sorted(funcs)
    rows = []
    for i, a in enumerate(order):
        src, how = anchor.get(a), 'assert/log string' if a in anchor else None
        if src is None and LO <= a < HI:
            before = next((anchor[b] for b in reversed(order[:i]) if b in anchor), None)
            after = next((anchor[b] for b in order[i + 1:] if b in anchor), None)
            if before is not None and before == after:
                src, how = before, 'contiguity (inferred)'
        role, ported, proven, notes = ANNOTATIONS.get(a, (None, None, None, None))
        rows.append({'address': f'{a:08x}', 'size': funcs[a], 'area': area(a), 'source_file': src, 'source_evidence': how,
                     'strings': strings.get(a, [])[:8], 'role': role, 'ported': ported, 'proven': proven, 'notes': notes})
    for a, (role, ported, proven, notes) in ANNOTATIONS.items():
        if a not in funcs:
            rows.append({'address': f'{a:08x}', 'size': None, 'area': area(a), 'source_file': None, 'source_evidence': None,
                         'strings': [], 'role': role, 'ported': ported, 'proven': proven, 'notes': notes})
    rows.sort(key=lambda r: r['address'])
    summary = {
        'functions': len(rows),
        'annotated': sum(r['role'] is not None for r in rows),
        'ported': sum(r['ported'] is not None for r in rows),
        'proven_by_oracle': sum(r['proven'] is not None for r in rows),
        'by_source_file': {},
        'by_area': {},
    }
    for r in rows:
        k = r['source_file'] or 'unattributed'
        summary['by_source_file'][k] = summary['by_source_file'].get(k, 0) + 1
        k = r['area'] or 'outside range'
        summary['by_area'][k] = summary['by_area'].get(k, 0) + 1
    out = {'schema': 1, 'scope': __doc__, 'range': [f'{LO:08x}', f'{HI:08x}'], 'summary': summary, 'functions': rows}
    p = HERE / 'inventory.json'
    p.write_text(json.dumps(out, indent=1) + '\n')
    print(json.dumps(summary, indent=1))

if __name__ == '__main__':
    main()
