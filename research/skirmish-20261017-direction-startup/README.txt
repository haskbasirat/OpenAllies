Actual direction startup capability
==================================

The new runtime actor_creation_startup::StartupInputs is an immutable,
move-only application capability. Its only public constructor accepts the
existing VerifiedStartupReceipt to attest the same captured native executable.
It deliberately never reads that receipt's source-loading arithmetic PC.

The actual Windows/v14 capture02 entered CRT initializer9C8680 and returned
to96ECD4 on the same thread. Both legacy and FXSAVE contexts independently
report control word027F at entry and return. No floating-point state was
written by the debugger. The captured table has2048 pairs/16384 bytes, SHA256
a0afa5e1cb34b5caedeadf9ced0020bdf3a8b4a5f553da33fe80b018176a1c02.

Every captured pair equals the prior embedded Double53 table. Single24 differs
at433 vectors. Extended64 happens to have identical table bytes, so equality
does not authorize choosing that PC; the observed027F control selectsDouble53.
The initializer's2048 count is a native random-heading semantic, not a unit,
memory or actor budget. Live arithmetic remains an independent precision input.

verify_reference.py reads the existing capture/table/reference executable and
pins three equal original/v14 code windows:9C8680,408ED0 and408D50. It does not
launch any program, write an image, rerun a debugger or change FP state. Its
default mode verifies immutable hardware-direction-reference.json byte-for-byte.

Reference hashes:
  capture.json:
    6e674da7f51963b98e3331a583150f5d5bf760c61ae9bac66322cd7ef66dc18e
  hardware-direction-reference.json:
    f97ee667f7773c7616b968b926593c06569a7b211122bbc649e6b69e17824e54
  all three prior direction-tables.bin:
    20826310088f1a222dd5e480efe7697a8e47814f7f5743175b26ca6238dc0e95

The Rust verifier checks those immutable digests, native executable target,
same-thread entry/return, stack return, successful cleanup, raw FP control
blocks, actual table count/hash and every selected DirectionTable pair. No
public raw/default/Clone/Deserialize constructor permits choosing another PC.
The live map_actor_creation entry now requires a borrowed StartupInputs; the
generic constructor's explicit-PC laboratory API remains a separate diagnostic.

Scope and verification status
-----------------------------
This is evidence of the isolated static initializer before WinMain/source
loading, not a capture of a Rust thread, complete source startup or a battle.
The existing Windows capture was performed by the root agent; this task only
verified it read-only. The final read-only verification passed. Both focused
Rust guard/vector tests passed in frozen-integration-tests-04.exe, SHA256
48a507152d449e44be087b0a5f2668b2691a4e4594840b0900d0b5aed78dc9b7.
The application and live creation callers compile against this sealed API.
Those two private verifier tests do not claim a successful public constructor
integration, complete actor startup or battle. Findings and exact source pins
are retained in findings.json.
