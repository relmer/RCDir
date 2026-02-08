<#
.SYNOPSIS
    Deploys release builds to a destination folder.

.DESCRIPTION
    Copies release executables to a deployment folder specified by the
    RCDIR_DEPLOY_PATH environment variable. The ARM64 binary is renamed
    to RCDir_ARM64.exe to distinguish it from the x64 version.

.PARAMETER Force
    Overwrite existing files without prompting.

.EXAMPLE
    # First, set the environment variable (one-time setup):
    [Environment]::SetEnvironmentVariable('RCDIR_DEPLOY_PATH', 'C:\Path\To\Utils', 'User')
    
    # Then run the script:
    .\scripts\Deploy.ps1

.NOTES
    The RCDIR_DEPLOY_PATH environment variable must be set to the destination folder.
    This keeps machine-specific paths out of the repository.
#>

[CmdletBinding()]
param(
    [switch]$Force
)

$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path $PSScriptRoot -Parent

# Get deployment path from environment variable
$deployPath = [Environment]::GetEnvironmentVariable('RCDIR_DEPLOY_PATH', 'User')
if (-not $deployPath) {
    $deployPath = [Environment]::GetEnvironmentVariable('RCDIR_DEPLOY_PATH', 'Process')
}

if (-not $deployPath) {
    Write-Error @"
RCDIR_DEPLOY_PATH environment variable is not set.

To set it (one-time setup), run:
    [Environment]::SetEnvironmentVariable('RCDIR_DEPLOY_PATH', 'C:\Your\Deploy\Path', 'User')

Then restart your terminal and run this script again.
"@
    exit 1
}

if (-not (Test-Path $deployPath)) {
    Write-Error "Deployment path does not exist: $deployPath"
    exit 1
}

# Define source and destination mappings
# Cargo puts binaries in: target/<rust-target>/release/<binary-name>.exe
$deployments = @(
    @{
        Source      = Join-Path $repoRoot 'target\x86_64-pc-windows-msvc\release\rcdir.exe'
        Destination = Join-Path $deployPath 'RCDir.exe'
        Description = 'x64 Release'
    },
    @{
        Source      = Join-Path $repoRoot 'target\aarch64-pc-windows-msvc\release\rcdir.exe'
        Destination = Join-Path $deployPath 'RCDir_ARM64.exe'
        Description = 'ARM64 Release'
    }
)

$copied = 0
$skipped = 0

foreach ($deploy in $deployments) {
    $src = $deploy.Source
    $dst = $deploy.Destination
    $desc = $deploy.Description

    if (-not (Test-Path $src)) {
        Write-Warning "$desc not found: $src (skipping)"
        $skipped++
        continue
    }

    $srcInfo = Get-Item $src
    $copyNeeded = $true

    if ((Test-Path $dst) -and -not $Force) {
        $dstInfo = Get-Item $dst
        if ($srcInfo.LastWriteTime -le $dstInfo.LastWriteTime) {
            Write-Host "$desc is up to date: $(Split-Path $dst -Leaf)" -ForegroundColor DarkGray
            $copyNeeded = $false
            $skipped++
        }
    }

    if ($copyNeeded) {
        Copy-Item -Path $src -Destination $dst -Force
        Write-Host "$desc deployed: $(Split-Path $dst -Leaf)" -ForegroundColor Green
        $copied++
    }
}

Write-Host ''
if ($copied -gt 0) {
    Write-Host "Deployed $copied file(s) to $deployPath" -ForegroundColor Green
}
if ($skipped -gt 0) {
    Write-Host "Skipped $skipped file(s) (up to date or not found)" -ForegroundColor DarkGray
}
