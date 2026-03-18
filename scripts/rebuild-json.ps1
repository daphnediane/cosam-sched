#!/usr/bin/env pwsh

# PowerShell equivalent of rebuild-json.sh
# Helper script to rebuild all JSON files for testing
# Copyright (c) 2026 Daphne Pfister
# SPDX-License-Identifier: BSD-2-Clause

param(
    [switch]$Verbose
)

$ErrorActionPreference = "Stop"

# Get script directory and project root
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RootDir = Split-Path -Parent $ScriptDir
$InputDir = Join-Path $RootDir "input"

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

# Main execution
try {
    Write-Status "Rebuilding JSON files for testing..."
    Write-Host "Script directory: $ScriptDir"
    Write-Host "Input directory: $InputDir"
    Write-Host "Project root: $RootDir"
    Write-Host ""
    
    $builtFiles = @()
    $currentYear = (Get-Date).Year
    
    for ($year = 2016; $year -le $currentYear; $year++) {
        $srcFile = Join-Path $InputDir "${year} Schedule.xlsx"
        
        if (-not (Test-Path $srcFile)) {
            Write-Warning "Skipping ${year} - file not found: $srcFile"
            continue
        }
        
        # Build files for this year
        Write-Status "Building ${year} files..."
        
        $outputDir = Join-Path $RootDir "widget"
        $outputFile = Join-Path $outputDir "${year}.json"
        
        Write-Host "  Building ${year}.json with Rust converter CLI..."
        
        if ($Verbose) {
            Write-Host "    Source: $srcFile"
            Write-Host "    Output: $outputFile"
        }
        
        try {
            # Change to project root directory for cargo command
            Push-Location $RootDir
            
            $cargoArgs = @(
                "run", 
                "-p", "cosam-convert", 
                "--",
                "--input", $srcFile,
                "--output", $outputFile,
                "--title", "Cosplay America ${year} Schedule"
            )
            
            if ($Verbose) {
                $cargoArgs += "--verbose"
            }
            
            & cargo @cargoArgs
            
            if ($LASTEXITCODE -eq 0) {
                $builtFiles += "${year}.json (Rust converter CLI)"
                Write-Host "    ✓ Successfully built ${year}.json"
            }
            else {
                throw "Cargo command failed with exit code $LASTEXITCODE"
            }
            
        }
        catch {
            $builtFiles += "${year}.json (Rust converter CLI) - FAILED: $($_.Exception.Message)"
            Write-Warning "Failed to build ${year}.json: $($_.Exception.Message)"
        }
        finally {
            Pop-Location
        }
        
        Write-Host ""
    }
    
    Write-Status "JSON rebuild process completed!"
    Write-Host ""
    Write-Host "Files processed:"
    foreach ($file in $builtFiles) {
        Write-Host "  - widget/$file"
    }
    
    $successCount = ($builtFiles | Where-Object { $_ -notlike "*FAILED*" }).Count
    $totalCount = $builtFiles.Count
    
    Write-Host ""
    Write-Status "Summary: $successCount/$totalCount files built successfully"
    
    if ($successCount -lt $totalCount) {
        Write-Warning "Some files failed to build. Check the output above for details."
        exit 1
    }
    
}
catch {
    Write-Error "Script failed: $_"
    exit 1
}
