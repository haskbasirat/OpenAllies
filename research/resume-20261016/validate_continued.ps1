param([ValidateSet('regressions','actors','mind','runtime','census','compile')][string]$Phase='regressions')
$ErrorActionPreference='Stop'
$taskRoot=Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
Set-Location -LiteralPath $taskRoot
$env:AA_GAME_ROOT=Split-Path $taskRoot -Parent
$env:CARGO_TARGET_DIR=Join-Path $taskRoot 'target/s-owner-recruitment'
$taskOutput=Join-Path $taskRoot 'qa-output/resume-20261016'
function Invoke-ScopedCargo([string]$Name,[string[]]$Arguments) {
    $taskLog=Join-Path $taskOutput ('continued-'+$Name+'.log')
    & cargo @Arguments *> $taskLog
    $taskCode=$LASTEXITCODE
    Get-Content -LiteralPath $taskLog | Select-Object -Last 32
    if($taskCode -ne 0){throw "Scoped cargo failed: $Name (exit $taskCode); see $taskLog"}
}
switch -Regex ($Phase){
    '^(regressions|actors)$' {
        Invoke-ScopedCargo 'actors' @('test','-p','aa-actors','--release','--locked','--offline','-j','4',
            '--test','native_owner_recruitment','--test','native_research_actions',
            '--test','native_structure_actions','--test','native_denizen_actions',
            '--test','native_construction_actions','--test','native_ability_flags')
    }
    '^(regressions|mind)$' {
        Invoke-ScopedCargo 'mind' @('test','-p','aa-mind','--release','--locked','--offline','-j','4',
            '--test','regiment_context_native','--test','regiment_saved_load_native',
            '--test','saved_load_native','--test','saved_commands_native')
    }
    '^(regressions|runtime)$' {
        Invoke-ScopedCargo 'runtime-lib' @('test','-p','aa-runtime','--lib','--release','--locked','--offline','-j','4',
            '--','--include-ignored')
        Invoke-ScopedCargo 'runtime-integration' @('test','-p','aa-runtime','--release','--locked','--offline','-j','4',
            '--test','skirmish_restore','--test','skirmish_start','--test','saved_command_ownership',
            '--test','element_attach_restore','--test','load_tail_world','--test','world_start_world',
            '--','--include-ignored')
    }
    '^census$' {
        $env:AA_RESTORE_NATIVE_LOAD_MODE='1'
        $env:AA_RESTORE_INVENTORY_OUT=Join-Path $taskOutput 'continued-mode1-inventory.json'
        Invoke-ScopedCargo 'mode1-inventory' @('test','-p','aa-runtime','--release','--locked','--offline','-j','4',
            '--test','restore_inventory','--','--ignored','--nocapture')
    }
    '^compile$' {
        # Reuse the root's prior complete workspace metadata cache; no other
        # writer/build owns this target at the continuation checkpoint.
        $env:CARGO_TARGET_DIR=Join-Path $taskRoot 'target/s-definitions'
        Invoke-ScopedCargo 'workspace-check' @('check','--workspace','--all-targets','--release','--locked','--offline','-j','4')
    }
}
