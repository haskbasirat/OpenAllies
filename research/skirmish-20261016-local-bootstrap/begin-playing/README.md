# Concrete local BeginPlaying owners and native caller slices

These modules continue the historical Host checkpoint through its actual
retained players, Sim and order node. They do not mark a battle ready.

* `local_begin_play.rs` runs773E30 over the same SessionBC roster, actual
  ActorStore and live Kingdom slots. Find4F48E0 is case-sensitive, first-match
  slot order.765E10 updates the actual player cache/VWorld.50E0D0 adjusts to
  Kingdom194/198, not ActorDC/E0. External creator88, glyph, editor-Nation and
  diagnostic owners remain required at their reached callbacks.
* `live_map_strings.rs` ports49AE00 for an existing unique ASCII registered
  key: pair+4 is assigned before global entry+10. Missing-entry construction,
  non-ASCII/NUL hashing and ambiguous registrations refuse explicitly after
  the committed pair store. The SAME Localization is mutated; provenance is
  retained as the declaration's origin.
* `local_sai_setup.rs` binds766960(false) to the real AddService facade, actual
  Sim world slot, player28, source/controller arena and GameAppState. Lookup
  precedes the null-World/null-Kingdom exits. The native caller uses profile10
  name for both normal SetupAI display/profile strings; editor4 alone may use
  actual Nation10 for display. Selected profile8 IDs are used for diagnostics.
  Session4/8/C is one growable queue, initially empty with capacity four.
  Session108 is an unrelated CString. There is no four-player engine cap.
* `local_host.rs` retains constructor28C/2CD/2D0/2DC/26C and reset writers.
 751770 clears28C/2CD/2DC, preserving2D0. RuntimeBorrow.frame directly loans
  the same fields to the shared frame kernel. Running2C0 uses the actual
  75B126 Host store; Battle never supplies a guessed true value.
* Session1C owns the move-only film Resource; Node268 is a weak typed handle
  to the same stream.74D5F0 stores268 before clearing only the actual mirror
  writer274/278/27C.74B310 only detaches. No250/resume fields are reset. Film
  resource creation, header, map strings, roster and rotation must still run
  through their concrete source-backed producers before recording is live.

Native fixtures run both original and v14 with PC24/53/64. Every executed byte
window is pinned and compared across executables:

| Fixture | Rows | Qualified scope |
| --- | ---: | --- |
| native-prepare-players.json |42| Whole773E30; actual Find/765E10; named external AI/name/creator/glyph/diagnostic leaves |
| native-map-string-set.json |72| Whole49AE00 pair and existing-entry writes; named4BAA20 registered lookup |
| native-setup-ai.json |54| Actual778F20(false)/778730/7756E0; append/growth through9 triples and embeddedNUL |
| native-create-sai.json |42| Whole766960(false) and actual queued writer; named48D500/79FC40/diagnostic seams |
| native-node-frame-fields.json |12| Constructor XOR/MOV projection, whole fresh751770, whole74D5F0/74B310 |

These are successful allocation/CString seams, not SEH/allocator-reentry or
whole-engine startup proofs. The Rust tests replay all fixture rows and
reached error prefixes. Historical failed logs remain preserved. The first
MapString Rust retry was blocked by foreign in-flight compilation errors;
it is not recorded as an owned behavioral failure. Final checkpoint metadata
records the actual successful tests and source hashes after freeze.

The current Host prefix still ends75B20E. Actual selected launch mode/map/
options/settings,77A950, application transition784220, creator88 selection,
complete7757F0 film/AI world-ready and activation remain separate integration
work. Native replay success here does not claim interactive battles,
network transport, independent internet tests or complete game parity.
