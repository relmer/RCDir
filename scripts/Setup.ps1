<#
.SYNOPSIS
    Sets up the Rust development environment for building RCDir.

.DESCRIPTION
    This script checks for and installs all required infrastructure:
      1. PowerShell 7 (pwsh) - validated, not installed automatically
      2. Rust toolchain via rustup - offers to install if missing
      3. Required compilation targets (x86_64 and aarch64 Windows MSVC)
      4. Required components (clippy, rustfmt)

    The rust-toolchain.toml file in the repo root also drives automatic
    toolchain/target/component installation whenever cargo or rustup runs,
    but this script provides a friendlier first-time experience with
    clear diagnostics.

.EXAMPLE
    .\scripts\Setup.ps1
    Checks and installs all prerequisites.

.EXAMPLE
    .\scripts\Setup.ps1 -Verbose
    Shows detailed output for each step.

.NOTES
    Run this once after cloning the repo.
    Requires an internet connection to download Rust components.
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

$requiredTargets = @(
    'x86_64-pc-windows-msvc',
    'aarch64-pc-windows-msvc'
)

$requiredComponents = @(
    'clippy',
    'rustfmt'
)

# Minimum Rust version required (2024 edition support)
$minimumRustVersion = [version]'1.85.0'


# ==============================================================================
#  Helpers
# ==============================================================================

function Write-Step {
    param([string]$Message)
    Write-Host ''
    Write-Host "--- $Message ---" -ForegroundColor Cyan
}

function Write-Ok {
    param([string]$Message)
    Write-Host "  OK  $Message" -ForegroundColor Green
}

function Write-Warn {
    param([string]$Message)
    Write-Host "  !!  $Message" -ForegroundColor Yellow
}

function Write-Fail {
    param([string]$Message)
    Write-Host "  XX  $Message" -ForegroundColor Red
}

function Write-Info {
    param([string]$Message)
    Write-Host "      $Message" -ForegroundColor DarkGray
}


# ==============================================================================
#  Step 1: Verify PowerShell 7
# ==============================================================================

Write-Step 'Checking PowerShell version'

if ($PSVersionTable.PSVersion.Major -ge 7) {
    Write-Ok "PowerShell $($PSVersionTable.PSVersion) detected."
}
else {
    Write-Warn "PowerShell $($PSVersionTable.PSVersion) detected. PowerShell 7+ (pwsh) is required for the build scripts."
    Write-Info 'Install from: https://learn.microsoft.com/en-us/powershell/scripting/install/installing-powershell-on-windows'
    Write-Info 'Or run: winget install Microsoft.PowerShell'
}


# ==============================================================================
#  Step 2: Check / install rustup
# ==============================================================================

Write-Step 'Checking for Rust toolchain (rustup)'

$rustupCmd = Get-Command rustup -ErrorAction SilentlyContinue

if ($rustupCmd) {
    $rustupVersion = (rustup --version 2>&1) -join ' '
    Write-Ok "rustup found: $rustupVersion"
}
else {
    Write-Warn 'rustup is not installed (or not on PATH).'
    Write-Info 'The Rust toolchain is required to build this project.'
    Write-Info 'Installer: https://rustup.rs/'
    Write-Host ''

    $response = Read-Host '  Install Rust now? (Y/n)'

    if ($response -eq '' -or $response -match '^[Yy]') {
        Write-Info 'Downloading rustup-init.exe ...'

        $installerUrl  = 'https://win.rustup.rs/x86_64'
        $installerPath = Join-Path $env:TEMP 'rustup-init.exe'

        try {
            [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
            Invoke-WebRequest -Uri $installerUrl -OutFile $installerPath -UseBasicParsing
        }
        catch {
            Write-Fail "Failed to download rustup-init.exe: $_"
            Write-Info 'Please install manually from https://rustup.rs/'
            exit 1
        }

        Write-Info 'Running rustup-init.exe (follow the prompts) ...'
        Write-Host ''

        & $installerPath

        if ($LASTEXITCODE -ne 0) {
            Write-Fail "rustup-init exited with code $LASTEXITCODE."
            exit 1
        }

        # Refresh PATH so we can find cargo/rustup without restarting the shell
        $cargobin = Join-Path $env:USERPROFILE '.cargo\bin'
        if ($env:PATH -notlike "*$cargobin*") {
            $env:PATH = "$cargobin;$env:PATH"
        }

        $rustupCmd = Get-Command rustup -ErrorAction SilentlyContinue
        if (-not $rustupCmd) {
            Write-Fail 'rustup still not found on PATH after installation.'
            Write-Info "You may need to restart your terminal and re-run this script."
            exit 1
        }

        Write-Ok 'rustup installed successfully.'
    }
    else {
        Write-Fail 'Rust toolchain is required. Please install from https://rustup.rs/ and re-run this script.'
        exit 1
    }
}


# ==============================================================================
#  Step 3: Verify Rust version
# ==============================================================================

Write-Step 'Checking Rust compiler version'

$rustcVersionOutput = (rustc --version 2>&1) -join ''

if ($rustcVersionOutput -match '(\d+\.\d+\.\d+)') {
    $installedVersion = [version]$Matches[1]
    Write-Info "rustc $installedVersion"

    if ($installedVersion -ge $minimumRustVersion) {
        Write-Ok "Rust $installedVersion meets the minimum requirement ($minimumRustVersion)."
    }
    else {
        Write-Warn "Rust $installedVersion is below the minimum $minimumRustVersion (required for edition 2024)."
        Write-Info 'Updating toolchain ...'
        rustup update stable
        if ($LASTEXITCODE -ne 0) {
            Write-Fail 'Failed to update Rust toolchain.'
            exit 1
        }
        Write-Ok 'Rust toolchain updated.'
    }
}
else {
    Write-Warn "Could not parse Rust version from: $rustcVersionOutput"
    Write-Info 'Attempting toolchain update ...'
    rustup update stable
}


# ==============================================================================
#  Step 4: Install required targets
# ==============================================================================

Write-Step 'Checking compilation targets'

$installedTargets = (rustup target list --installed 2>&1) | ForEach-Object { $_.Trim() }

foreach ($target in $requiredTargets) {
    if ($installedTargets -contains $target) {
        Write-Ok "Target '$target' is installed."
    }
    else {
        Write-Info "Installing target '$target' ..."
        rustup target add $target
        if ($LASTEXITCODE -ne 0) {
            Write-Fail "Failed to install target '$target'."
            exit 1
        }
        Write-Ok "Target '$target' installed."
    }
}


# ==============================================================================
#  Step 5: Install required components
# ==============================================================================

Write-Step 'Checking toolchain components'

$installedComponents = (rustup component list --installed 2>&1) | ForEach-Object { $_.Trim() }

foreach ($component in $requiredComponents) {
    # rustup component list shows e.g. "clippy-x86_64-pc-windows-msvc" so we match the prefix
    $found = $installedComponents | Where-Object { $_ -like "$component*" }
    if ($found) {
        Write-Ok "Component '$component' is installed."
    }
    else {
        Write-Info "Installing component '$component' ..."
        rustup component add $component
        if ($LASTEXITCODE -ne 0) {
            Write-Fail "Failed to install component '$component'."
            exit 1
        }
        Write-Ok "Component '$component' installed."
    }
}


# ==============================================================================
#  Step 6: Verify cargo works
# ==============================================================================

Write-Step 'Verifying cargo'

$cargoCmd = Get-Command cargo -ErrorAction SilentlyContinue

if ($cargoCmd) {
    $cargoVersion = (cargo --version 2>&1) -join ''
    Write-Ok "cargo found: $cargoVersion"
}
else {
    Write-Fail 'cargo not found on PATH. Something went wrong with the Rust installation.'
    exit 1
}


# ==============================================================================
#  Done
# ==============================================================================

Write-Host ''
Write-Host '========================================' -ForegroundColor Green
Write-Host '  Setup complete! Ready to build RCDir.' -ForegroundColor Green
Write-Host '========================================' -ForegroundColor Green
Write-Host ''
Write-Host 'Next steps:' -ForegroundColor White
Write-Host '  Build (debug):   .\scripts\Build.ps1' -ForegroundColor DarkGray
Write-Host '  Build (release): .\scripts\Build.ps1 -Configuration Release' -ForegroundColor DarkGray
Write-Host '  Run tests:       .\scripts\RunTests.ps1' -ForegroundColor DarkGray
Write-Host '  Run clippy:      .\scripts\Build.ps1 -Target Clippy' -ForegroundColor DarkGray
Write-Host ''
