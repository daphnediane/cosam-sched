#!/usr/bin/env pwsh

# PowerShell equivalent of build-rust-targets.sh
# Builds Rust CLI and GUI for multiple targets:
# - Native target (dynamically detected)
# - Windows 11 x86_64 (x86_64-pc-windows-msvc) 
# - Windows 11 ARM (aarch64-pc-windows-msvc) - added if not native

param(
    [switch]$SkipCheck,
    [switch]$Verbose
)

$ErrorActionPreference = "Stop"

function Write-Status {
    param([string]$Message)
    $timestamp = Get-Date -Format "HH:mm:ss"
    Write-Host "[$timestamp] $Message" -ForegroundColor Green
}

function Write-Warning {
    param([string]$Message)
    $timestamp = Get-Date -Format "HH:mm:ss"
    Write-Host "[$timestamp] WARNING: $Message" -ForegroundColor Yellow
}

# Get script directory and project root
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RootDir = Split-Path -Parent $ScriptDir

# Determine native target
$NativeTarget = rustc -vV | Select-String "host:" | ForEach-Object { $_.ToString().Split(':')[1].Trim() }
Write-Status "Detected native target: $NativeTarget"

# Target configurations
$Targets = @(
    @{
        Name     = "Native ($NativeTarget)"
        Triple   = $NativeTarget
        IsNative = $true
    },
    @{
        Name     = "Windows 11 x86_64"
        Triple   = "x86_64-pc-windows-msvc"
        IsNative = $false
    },
    @{
        Name     = "Windows 11 ARM"
        Triple   = "aarch64-pc-windows-msvc"
        IsNative = $false
    }
)

# Remove duplicate targets (if native target is also a Windows target)
$Targets = $Targets | Where-Object { $_.Triple -ne $NativeTarget -or $_.IsNative }

function Test-RustTarget {
    param([string]$Target)
    
    Write-Status "Checking Rust target $Target..."
    $installedTargets = rustup target list --installed
    $targetFound = $installedTargets | Select-String $Target -Quiet
    if (-not $targetFound) {
        Write-Warning "Missing Rust target $Target"
        Write-Host "Install with: rustup target add $Target"
        return $false
    }
    return $true
}

function Build-Target {
    param(
        [string]$Target,
        [bool]$IsNative,
        [string]$Name
    )
    
    Write-Status "Building for $Name ($Target)..."
    
    $cargoArgs = @(
        "--manifest-path", "$RootDir\Cargo.toml",
        "-p", "cosam-convert",
        "-p", "cosam-editor",
        "-p", "cosam-modify"
    )
    
    if (-not $IsNative) {
        $cargoArgs += @("--target", $Target)
    }
    
    if ($Verbose) {
        $cargoArgs += "--verbose"
    }
    
    & cargo build @cargoArgs
    
    if ($LASTEXITCODE -ne 0) {
        throw "Build failed for $Target"
    }
    
    Write-Status "Build completed for $Name"
}

# Main execution
try {
    Write-Status "Starting Rust multi-target build..."
    Write-Host "Project root: $RootDir"
    Write-Host ""
    
    foreach ($targetConfig in $Targets) {
        $target = $targetConfig.Triple
        $isNative = $targetConfig.IsNative
        $name = $targetConfig.Name
        
        Write-Host "Processing target: $name"
        
        if (-not $SkipCheck -and -not $isNative) {
            if (-not (Test-RustTarget $target)) {
                Write-Warning "Skipping $name due to missing prerequisites"
                continue
            }
        }
        
        Build-Target -Target $target -IsNative $isNative -Name $name
        Write-Host ""
    }
    
    Write-Status "All builds completed successfully!"
    
}
catch {
    Write-Error "Build failed: $_"
    exit 1
}
