The modern battle renderer now consumes the actual NiCamera8737B0 matrix,
eye and direction through sealed `native_camera::backend::Snapshot` transport.
It does not rebuild a perspective matrix from the inspector's 43-degree FOV.
The four native rows are converted from native coordinates `[x,y,z]` to render
coordinates `[x,z,-y]`; the native clip depth remains unchanged.

`Frame::from_current` checks the actual World identity and camera generation.
The UI draw checks that generation again before and after submitting. The full
native viewport is admitted; a changed physical viewport requires its actual
native projection update. A non-full normalized viewport is explicitly required
until an actual viewport/scissor adapter exists. GPU dimension checks use the
actual device limit. There is no engine memory or texture budget here.

Only a successful draw creates `Rendered`. Its geometric picking ray uses the
same inverse projection and undoes the same installed native scene root once.
Picking rechecks camera/World lifetime and renderer allocation/scene epoch.
This is presentation geometry, not a port of native gameplay selection/order
rules. Inspector rendering is retained for inspection scenes and refuses an
installed native scene that lacks the actual native camera packet.

Validation is bounded. `client-check02.log` passes the current client test
typecheck with release/locked/offline/-j4. The exact production conversion
module was built by `projection_test_harness.rs` and passes 2/2 tests in
`projection-tests02.log`. It reads the existing independently generated
`native-camera-ni.json`: 150 rows total, 144 completed native child rows,
120 usable matrix rows and 24 deliberately zero-basis rows refused for
inverse picking. Six parent/attached required-child rows are excluded.
`projection-tests01.log` preserves the initial test's incorrect assumption
that all completed synthetic rows were invertible; production correctly
refused them, and the test now records that intended refusal.

No current native-camera Snapshot producer exists at this checkpoint. Its
private runtime fields cannot be fabricated by the renderer or a bare partial
Camera State. Leader owns the actual constructor/directory/current-camera and
VWorld association producer. Consequently no new native-camera GPU draw,
VWorld completion, AD,74FF50, activation or playable battle is claimed.
The previous headless GPU test predates this camera hook; this checkpoint is
source/typechecking plus the exact projection tests, without a new full link.

The additive `VWorldOwner::attach_pending_commands` is also typechecked here.
It binds actual50CAB0 pending ownership to the same VWorld's43B870 list using
the private ViewToken and current World revision. A child error retains the
original manager-detached payload. This is only the first child of4F0750;
the following native Thing traversal/factory/publication remains separate.
The43B870 native proof belongs to `view_fx_commands`' author; this checkpoint
does not independently certify that proof or later FX update/render behavior.
