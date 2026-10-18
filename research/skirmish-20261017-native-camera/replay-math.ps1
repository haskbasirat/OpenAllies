$ErrorActionPreference = 'Stop'
$taskRoot = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
Push-Location $taskRoot
try {
    # Reuses this agent's accepted non-LTO dependency artifacts. No Cargo
    # workspace/runtime rebuild, native game process, profile or SDK is used.
    & rustc --edition=2024 --test research/skirmish-20261017-native-camera/standalone_camera.rs -L dependency=target/s-graphics-review/release/deps --extern aa_numeric=target/s-graphics-review/release/deps/libaa_numeric-55927aa222c0a1ba.rlib --extern aa_trig=target/s-graphics-review/release/deps/libaa_trig-04930027980b147e.rlib --extern serde_json=target/s-graphics-review/release/deps/libserde_json-3f0b3bb60c709793.rlib -o target/s-graphics-review/native-camera-tests.exe
    if ($LASTEXITCODE -ne 0) { throw 'Camera exact-source test compilation failed' }
    & target/s-graphics-review/native-camera-tests.exe --nocapture
    if ($LASTEXITCODE -ne 0) { throw 'Camera exact-source replay failed' }
} finally { Pop-Location }
