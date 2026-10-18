$ErrorActionPreference='Stop'
$taskRoot=Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
Push-Location $taskRoot
try {
    $env:CARGO_MANIFEST_DIR=Join-Path $taskRoot 'crates/aa-runtime'
    $json=(Resolve-Path target/s-borrowed-leader/release/deps/libserde_json-3f0b3bb60c709793.rlib).Path
    & rustc --edition=2024 --test research/skirmish-20261017-audio-pre-request/replay_audio.rs -L dependency=target/s-borrowed-leader/release/deps --extern "serde_json=$json" -o target/s-borrowed-leader/audio-request-tests.exe
    if ($LASTEXITCODE -ne 0) {throw 'Audio source/core compilation failed'}
    & target/s-borrowed-leader/audio-request-tests.exe audio_data_pre_request --nocapture
    if ($LASTEXITCODE -ne 0) {throw 'Audio exact-source/core replay failed'}
} finally {Pop-Location}
