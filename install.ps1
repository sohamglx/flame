# ==============================================================================
# Flame & Blaze Toolchain Installer (PowerShell for Windows)
# ==============================================================================

$ErrorActionPreference = "Stop"

Write-Host "  _    _      _ _             _____                 _                         " -ForegroundColor Cyan
Write-Host " | |  | |    | | |           |  __ \               | |                        " -ForegroundColor Cyan
Write-Host " | |__| | ___| | | ___       | |  | | _____   _____| | ___  _ __   ___ _ __   " -ForegroundColor Cyan
Write-Host " |  __  |/ _ \ | |/ _ \      | |  | |/ _ \ \ / / _ \ |/ _ \| '_ \ / _ \ '__|  " -ForegroundColor Cyan
Write-Host " | |  | |  __/ | | (_) |     | |__| |  __/\ V /  __/ | (_) | |_) |  __/ |     " -ForegroundColor Cyan
Write-Host " |_|  |_|\___|_|_|\___/      |_____/ \___| \_/ \___|_|\___/| .__/ \___|_|     " -ForegroundColor Cyan
Write-Host "                                                           | |                " -ForegroundColor Cyan
Write-Host "                                                           |_|                " -ForegroundColor Cyan
Write-Host ""
Write-Host "Installing Flame Language & Blaze Toolchain..." -ForegroundColor Yellow
Write-Host ""

# 1. Verify Cargo is available
$cargoCmd = Get-Command cargo -ErrorAction SilentlyContinue
if (-not $cargoCmd) {
    Write-Host "Error: Cargo is not installed or not in PATH." -ForegroundColor Red
    Write-Host "Please install Rust and Cargo from https://rustup.rs/ before continuing."
    exit 1
}

$cargoVer = & cargo --version
Write-Host "Rust/Cargo detected: $cargoVer" -ForegroundColor Green

# 2. Build and install via Cargo
$scriptDir = ""
if ($MyInvocation.MyCommand.Path) {
    $scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
}
if (-not $scriptDir) {
    $scriptDir = (Get-Location).Path
}

Write-Host ""
Write-Host "[1/3] Building and installing Flame binaries (fmp & flamelang)..." -ForegroundColor Cyan

$localCargo = Join-Path $scriptDir "Cargo.toml"
if (Test-Path $localCargo) {
    Write-Host "Installing from local repository at $scriptDir..." -ForegroundColor Green
    & cargo install --path $scriptDir --force
} else {
    Write-Host "Installing flamelang from Cargo registry (crates.io)..." -ForegroundColor Green
    $installOk = $false
    try {
        & cargo install --force flamelang
        if ($LASTEXITCODE -eq 0) { $installOk = $true }
    } catch {
        $installOk = $false
    }
    if (-not $installOk) {
        Write-Host "Registry install not yet available or failed; installing latest from Git repository..." -ForegroundColor Yellow
        & cargo install --git https://github.com/shoya-129/flame.git --force
    }
}

# 3. Locate Cargo bin directory
$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
if ($env:CARGO_HOME) {
    $cargoBin = if ($env:CARGO_HOME.EndsWith("bin")) { $env:CARGO_HOME } else { Join-Path $env:CARGO_HOME "bin" }
}

$cargoCmdObj = Get-Command cargo -ErrorAction SilentlyContinue
if ($cargoCmdObj -and $cargoCmdObj.Source) {
    $cargoDir = Split-Path -Parent $cargoCmdObj.Source
    if ((Test-Path (Join-Path $cargoDir "fmp.exe")) -or (Test-Path (Join-Path $cargoDir "flamelang.exe"))) {
        $cargoBin = $cargoDir
    }
}

if (-not (Test-Path $cargoBin)) {
    New-Item -ItemType Directory -Path $cargoBin -Force | Out-Null
}

function Safe-CopyBinary($src, $dst) {
    if (-not (Test-Path $src) -or ($src -eq $dst)) { return }
    try {
        Copy-Item $src $dst -Force
    } catch {
        $tempPath = "$dst.deleteme.$PID"
        Move-Item $dst $tempPath -Force -ErrorAction SilentlyContinue
        Copy-Item $src $dst -Force
    }
}

# Ensure ONLY fmp.exe exists, safely made from flamelang.exe
$fmpExe = Join-Path $cargoBin "fmp.exe"
$flameExe = Join-Path $cargoBin "flame.exe"
$flamelangExe = Join-Path $cargoBin "flamelang.exe"

# If flamelang.exe exists (newly compiled from cargo), safely overwrite fmp.exe
if (Test-Path $flamelangExe) {
    Safe-CopyBinary $flamelangExe $fmpExe
    Remove-Item $flamelangExe -Force -ErrorAction SilentlyContinue
}
elseif (Test-Path $flameExe) {
    Safe-CopyBinary $flameExe $fmpExe
    Remove-Item $flameExe -Force -ErrorAction SilentlyContinue
}

# Also check other system path locations if cargo placed flamelang elsewhere
$flameCmdObj = Get-Command flamelang -ErrorAction SilentlyContinue
if ($flameCmdObj -and $flameCmdObj.Source -and (Test-Path $flameCmdObj.Source) -and ($flameCmdObj.Source -ne $flamelangExe)) {
    if (-not (Test-Path $fmpExe)) {
        Safe-CopyBinary $flameCmdObj.Source $fmpExe
    }
    Remove-Item $flameCmdObj.Source -Force -ErrorAction SilentlyContinue
}

# Remove any lingering flamelang and flame binaries across cargo bin
Remove-Item $flamelangExe -Force -ErrorAction SilentlyContinue
Remove-Item $flameExe -Force -ErrorAction SilentlyContinue
Remove-Item (Join-Path $cargoBin "flamelang.cmd") -Force -ErrorAction SilentlyContinue
Remove-Item (Join-Path $cargoBin "flamelang.bat") -Force -ErrorAction SilentlyContinue
Remove-Item (Join-Path $cargoBin "flame.cmd") -Force -ErrorAction SilentlyContinue
Remove-Item (Join-Path $cargoBin "flame.bat") -Force -ErrorAction SilentlyContinue

# Clean up any leftover *.deleteme.* files
Get-ChildItem -Path $cargoBin -Filter "*.deleteme.*" -ErrorAction SilentlyContinue | Remove-Item -Force -ErrorAction SilentlyContinue

# Create command shims ONLY for fmp
$fmpCmd = Join-Path $cargoBin "fmp.cmd"
$fmpBat = Join-Path $cargoBin "fmp.bat"
Set-Content -Path $fmpCmd -Value '@"%~dp0fmp.exe" %*' -Encoding ASCII
Set-Content -Path $fmpBat -Value '@"%~dp0fmp.exe" %*' -Encoding ASCII

# Create POSIX shell wrapper for WSL and Git Bash so 'fmp' works directly in Linux/WSL terminals
$fmpSh = Join-Path $cargoBin "fmp"
Set-Content -Path $fmpSh -Value "#!/bin/sh`nDIR=`"`$(cd `"`$(dirname `"`$0`")`" && pwd)`"`nexec `"`$DIR/fmp.exe`" `"`$@`"" -Encoding ASCII

# 4. Install Blaze standard library definitions
Write-Host ""
Write-Host "[2/3] Setting up Blaze standard library definition directory..." -ForegroundColor Cyan

$sourceBlaze = ""
if (Test-Path (Join-Path $scriptDir "Blaze\std")) {
    $sourceBlaze = Join-Path $scriptDir "Blaze\std"
} elseif (Test-Path (Join-Path $scriptDir "std")) {
    $sourceBlaze = Join-Path $scriptDir "std"
}

$cleanupTemp = $null
if (-not $sourceBlaze) {
    Write-Host "Fetching Blaze standard library definitions from repository..." -ForegroundColor Yellow
    $tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("flame_std_" + [System.Guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $tempDir -Force | Out-Null
    $cleanupTemp = $tempDir
    $tempZip = Join-Path $tempDir "repo.zip"
    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        Invoke-WebRequest -Uri "https://github.com/shoya-129/flame/archive/refs/heads/main.zip" -OutFile $tempZip -UseBasicParsing
        Expand-Archive -Path $tempZip -DestinationPath $tempDir -Force
        $extractedBlaze = Join-Path $tempDir "flame-main\Blaze\std"
        if (Test-Path $extractedBlaze) {
            $sourceBlaze = $extractedBlaze
        } else {
            $extractedStd = Join-Path $tempDir "flame-main\std"
            if (Test-Path $extractedStd) {
                $sourceBlaze = $extractedStd
            }
        }
    } catch {
        Write-Host "Warning: Could not download definitions archive: $_" -ForegroundColor Yellow
    }
}

if ((-not $sourceBlaze) -or (-not (Test-Path $sourceBlaze))) {
    Write-Host "Warning: Standard library definitions could not be located." -ForegroundColor Yellow
    Write-Host "Definitions can be initialized later via 'fmp update'." -ForegroundColor Yellow
} else {
    $targetBlazeDirs = @(
        (Join-Path $env:LOCALAPPDATA "Blaze\std"),
        (Join-Path $env:USERPROFILE ".blaze\std")
    )

    if ($env:ProgramFiles) {
        $progBlaze = Join-Path $env:ProgramFiles "Blaze\std"
        $targetBlazeDirs = @($progBlaze) + $targetBlazeDirs
    }

    $primaryBlazeDir = ""

    foreach ($dest in $targetBlazeDirs) {
        try {
            if (-not (Test-Path $dest)) {
                New-Item -ItemType Directory -Path $dest -Force | Out-Null
            }
            Copy-Item -Path "$sourceBlaze\*" -Destination $dest -Recurse -Force | Out-Null
            Write-Host "  Installed definitions to: $dest" -ForegroundColor Green
            if (-not $primaryBlazeDir) {
                $primaryBlazeDir = (Split-Path -Parent $dest)
            }
        }
        catch {
            # continue to next candidate directory if permission denied
        }
    }
}

if ($cleanupTemp -and (Test-Path $cleanupTemp)) {
    Remove-Item -Path $cleanupTemp -Recurse -Force -ErrorAction SilentlyContinue
}

# 5. Environment & Permanent PATH persistence
Write-Host ""
Write-Host "[3/3] Setting up environment and permanently persisting PATH..." -ForegroundColor Cyan

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (-not ($userPath -split ";" -contains $cargoBin)) {
    $newPath = ($userPath.TrimEnd(";") + ";" + $cargoBin).TrimStart(";")
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Write-Host "  Permanently added $cargoBin to User PATH." -ForegroundColor Green
}

# Update current PowerShell process environment PATH immediately so fmp is available right away
if (-not ($env:PATH -split ";" -contains $cargoBin)) {
    $env:PATH = "$cargoBin;$env:PATH"
}

if ($primaryBlazeDir) {
    [Environment]::SetEnvironmentVariable("BLAZE_HOME", $primaryBlazeDir, "User")
    $env:BLAZE_HOME = $primaryBlazeDir
    Write-Host "  Set BLAZE_HOME to $primaryBlazeDir in User Environment." -ForegroundColor Green
}

Write-Host ""
Write-Host "[OK] Flame and Blaze toolchain successfully installed!" -ForegroundColor Green
Write-Host ""
Write-Host "  Primary Command:  fmp" -ForegroundColor Cyan
Write-Host "  Binary Location:  $fmpExe"
Write-Host "  Blaze Definitions:$primaryBlazeDir\std"

$fmpCmdCheck = Get-Command fmp -ErrorAction SilentlyContinue
if (-not $fmpCmdCheck) {
    Write-Host ""
    Write-Host "Notice: If 'fmp' is not recognized in your current shell, run:" -ForegroundColor Yellow
    Write-Host "  `$env:PATH = `"$cargoBin;`$env:PATH`"" -ForegroundColor Green
    Write-Host "or open a new PowerShell window."
}

Write-Host ""
Write-Host "Quick Start:" -ForegroundColor Yellow
Write-Host "  Check version:    fmp --version"
Write-Host "  CLI Help menu:    fmp help"
Write-Host "  Update release:   fmp update"
Write-Host "  Uninstall:        fmp uninstall"
Write-Host ""
