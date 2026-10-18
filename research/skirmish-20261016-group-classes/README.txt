ORGANIZATION578A30 SOURCE AUDIT -- 2026-10-16

CORRECTION TO THE INITIAL TASK
ActorData+1EC/+1F0 is the ordered Performer AbilityData pointer vector/count,
not ActorClass references. Each local AbilityData+0 is an AbilityDefinition
pointer. Organization578A30 reads that definition+54 and tests bit3, BOMBARD.
The historical folder name follows the initial task, not the recovered type.
No production files were modified for this evidence package.

EXACT PRODUCER / CONSUMER
464BD0 at4655E8 passes the literal Ability to the PerformerComponent child
iterator.465622 allocates0x78,46563E calls AbilityData constructor4BE9B0,
and465666 calls4BF990 to load one entry.4656AB reads actor+1F0;4656C0 reads
actor+1EC.4656C9/4656CB dereference the local pointer and obtain the shared
or inline AbilityDefinition.4656CD compares its+10 name against previous
definitions using native wcscmp96CB83. Matching names log a warning4656FD;
they are still appended46570E..465783. There is no deduplication.
This list is later consumed by Performer constructor57F4D0, which invokes
4C8650 for each entry in the same order. Constructor4624B0 initializes its
pointer/count/capacity fields to an allocated12-byte vector/count0/capacity3.

AbilityDefinition vtable A215EC[0] is LoadData4BF390. Its required Flags child
is entered4BF67F..4BF68D.4BF690..4BF6BC reads11 typed booleans in index order
and ORs1<<index into definition+54. Registration4BF200 supplies these names:
0VORPAL,1SPELL,2ENCHANTMENT,3BOMBARD,4CENTER_ON_SOURCE,5TARGET_LOCATION,
6HEAL,7IGNORE_FOR_CV,8SUMMON_MULTIPLE,9SUMMON_UNIQUE,10PRIMARY_ISLAND_ONLY.
Further effect-derived bits are applied by4BEE20; existing Rust loaded.flags
contains those too.578A30 only tests bit3.

SOURCE SCHEMA
Templates/Components/template_performer.tgi declares [list Ability Template=Ability].
Templates/template_actorability.tgi provides AbilityShared, with ability_IDS,
and AbilityLocal, with nested [AbilityDefinition Template=AbilityDefinition].
The native4BF990 local child takes precedence. If that child is absent,
ability_IDS is resolved once through48D500; unknown lookup throws410C40.
The local path may generate/insert a named definition or allocate an unnamed
definition; it is not a global ActorClass lookup. Source inheritance and
ordered list expansion belong to the existing native typed loader.
Templates/template_abilitydefinition.tgi [Flags] declares bombard=false;
the AA ability templates override it for bombard definitions. Missing typed
fields are not invented defaults: their assigned template values are required.

REUSE / IDENTITY
Catalog::actor_bindings(id, precision) already retains
ActorBindings.performer:Option<LoadedPerformer>. Its abilities() slice retains
the local list order and LoadedAbility.flags is the exact field required here.
DEFINITION_FLAGS[3] is already bombard. A second ActorClass abstraction would
misrepresent this data and should not be introduced.

The group Element source identity is already a native-resolved ActorData ID.
Call require_untranslated_actor_identity before actor_bindings, because the
latter accepts lookup strings and later calls other lookup-based helpers.
An existing canonical IDS shadowed by Translate must not translate again.

One existing shared-reference limitation was reported to the parent:
native_ability_inputs uses resolved_definition for ability_IDS, an exact-source
scan, whereas native4BF990 uses48D500 with one-hop translations. Alias-only
unknowns error, but a registered source IDS shadowed by a translation could
silently select the old source. Before reusing loaded flags under this bounded
implementation, validate the native generic registry target against the source
definition selected, with C-string prefix/case rules; fail explicitly on any
unsupported mismatch. Alternatively port that lookup narrowly with dedicated
native alias tests. Do not resolve an already canonical target a second time.

EXECUTED PROOF
native_ability_list.py -> native-ability-list.json executes63 bounded original
cases:18 ordered-list cases and45 flag cases across x87PC24/53/64. Every visited
native instruction is matched against original and v14 unchanged bytes.
Cases cover empty/one/many abilities, repeated names, case-different names,
vector growth, every flag bit, all/alternating flags, and nonzero255 bool byte.
All list cases preserve every ordered input, including repeats. Duplicate
comparison is case-sensitive; same/Same/same generates one warning, not three.

SEAMS / LIMITS
The supplied ActorData starts with constructor-shaped empty vector fields.
The original local AbilityData constructor executes; full ActorData constructor
does not. Original child enumeration receives supplied ordered nodes;4BF990
is a seam returning supplied completed definitions, not complete ability-load
or alias proof. Allocation and vector growth are explicit seams; growth copies
the prior list prefix. Warning logging is recorded without host I/O.
Original flag registration and bit loop execute; their registration allocation,
required Flags-node lookup and typed boolean read are leaves supplied by the
harness. Ordinary pagezero accesses are rejected, allowing only FS-prefix SEH.
The full4BF390 source loader is statically pinned, not fully executed here.
No new Rust build, source-data census, world tick or gameplay test was run by
this audit. Parent owns runtime binding and its acceptance tests.

REPRODUCE FROM rust-rebuild
python research/skirmish-20261016-group-classes/native_ability_list.py

PINS
Original AA.exe SHA256:
5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26
v14 AA.exe SHA256:
3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1
Detailed instruction windows/source hashes are in native-ability-list.json
and findings.json. Original files were read only.
