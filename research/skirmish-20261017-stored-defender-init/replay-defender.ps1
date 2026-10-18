$ErrorActionPreference='Stop'
$taskRoot=Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
Push-Location $taskRoot
try {
 $env:CARGO_MANIFEST_DIR=Join-Path $taskRoot 'crates/aa-actors'
 $json=(Resolve-Path target/s-borrowed-leader/release/deps/libserde_json-3f0b3bb60c709793.rlib).Path
 $actors=(Resolve-Path target/s-borrowed-leader/release/deps/libaa_actors-da01587d98fcfe3e.rlib).Path
 & rustc --edition=2024 --test research/skirmish-20261017-stored-defender-init/replay_defender.rs -L dependency=target/s-borrowed-leader/release/deps --extern "serde_json=$json" --extern "aa_actors=$actors" -o target/s-borrowed-leader/defender-tests.exe
 if($LASTEXITCODE-ne0){throw 'Defender exact core/test compile failed'}
 & target/s-borrowed-leader/defender-tests.exe --nocapture
 if($LASTEXITCODE-ne0){throw 'Defender exact core replay failed'}
} finally {Pop-Location}
