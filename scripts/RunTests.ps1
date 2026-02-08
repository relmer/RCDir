<#
.SYNOPSIS
    Runs tests for the RCDir project using Cargo.

.DESCRIPTION
    Runs the test suite for RCDir using cargo test.
    Supports running tests for specific platforms and configurations.

.PARAMETER Configuration
    The build configuration. Valid values are 'Debug' or 'Release'.
    Default: Debug

.PARAMETER Platform
    The target platform. Valid values are 'x64', 'ARM64', or 'Auto'.
    'Auto' detects the current OS architecture.
    Default: Auto

.EXAMPLE
    .\RunTests.ps1
    Runs Debug tests for the current architecture.

.EXAMPLE
    .\RunTests.ps1 -Configuration Release -Platform x64
    Runs Release tests for x64.

.NOTES
    Requires Rust toolchain with the appropriate target installed.
#>
[CmdletBinding()]
param(
    [ValidateSet('Debug', 'Release')]
    [string]$Configuration = 'Debug',

    [ValidateSet('x64', 'ARM64', 'Auto')]
    [string]$Platform = 'Auto'
)

# Resolve 'Auto' platform to actual architecture
if ($Platform -eq 'Auto') {
    if ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -eq [System.Runtime.InteropServices.Architecture]::Arm64) {
        $Platform = 'ARM64'
    } else {
        $Platform = 'x64'
    }
}

$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path $PSScriptRoot -Parent

function Get-RustTarget {
    param([string]$Platform)
    
    switch ($Platform) {
        'x64'   { return 'x86_64-pc-windows-msvc' }
        'ARM64' { return 'aarch64-pc-windows-msvc' }
        default { throw "Unknown platform: $Platform" }
    }
}

$rustTarget = Get-RustTarget -Platform $Platform

Push-Location $repoRoot

try {
    $cargoArgs = @('test', '--target', $rustTarget)
    
    if ($Configuration -eq 'Release') {
        $cargoArgs += '--release'
    }

    Write-Host "Running tests from $repoRoot" -ForegroundColor Cyan
    Write-Host "cargo $($cargoArgs -join ' ')" -ForegroundColor DarkGray

    & cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}
finally {
    Pop-Location
}
