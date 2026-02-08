<#
.SYNOPSIS
    Builds the RCDir project using Cargo.

.DESCRIPTION
    This script builds the RCDir Rust project using Cargo.
    It supports building for x64 and ARM64 platforms in Debug or Release configurations.
    The script automatically detects the current architecture when using -Platform Auto.

.PARAMETER Configuration
    The build configuration. Valid values are 'Debug' or 'Release'.
    Debug corresponds to default Cargo build, Release uses --release flag.
    Default: Debug

.PARAMETER Platform
    The target platform. Valid values are 'x64', 'ARM64', or 'Auto'.
    'Auto' detects the current OS architecture.
    Default: Auto

.PARAMETER Target
    The build target. Valid values are:
      - Build            Build the project (default)
      - Clean            Clean build outputs
      - Rebuild          Clean and rebuild
      - BuildAllRelease  Build Release for all platforms (x64 and ARM64)
      - CleanAll         Clean all build outputs
      - RebuildAllRelease  Rebuild Release for all platforms
      - Clippy           Run clippy lints
    Default: Build

.EXAMPLE
    .\Build.ps1
    Builds Debug configuration for the current architecture.

.EXAMPLE
    .\Build.ps1 -Configuration Release -Platform x64
    Builds Release configuration for x64.

.EXAMPLE
    .\Build.ps1 -Target BuildAllRelease
    Builds Release configuration for both x64 and ARM64 platforms.

.EXAMPLE
    .\Build.ps1 -Target Clean
    Cleans the build outputs.

.NOTES
    Requires Rust toolchain with both x86_64-pc-windows-msvc and aarch64-pc-windows-msvc targets.
    Install targets with: rustup target add x86_64-pc-windows-msvc aarch64-pc-windows-msvc
#>
param(
    [ValidateSet('Debug', 'Release')]
    [string]$Configuration = 'Debug',

    [ValidateSet('x64', 'ARM64', 'Auto')]
    [string]$Platform = 'Auto',

    [ValidateSet('Build', 'Clean', 'Rebuild', 'BuildAllRelease', 'CleanAll', 'RebuildAllRelease', 'Clippy')]
    [string]$Target = 'Build'
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

$script:BuildResults = @()

function Get-RustTarget {
    param([string]$Platform)
    
    switch ($Platform) {
        'x64'   { return 'x86_64-pc-windows-msvc' }
        'ARM64' { return 'aarch64-pc-windows-msvc' }
        default { throw "Unknown platform: $Platform" }
    }
}

function Test-RustTargetInstalled {
    param([string]$RustTarget)
    
    $installedTargets = rustup target list --installed 2>&1
    return $installedTargets -contains $RustTarget
}

function Add-BuildResult {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Configuration,
        [Parameter(Mandatory = $true)]
        [string]$Platform,
        [Parameter(Mandatory = $true)]
        [string]$Target,
        [Parameter(Mandatory = $true)]
        [ValidateSet('Succeeded', 'Failed', 'Skipped', 'Warning')]
        [string]$Status,
        [int]$ExitCode = 0,
        [TimeSpan]$Duration,
        [string]$Message
    )

    $script:BuildResults += [PSCustomObject]@{
        Configuration = $Configuration
        Platform      = $Platform
        Target        = $Target
        Status        = $Status
        ExitCode      = $ExitCode
        Duration      = $Duration
        Message       = $Message
    }
}

function Write-BuildSummary {
    if (-not $script:BuildResults -or $script:BuildResults.Count -eq 0) {
        return
    }

    Write-Host ''
    Write-Host 'SUMMARY' -ForegroundColor White

    foreach ($r in $script:BuildResults) {
        $statusText = $r.Status.ToUpperInvariant()
        $label      = "{0}|{1} {2}" -f $r.Configuration, $r.Platform, $r.Target
        $timeText   = ''
        $details    = ''

        if ($r.Duration -and $r.Duration -gt [TimeSpan]::Zero -and $r.Status -ne 'Skipped') {
            $minutes  = [int][Math]::Floor($r.Duration.TotalMinutes)
            $timeText = " ({0:00}:{1:00}.{2:000})" -f $minutes, $r.Duration.Seconds, $r.Duration.Milliseconds
        }

        if ($r.Message) {
            $details = " - {0}" -f $r.Message
        }
        elseif ($r.ExitCode -ne 0) {
            $details = " - ExitCode {0}" -f $r.ExitCode
        }

        $line = "{0,-20} {1}{2}{3}" -f $label, $statusText, $timeText, $details

        switch ($r.Status) {
            'Succeeded' { Write-Host $line -ForegroundColor Green }
            'Failed'    { Write-Host $line -ForegroundColor Red }
            'Warning'   { Write-Host $line -ForegroundColor Yellow }
            'Skipped'   { Write-Host $line -ForegroundColor Cyan }
            default     { Write-Host $line }
        }
    }
}

function Invoke-CargoBuild {
    param(
        [string]$Configuration,
        [string]$Platform,
        [string]$CargoCommand = 'build'
    )

    $rustTarget = Get-RustTarget -Platform $Platform
    
    $cargoArgs = @($CargoCommand, '--target', $rustTarget)
    
    if ($Configuration -eq 'Release') {
        $cargoArgs += '--release'
    }

    Write-Host "cargo $($cargoArgs -join ' ')" -ForegroundColor Cyan
    
    $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
    & cargo @cargoArgs
    $exitCode = $LASTEXITCODE
    $stopwatch.Stop()

    return @{
        ExitCode = $exitCode
        Duration = $stopwatch.Elapsed
    }
}

$repoRoot = Split-Path $PSScriptRoot -Parent

# Change to repo root for Cargo
Push-Location $repoRoot

$scriptExitCode = 0

try {
    # Verify cargo is available
    $cargoVersion = cargo --version 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw 'Cargo not found. Install Rust from https://rustup.rs/'
    }
    Write-Host "Using $cargoVersion" -ForegroundColor DarkGray

    if ($Target -eq 'BuildAllRelease' -or $Target -eq 'CleanAll' -or $Target -eq 'RebuildAllRelease') {
        $platformsToBuild = @('x64', 'ARM64')
        
        # Check which targets are installed
        foreach ($plat in @('x64', 'ARM64')) {
            $rustTarget = Get-RustTarget -Platform $plat
            if (-not (Test-RustTargetInstalled -RustTarget $rustTarget)) {
                Write-Host "$plat Rust target ($rustTarget) not installed; skipping." -ForegroundColor Cyan
                $platformsToBuild = $platformsToBuild | Where-Object { $_ -ne $plat }
                
                if ($Target -ne 'CleanAll') {
                    Add-BuildResult -Configuration 'Release' -Platform $plat -Target $Target -Status 'Skipped' -Message "Target $rustTarget not installed (run: rustup target add $rustTarget)"
                }
            }
        }

        if ($Target -eq 'CleanAll') {
            Write-Host 'cargo clean' -ForegroundColor Cyan
            cargo clean
            if ($LASTEXITCODE -eq 0) {
                Add-BuildResult -Configuration 'All' -Platform 'All' -Target 'Clean' -Status 'Succeeded'
            } else {
                Add-BuildResult -Configuration 'All' -Platform 'All' -Target 'Clean' -Status 'Failed' -ExitCode $LASTEXITCODE
                $scriptExitCode = $LASTEXITCODE
            }
        }
        else {
            foreach ($platformToBuild in $platformsToBuild) {
                if ($Target -eq 'RebuildAllRelease') {
                    # For rebuild, clean the specific target first
                    $rustTarget = Get-RustTarget -Platform $platformToBuild
                    Write-Host "cargo clean --target $rustTarget --release" -ForegroundColor Cyan
                    cargo clean --target $rustTarget --release
                }

                $result = Invoke-CargoBuild -Configuration 'Release' -Platform $platformToBuild

                if ($result.ExitCode -ne 0) {
                    Add-BuildResult -Configuration 'Release' -Platform $platformToBuild -Target 'Build' -Status 'Failed' -ExitCode $result.ExitCode -Duration $result.Duration
                    $scriptExitCode = $result.ExitCode
                    break
                }

                Add-BuildResult -Configuration 'Release' -Platform $platformToBuild -Target 'Build' -Status 'Succeeded' -Duration $result.Duration
            }
        }
    }
    elseif ($Target -eq 'Clippy') {
        $rustTarget = Get-RustTarget -Platform $Platform
        
        if (-not (Test-RustTargetInstalled -RustTarget $rustTarget)) {
            Add-BuildResult -Configuration $Configuration -Platform $Platform -Target 'Clippy' -Status 'Failed' -ExitCode 1 -Message "Target $rustTarget not installed"
            throw "Rust target $rustTarget is not installed. Run: rustup target add $rustTarget"
        }

        $cargoArgs = @('clippy', '--target', $rustTarget)
        if ($Configuration -eq 'Release') {
            $cargoArgs += '--release'
        }
        $cargoArgs += '--', '-D', 'warnings'

        Write-Host "cargo $($cargoArgs -join ' ')" -ForegroundColor Cyan
        
        $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
        & cargo @cargoArgs
        $stopwatch.Stop()

        if ($LASTEXITCODE -ne 0) {
            Add-BuildResult -Configuration $Configuration -Platform $Platform -Target 'Clippy' -Status 'Failed' -ExitCode $LASTEXITCODE -Duration $stopwatch.Elapsed
            $scriptExitCode = $LASTEXITCODE
        } else {
            Add-BuildResult -Configuration $Configuration -Platform $Platform -Target 'Clippy' -Status 'Succeeded' -Duration $stopwatch.Elapsed
        }
    }
    elseif ($Target -eq 'Clean') {
        $rustTarget = Get-RustTarget -Platform $Platform
        
        $cargoArgs = @('clean', '--target', $rustTarget)
        if ($Configuration -eq 'Release') {
            $cargoArgs += '--release'
        }

        Write-Host "cargo $($cargoArgs -join ' ')" -ForegroundColor Cyan
        cargo @cargoArgs

        if ($LASTEXITCODE -ne 0) {
            Add-BuildResult -Configuration $Configuration -Platform $Platform -Target 'Clean' -Status 'Failed' -ExitCode $LASTEXITCODE
            $scriptExitCode = $LASTEXITCODE
        } else {
            Add-BuildResult -Configuration $Configuration -Platform $Platform -Target 'Clean' -Status 'Succeeded'
        }
    }
    elseif ($Target -eq 'Rebuild') {
        $rustTarget = Get-RustTarget -Platform $Platform
        
        if (-not (Test-RustTargetInstalled -RustTarget $rustTarget)) {
            Add-BuildResult -Configuration $Configuration -Platform $Platform -Target 'Rebuild' -Status 'Failed' -ExitCode 1 -Message "Target $rustTarget not installed"
            throw "Rust target $rustTarget is not installed. Run: rustup target add $rustTarget"
        }

        # Clean first
        $cargoArgs = @('clean', '--target', $rustTarget)
        if ($Configuration -eq 'Release') {
            $cargoArgs += '--release'
        }

        Write-Host "cargo $($cargoArgs -join ' ')" -ForegroundColor Cyan
        cargo @cargoArgs

        # Then build
        $result = Invoke-CargoBuild -Configuration $Configuration -Platform $Platform

        if ($result.ExitCode -ne 0) {
            Add-BuildResult -Configuration $Configuration -Platform $Platform -Target 'Rebuild' -Status 'Failed' -ExitCode $result.ExitCode -Duration $result.Duration
            $scriptExitCode = $result.ExitCode
        } else {
            Add-BuildResult -Configuration $Configuration -Platform $Platform -Target 'Rebuild' -Status 'Succeeded' -Duration $result.Duration
        }
    }
    else {
        # Build
        $rustTarget = Get-RustTarget -Platform $Platform
        
        if (-not (Test-RustTargetInstalled -RustTarget $rustTarget)) {
            Add-BuildResult -Configuration $Configuration -Platform $Platform -Target 'Build' -Status 'Failed' -ExitCode 1 -Message "Target $rustTarget not installed"
            throw "Rust target $rustTarget is not installed. Run: rustup target add $rustTarget"
        }

        $result = Invoke-CargoBuild -Configuration $Configuration -Platform $Platform

        if ($result.ExitCode -ne 0) {
            Add-BuildResult -Configuration $Configuration -Platform $Platform -Target 'Build' -Status 'Failed' -ExitCode $result.ExitCode -Duration $result.Duration
            $scriptExitCode = $result.ExitCode
        } else {
            Add-BuildResult -Configuration $Configuration -Platform $Platform -Target 'Build' -Status 'Succeeded' -Duration $result.Duration
        }
    }
}
catch {
    if ($scriptExitCode -eq 0) {
        $scriptExitCode = 1
    }

    Write-Host $_ -ForegroundColor Red
}
finally {
    Pop-Location
    Write-BuildSummary
}

exit $scriptExitCode
