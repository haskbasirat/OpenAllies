This checkpoint ports Application+10C/+110 and40CE10 SetState. These fields are
distinct from GameAppState+C and Session+E4/+F8. The actual application retains
one move-only State; no mutable current-state setter exists.

Requested0 returns immediately, even for an invalid current table index. Equal
positive state still calls OnEnter. A changed request calls diagnostic404F60,
rereads current+10C, and calls that node's OnLeave+18 when current is nonzero.
It rereads current again after OnLeave and writes that value to previous+110,
then writes the requested state to current+10C. It reads current for the final
table lookup and calls OnEnter+14. Callback writes survive all reached errors.
The table lookup/child calls are mandatory services; there are no null-owner
defaults or successful placeholder callbacks.

Services transports an opaque Node and the same borrowed Arg8/ArgC values.
These types need neither Clone nor Copy. The kernel cannot truncate a64-bit
MenuInterface or LaunchState pointer to a32-bit integer. Actual UI/controller,
node construction, notification handling and argument lifetime belong to the
concrete client integration, not this pure coordinator.

State::from_constructor covers only the two proven zero fields. The native
proof executes4178F0/4178F6 with explicit ESI allocation and EBX0 precondition,
and separately pins the constructor's actual xor-ebx producer. This is a field
projection; it does not claim to execute the4177E0 managers, threads, other
members or whole application constructor.

The oracle executes complete40CE10 for198 rows in separate original/v14 images
at all three x87 precisions, plus9 paired field-constructor rows:414 native
executions,41 instruction addresses.404F60 and node OnLeave/OnEnter are named
controlled callback seams. Cases include callback state mutations and required
service stops, target0, equal-positive reentry, current rereads and actual null
node dereference attempts. Those null attempts establish the native boundary;
portable Result errors do not claim native SEH/unwind parity. Both faulting
addresses also complete normally in the nonnull rows.

The exact production-source test replays every row and checks argument borrow
identity. A second test reenters set_state from OnLeave through the same owner
using the actual public setter; the outer previous store must see the nested
state. No UI widgets, Host or VWorld completion is inferred from these tests.
