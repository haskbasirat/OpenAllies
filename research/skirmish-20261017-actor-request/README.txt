ActorData request FX and local AbilityDefinition source projection
===============================================================

Production
----------
data_pre_request::State retains ONE actor_requests field per canonical Data.
Opaque Ability handles contain that Data identity plus actual list/index. No
raw or public Ability constructor exists. Foreign/dead Data owners fail through
the existing canonical directory. The moved source Bank and its real source PC
are required; the live/UI PC is never used for immutable source materialization.

prepare_actor_request_fields is called by the existing Object/Actor source
orchestrator, after selected Object fields. It compiles root WhenEnter/WhenLeave
using the existing compiler, then actual Character animations, Performer and
Defender local definition FX, Mover, Body OnDeathAbility/health FX, and Portal.
This is explicitly a selected source projection, NOT whole464BD0/4BF990 or
global source-load completion. Unrelated validation/source writes are not
advertised as complete. A failure latches and keeps actual prior graph/resource
owners and partial Draft; no retry fabricates a new source allocation.

The eight Actor heads are194/19C,24C/254,294/29C,598/5A0. Each component has its
own actual11-bucket first-CreateVFX context. Mover synthesizes Start->Stop;
Body46651E passes Health as BOTH source and destination. This reuses the SAME
existing recursive42B8E0 compiler algorithm. Body then rejects negative signed
health triggers. Portal has no synthesis call here. Native constructor-empty
heads are used only when that component is actually absent under the existing
single-declaration/canonical-source guard.

4BE420 first dereferences AbilityData+0 to the actual AbilityDefinition. It then
requests five lists at Definition70/78/80/88/90 in native order with unchanged
delta/mode. The source enum is actual4987B0. Local AbilityDefinition graphs share
one context across all five lists. Graph selection is revalidated against the
canonical Actor's actual local source nodes. Child AbilityDefinition is selected
first; ability_IDS is the native NULL-child branch. Positive shared definitions
remain a required distinct source-order/shared allocation owner, never a fake
copy of the outer Ability's fields. Positive derived ActorData534 also remains
required: constructor NULL is not proof after a reached post-loader writer.

Proof
-----
native_actor_request.py defaults to immutable compare; --write is author-only.
Original/v14 bytes and outputs agree at all PC24/53/64. Six fresh machine
instances execute six real enum constructors each, actual4624B0 Actor constructor
field writes (49C760 completed-Object-base transport explicitly intercepted),
4BE420 whole five-list loop, selected real464BD0/4BF840 source intervals with
real context allocation and42BF10/empty42B8E0, signed health validation and the
actual4BFDD7 child-first selection branch.

Native executions across both images:
  enum constructor calls36; Actor constructor calls6;
  five-list request runs288; selected source interval runs108;
  signed health runs48; inline/shared selection runs12;
  1652 distinct genuinely executed instruction pins.

The source intervals start with explicit enclosing owner/source registers and
stack; they are not whole ActorDataLoadData. Typed source count/iterator calls
are transports. Positive FX loaders stop at the precise required42B990 entry;
their already existing compiler proof is reused, not claimed anew here.
4BE420's42B8B0 per-list callbacks are named transports that record unchanged
delta/mode and precise failure prefixes. No Resource decoder is bypassed into
success. Intercepted addresses are listed separately from executed pins.

Four standalone tests include the exact production actor_request_core.rs:
real registered names/constructor heads, five-list callback/error prefixes,
component shared-context ordering/synthesis direction, and signed health tests.
They pass4/4. Native readonly replay passes. Scoped runtime --lib production
check passes; final log identifies the exact current tree checkpoint.

New runtime owner-refusal and ignored actual Farm/ger_corps source-graph tests
are saved for the parent's next warm cohort; NOT run or counted as passing in
this scope. No full concrete source adapter, wholeData/source phase, decoded
Sequence/model, resource barrier, VWorldAD or controllable battle claim.

Earlier Animation registry proof used a typed three-name input table. This
successor genuinely executes4986C0 and confirms those exact names without
rewriting or retroactively broadening the earlier immutable evidence.
