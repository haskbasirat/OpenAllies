# Stored member command522890

`stored_member_command::run` adapts the existing
`aa_mind::cai_member_command::run` to the actual split ActorStore/Arena,
moved Bank, retained World Runtime, queues, scheduler owners, RNG, clock and
inventory. `run_on_owners` forwards the same algorithm after a caller has
already split the World. There is one current command owner and one delivery
path, using `player_order_frame::{Fields,Services}` and the actual scoped
`DefinitionBankLoan`; no LoadShell, temporary Runtime or alternate Bank is
constructed.

The original Organization allocation is captured by an identity-only receipt.
The adapter reads its actual BC/C0 point, the current member Element slot,
the current Organization offsets78, and current Regiment sleep point2B4/2B8.
Tile queries use World.zones geometry, then the actual pool/tile set with the
existing54EF40 kernel. Zone4 is captured as owner/key and its current actual
fields are reread, rather than copied into another zone. Pose and flags route
through the actual staged or published placement owner.

The no-Organization4CEDE0 branch compares the actual Mover slot3 with the
typed current active component. The target provider is Mind slot4 virtual80,
not Denizen: Element535EE0 reads90/94, Regiment524890 reads7C/80 and
Aircraft51C230 reads4C/50 plus54. Aircraft X alone copies through x87; the
existing hardware-correct `cai_thinks::x87_copy` handles signalling NaNs.
The finite native rows do not claim Unicorn's signalling-NaN behavior is
correct. Base virtual80 remains the actual purecall boundary.

Move and Stop input fields follow native4D9E20/4CA760. The shared predefined
command allocator creates the actual PlainCommand in Runtime.commands.
Delivery calls the existing4D9B60 adapter with actual Services, then releases
the constructor reference on success or error. `stop_on_owners` reuses that
same path for rejected commands and returns the real CommandDelivery byte;
the522890 parent intentionally ignores that byte as native does.

Positive flag1 calls the existing57C680 `element_position::set_position`
kernel over actual fields. Its inactive and unchanged-goal branches execute
normally; native goal stores precede required relocation/interpolation
children. No extra always-successful predicate was added. Positive4CEDE0
Organization traversal, full ground relocation, interpolation, scheduling and
other reached command backend policies remain explicit if unavailable.

New native evidence:387 rows,774 original/v14 executions at all three PCs,
105 executed address-byte pins and three immutable vtable data pins. The
actual concrete constructor result is reused from the frozen native Mind
constructor fixture. `native_member_command_fields.py` has no intercepted
numeric/getter child. It proves the whole5543F0 query and actual derived
virtual80 targets. Exact production fields tests pass3/3, including the
separate logical hardware-copy contract and missing-field/purecall prefixes.

The parent's separate522890 fixture contains288 pairs/576 executions and
298 pins, including the actual4D9E20/4CA760 constructor fields. It declares
the source/zone/activity/target/command backend seams explicitly. We reuse
that parent coordinator; we do not relabel its successful fixture callbacks
as live-owner acceptance. Existing element-position native regressions pass
4/4 after the new binding. Whole concrete ActorAccess execution and natural
mission acceptance remain pending unless a later integration report records
them. No whole runtime/client test link is performed for this checkpoint.

Reproduce from the repository root:

```powershell
$env:PYTHONPATH='E:\Games\Axis and Alies\reconstruction\tools\python-packages'
python research/skirmish-20261017-stored-member-command/native_member_command_fields.py
& research/skirmish-20261017-stored-member-command/exact-fields-tests.exe --nocapture
```

Parent cohort filter: `stored_member_command::fields::tests::`.
