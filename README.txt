AXIS & ALLIES RUST — DEVELOPMENT 07 TEST BUILD

DEVELOPER CONTINUATION: Read HANDOFF.txt in this workspace first. It records
the current source/build checkpoint, asset dependencies, tests, pending runtime
integration, historical-doc caveats and continuation commands for another chat.

This is an unfinished Rust reconstruction and a testable original-asset preview.
Full playable battles, AI, campaigns, original save compatibility, and multiplayer
are still unfinished. The recent engine work does not make those modes available.

START HERE
Open AxisAlliesRust.exe and choose the Live preview tab.
The included Start Live Preview.cmd opens that tab directly.

NEW INTERACTIVE PREVIEW
Choose original infantry from the USA, Britain, Germany, Japan, or Russia.
Load the actor, select an available original animation, and test playback,
pause/restart, visibility, team color, position and facing controls.
These controls exercise source model binding, resident actor rendering and posed
geometry. Position controls are preview transforms, not issued movement orders.
The preview actor identity and scene are explicitly diagnostic.

The Maps, Models and Animations browsers remain available. Map selection is
inspection only. Existing WASD/arrows, mouse-wheel zoom, Home/End zoom endpoints,
Q/E model rotation, UI scaling, and F11 fullscreen remain available as applicable.

ASSET REQUIREMENTS
Run this folder within the existing Axis and Alies installation. Keep its
assets-cache beside the executable. The application also needs the original
Data.rwd and the existing reconstruction directory, including recovered models,
maps, textures, original animation sources and runtime-expanded-v14 rule files.

The ZIP contains this test build and its cache. It is not a standalone complete
game for a PC that lacks the recovered original assets. No original game files,
saves, profiles or older Development releases are replaced by this package.

WHAT REMAINS LIMITED
Live preview currently uses the five recovered infantry rigs and the supported
original Idle/Jog/Attack1/Die1 clip family. Missing or unsupported bindings are
reported rather than replaced with a different model or guessed animation.
Broader animation acquisition, scene transitions, blending, root motion and full
gameplay-driven animation are unfinished. Renderer lighting, dynamic water,
shadows and other parity gaps described in the existing project documentation
remain unfinished.

Engine libraries also contain new restoration, actor, ability, resource,
collision, movement, scheduler and trigger work. These are partially integrated
and are not exposed as a finished game mode in this executable.

TECHNICAL
64-bit Windows executable; Direct3D 12 and Vulkan via wgpu.
Optional arguments: --backend dx12, --backend vulkan,
--live-preview usa_infantry [--clip jog] [--time 0.5],
--assets "E:\Games\Axis and Alies\reconstruction".
The included manifest and validation report identify the exact tested executable.
