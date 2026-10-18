SAI observer lifecycle during saved-map Element attachment
Date: 2026-10-16
Scope: read-only native evidence plus an executed inactive-branch oracle.
No production source files, native game files, saves, or profiles were changed.

RESULT
The native managed LoadGame task tears down the previous GWorld BEFORE all
three saved-map loading branches. That teardown deactivates a previously
active global SAI observer. During the subsequent GWorld::Load / embedded
Organization::Load -> Element::Attach, the observer exists but is inactive
under this lifecycle, so 7A0C20 performs no actor observation. A first load
starts with the same inactive byte established by the SAI constructor.

This supports an explicitly INACTIVE loading context. It does not support
silently omitting an active observer, invoking GWorld::Load directly over a
running world, or claiming that SAI simulation is implemented.

STATIC LIFECYCLE PROOF (not executed as a complete lifecycle)
1. SAI constructor 7A0AA0 publishes itself at B36528 and clears byte +20
   at 7A0B02. The application initialization pseudocode 00419fb5.c calls
   this constructor and retains the object at application+118. The SAI
   destructor 7A0840 clears the singleton through B40D48; ordinary map
   teardown destroys SAIMap, not the application SAI singleton.

2. LoadGame task constructor 59E6F0 installs vtable A43794. Its +1C entry
   is 5A4280, KKC_ResourceManagerTaskLoadGame::Run. At 5A4307 it calls
   Sim::DestroyGWorld (76AB80) before its mode dispatch:
     mode 1: 5A4323 -> 5A3E20 (editor)
     mode 2: 5A432F -> 5A3F20 (alternate load)
     else:   5A4336 -> 5A3EB0 (normal load)
   Each branch calls MapResource::Load 5A2B90, respectively at 5A3E8E,
   5A3F3B, and 5A3ECB. MapResource calls GWorld::Load 4FC600 at 5A3031.
   This proves subsequent-save teardown even if a caller bypasses the
   higher Session::CreateLaunchTask entry point.

3. Previous-world teardown call chain:
     76AB80:76ABE5 -> GWorld::Uninit 4F8690
     4F8690:4F89E8 -> GMap destructor 555030 (GWorld+30)
     555030:55506B -> GMap+4C SAIMap virtual destructor
     A9D3E4[0] = 7ACF30; 7ACF33 -> 7ACE50
     7ACE7C -> SAIMap::ClearStaticMapAnalysis 7AC9A0
     7AC9C5 reads SAIMap+5; if nonzero, 7AC9D1 loads [B36528]
     and 7AC9D7 calls SAI deactivation 79FF10
     79FFC3 stores zero to SAI+20.
   If there was no preceding world, the task safely skips world teardown.

4. Why a previously ACTIVE observer takes that conditional reset:
   SAI activation 7A0D40 calls 7ABC30 with [B3652C] at 7A0D7F;
   7ABC53 sets the current SAIMap+5 to 1. Only later, at 7A0E25,
   activation sets SAI+20 to 1. Therefore the ordinary active-world state
   has the SAIMap flag needed by the previous-world destructor.

5. Reactivation follows successful load task completion, not record parsing:
   The LoadGame task completes via 5A442D. 5A445F calls 5B18C0 with
   state 2 for success / 3 for failure; 5B18C0 stores task+14.
   Session::CheckForFinishedTask 775DB0 checks task+14 == 2 at 775E59,
   applies the 76AEB0 task-type gate, then calls 7757F0 at 775E76.
   Session::OnGWorldReadyForState 7757F0 calls 7A0D40 at 775B63 for
   session+E4 states 2/5, or 775BC3 for state 4. Other states need not
   activate SAI. The two calls are the only relative E8-call byte matches
   to 7A0D40 in both complete PE images; both decode as real calls.
   No absolute 7A0D40 pointer byte sequence occurs in either PE image.
   This does not purport to exclude arbitrary computed indirect calls.

6. Element::Attach 5705A0 writes Element+18,+10,+1C, runs actor refresh
   4D1350 and 4CF0F0, then loads [B36528] at 5705C5. If nonnull it
   calls 7A0C20 at 5705D3. Only this last observer callback is established
   as a no-op by the inactive context; the earlier refresh calls still
   require their own faithful implementations and proof.

EXECUTED PROOF
inactive-oracle.json contains 42 successful native executions: seven actor
pointer values x PC24/PC53/PC64 x original/v14 PE. Actor values include null,
1, DEADBEEF, FFFFFFFF, a mapped actor, an interior address, and the last byte
of its mapped page. The observer object exists and byte +20 is zero.

All runs execute exactly:
  7A0C20 push edi
  7A0C21 mov edi,ecx
  7A0C23 mov al,[edi+20]
  7A0C26 test al,al
  7A0C28 je 7A0D33
  7A0D33 pop edi
  7A0D34 ret 4

CPU data-read hooks prove the only reads are observer+20, the saved EDI,
and the return address. The actor argument is not even read from the stack.
There are no actor reads, no observer writes, no calls, and no seams. The
only write is push edi to the stack. Both allocated object pages remain
byte-identical; EDI is preserved and ESP advances by eight bytes.

Page zero is unmapped, so a null actor dereference cannot be disguised as
an SEH word. No FS-prefixed/SEH instructions are used in this branch.
Unicorn 2.1.4 and Capstone 5.0.7 were used. No floating-point instructions
execute; the three precision settings are coverage, not floating proof.

ARTIFACTS
- native_observer_lifecycle.py: reproducible static export + narrow oracle.
- lifecycle-static.json: full PE hashes, 22 identical original/v14 byte
  windows with disassembly, three vtables, activation-reference scan.
- lifecycle-instructions.txt: readable byte-pinned instruction windows.
- inactive-oracle.json: every executed instruction, memory event and result.
- findings.json: concise conclusions, boundaries and artifact hashes.

REPRODUCE
  cd /d "E:\Games\Axis and Alies\rust-rebuild"
  python research/resume-20261016-element-attach/observer/native_observer_lifecycle.py

The script reads both native EXEs and writes evidence only in this folder.
It pins original SHA256:
  5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26
and v14 SHA256:
  3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1

LIMITS
The complete teardown/load/activation lifecycle was inspected statically,
not executed in an emulator or live game. The oracle executes only the
inactive callback. No active classification, SAI AI, full-map load task,
Rust adapter, Rust tests/build, or gameplay-parity claim follows from this.
The load adapter must make its lifecycle assumption explicit; Active must
remain unsupported until its own machinery is ported and proven.
