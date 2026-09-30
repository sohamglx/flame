#!/usr/bin/env bash
# ==============================================================================
# Flame & Blaze Toolchain Installer
# Cross-platform installer for Windows (PowerShell/WSL/Git Bash), Linux, and macOS
# ==============================================================================

set -e

BOLD="\033[1m"
GREEN="\033[1;32m"
BLUE="\033[1;34m"
YELLOW="\033[1;33m"
RED="\033[1;31m"
RESET="\033[0m"

echo -e "${BLUE}${BOLD}"
echo "  _    _      _ _             _____                 _                         "
echo " | |  | |    | | |           |  __ \               | |                        "
echo " | |__| | ___| | | ___       | |  | | _____   _____| | ___  _ __   ___ _ __   "
echo " |  __  |/ _ \ | |/ _ \      | |  | |/ _ \ \ / / _ \ |/ _ \| '_ \ / _ \ '__|  "
echo " | |  | |  __/ | | (_) |     | |__| |  __/\ V /  __/ | (_) | |_) |  __/ |     "
echo " |_|  |_|\___|_|_|\___/      |_____/ \___| \_/ \___|_|\___/| .__/ \___|_|     "
echo "                                                           | |                "
echo "                                                           |_|                "
echo -e "${RESET}"
echo -e "${BOLD}Installing Flame Language & Blaze Toolchain...${RESET}\n"

# 1. Detect Operating System and Environment
OS_TYPE="unknown"
IS_WSL=false

if grep -qi "microsoft" /proc/version 2>/dev/null || uname -r | grep -qi "microsoft"; then
    IS_WSL=true
fi

case "$(uname -s)" in
    Linux*)
        if [[ "$IS_WSL" == true ]]; then
            OS_TYPE="wsl"
        elif [[ -d "/data/data/com.termux" || -n "$TERMUX_VERSION" ]]; then
            OS_TYPE="termux"
        else
            OS_TYPE="linux"
        fi
        ;;
    Darwin*)    OS_TYPE="macos" ;;
    CYGWIN*|MINGW*|MSYS*) OS_TYPE="windows" ;;
    *)
        if [[ "$OS" == "Windows_NT" ]]; then
            OS_TYPE="windows"
        else
            OS_TYPE="unix"
        fi
        ;;
esac

# 2. Find Cargo command (support native cargo and Windows cargo.exe)
CARGO_CMD="cargo"
TARGET_IS_WINDOWS=false

if command -v cargo &> /dev/null; then
    CARGO_CMD="cargo"
    if [[ "$OS_TYPE" == "windows" ]]; then
        TARGET_IS_WINDOWS=true
    fi
elif command -v cargo.exe &> /dev/null; then
    CARGO_CMD="cargo.exe"
    TARGET_IS_WINDOWS=true
else
    echo -e "${RED}Error: Cargo is not installed or not in PATH.${RESET}"
    echo "Please install Rust and Cargo from https://rustup.rs/ before continuing."
    exit 1
fi

if [[ "$OS_TYPE" == "wsl" && "$CARGO_CMD" == "cargo.exe" ]]; then
    TARGET_IS_WINDOWS=true
    echo -e "Detected environment: ${GREEN}Windows (invoked via WSL bash)${RESET}"
else
    echo -e "Detected platform: ${GREEN}${OS_TYPE}${RESET}"
fi

CARGO_VERSION=$("$CARGO_CMD" --version)
echo -e "Rust/Cargo detected: ${GREEN}${CARGO_VERSION}${RESET}"

# Helper function to convert Windows paths to POSIX paths
to_posix_path() {
    local p="$1"
    if [[ -z "$p" ]]; then echo ""; return; fi
    if command -v wslpath &> /dev/null; then
        wslpath -u "$p" 2>/dev/null || echo "$p"
    elif command -v cygpath &> /dev/null; then
        cygpath -u "$p" 2>/dev/null || echo "$p"
    else
        echo "$p"
    fi
}

# 3. Locate Cargo bin directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]:-$0}" )" 2>/dev/null && pwd )"
if [[ -z "$SCRIPT_DIR" || ! -d "$SCRIPT_DIR" ]]; then
    SCRIPT_DIR="$(pwd)"
fi

REAL_USER="$USER"
REAL_HOME="$HOME"
if [[ $EUID -eq 0 && -n "$SUDO_USER" ]]; then
    REAL_USER="$SUDO_USER"
    REAL_HOME="$(getent passwd "$SUDO_USER" 2>/dev/null | cut -d: -f6 || echo "/home/$SUDO_USER")"
fi

CARGO_BIN=""

if [[ "$TARGET_IS_WINDOWS" == true ]]; then
    # Query USERPROFILE from Windows environment if needed
    WIN_USERPROFILE="$USERPROFILE"
    if [[ -z "$WIN_USERPROFILE" ]] && command -v powershell.exe &> /dev/null; then
        WIN_USERPROFILE="$(powershell.exe -NoProfile -Command '$env:USERPROFILE' 2>/dev/null | tr -d '\r')"
    fi
    if [[ -z "$WIN_USERPROFILE" ]] && command -v cmd.exe &> /dev/null; then
        WIN_USERPROFILE="$(cmd.exe /c "echo %USERPROFILE%" 2>/dev/null | tr -d '\r')"
    fi

    if [[ -n "$WIN_USERPROFILE" ]]; then
        CARGO_BIN="$(to_posix_path "$WIN_USERPROFILE/.cargo/bin")"
    else
        CARGO_BIN="$HOME/.cargo/bin"
    fi
else
    if [[ -n "$CARGO_HOME" ]]; then
        if [[ "$CARGO_HOME" == */bin ]]; then
            CARGO_BIN="$CARGO_HOME"
        else
            CARGO_BIN="$CARGO_HOME/bin"
        fi
    elif command -v cargo &>/dev/null; then
        CARGO_DIR="$(dirname "$(command -v cargo 2>/dev/null)")"
        if [[ -d "$CARGO_DIR" && -w "$CARGO_DIR" ]]; then
            CARGO_BIN="$CARGO_DIR"
        else
            CARGO_BIN="$HOME/.cargo/bin"
        fi
    else
        CARGO_BIN="$HOME/.cargo/bin"
    fi
fi

mkdir -p "$CARGO_BIN" 2>/dev/null || true

# Helper: Ensure Linux development libraries are present for native crates (dbus, udev, pkg-config)
ensure_linux_dependencies() {
    if [[ "$TARGET_IS_WINDOWS" == true || "$OS_TYPE" == "macos" || "$OS_TYPE" == "termux" ]]; then
        return 0
    fi

    local missing_pkgs=()
    local apt_pkgs=()
    local dnf_pkgs=()
    local pacman_pkgs=()

    if ! command -v cc &>/dev/null && ! command -v gcc &>/dev/null; then
        missing_pkgs+=("build-essential (gcc / cc)")
        apt_pkgs+=("build-essential")
        dnf_pkgs+=("gcc" "gcc-c++")
        pacman_pkgs+=("base-devel")
    fi

    if ! command -v pkg-config &>/dev/null; then
        missing_pkgs+=("pkg-config")
        apt_pkgs+=("pkg-config")
        dnf_pkgs+=("pkgconf-pkg-config")
        pacman_pkgs+=("pkgconf")
    fi

    if ! pkg-config --exists dbus-1 2>/dev/null; then
        missing_pkgs+=("dbus-1 (libdbus-1-dev)")
        apt_pkgs+=("libdbus-1-dev")
        dnf_pkgs+=("dbus-devel")
        pacman_pkgs+=("dbus")
    fi

    if ! pkg-config --exists libudev 2>/dev/null; then
        missing_pkgs+=("libudev (libudev-dev)")
        apt_pkgs+=("libudev-dev")
        dnf_pkgs+=("systemd-devel")
        pacman_pkgs+=("systemd-libs")
    fi

    local has_libclang=false
    if ldconfig -p 2>/dev/null | grep -q 'libclang\.so'; then
        has_libclang=true
    elif compgen -G "/usr/lib*/**/libclang*.so*" &>/dev/null || compgen -G "/usr/lib*/libclang*.so*" &>/dev/null; then
        has_libclang=true
    elif command -v llvm-config &>/dev/null && [[ -f "$(llvm-config --libdir 2>/dev/null)/libclang.so" ]]; then
        has_libclang=true
    fi

    if [[ "$has_libclang" != true ]]; then
        missing_pkgs+=("libclang (clang / libclang-dev)")
        apt_pkgs+=("libclang-dev" "clang")
        dnf_pkgs+=("clang-devel")
        pacman_pkgs+=("clang")
    fi

    if [[ ${#missing_pkgs[@]} -gt 0 ]]; then
        echo -e "\n${YELLOW}Missing required Linux build dependencies:${RESET} ${missing_pkgs[*]}"
        echo -e "Attempting to install required system packages..."

        local installed=false
        local SUDO_CMD=""
        if [[ $EUID -ne 0 ]]; then
            if command -v sudo &>/dev/null; then
                SUDO_CMD="sudo"
            fi
        fi

        if command -v apt-get &>/dev/null; then
            echo -e "Installing via ${GREEN}apt-get${RESET}: ${apt_pkgs[*]}"
            if $SUDO_CMD apt-get update -y && $SUDO_CMD apt-get install -y "${apt_pkgs[@]}"; then
                installed=true
            fi
        elif command -v dnf &>/dev/null; then
            echo -e "Installing via ${GREEN}dnf${RESET}: ${dnf_pkgs[*]}"
            if $SUDO_CMD dnf install -y "${dnf_pkgs[@]}"; then
                installed=true
            fi
        elif command -v pacman &>/dev/null; then
            echo -e "Installing via ${GREEN}pacman${RESET}: ${pacman_pkgs[*]}"
            if $SUDO_CMD pacman -S --noconfirm "${pacman_pkgs[@]}"; then
                installed=true
            fi
        elif command -v zypper &>/dev/null; then
            if $SUDO_CMD zypper install -y dbus-1-devel systemd-devel pkg-config; then
                installed=true
            fi
        elif command -v apk &>/dev/null; then
            if $SUDO_CMD apk add pkgconf dbus-dev eudev-dev; then
                installed=true
            fi
        fi

        if [[ "$installed" == true ]]; then
            echo -e "${GREEN}✓ System dependencies installed successfully!${RESET}\n"
        else
            echo -e "\n${YELLOW}Notice: Could not automatically install system dependencies.${RESET}"
            echo -e "Please install the missing packages manually:"
            echo -e "  Debian/Ubuntu/WSL: ${GREEN}sudo apt-get update && sudo apt-get install -y build-essential pkg-config libdbus-1-dev libudev-dev libclang-dev clang${RESET}"
            echo -e "  Fedora/RHEL:       ${GREEN}sudo dnf install -y gcc gcc-c++ pkgconf-pkg-config dbus-devel systemd-devel clang-devel${RESET}"
            echo -e "  Arch Linux:        ${GREEN}sudo pacman -S --noconfirm base-devel pkgconf dbus systemd-libs clang${RESET}\n"
        fi
    fi
}

# 4. Build and install fmp & flamelang binaries via Cargo
echo -e "\n${BOLD}[1/3] Building and installing Flame binaries (fmp & flamelang)...${RESET}"

ensure_linux_dependencies

if [[ -f "$SCRIPT_DIR/Cargo.toml" ]]; then
    echo -e "Installing from local repository at ${GREEN}$SCRIPT_DIR${RESET}..."
    if [[ "$CARGO_CMD" == "cargo.exe" && -n "$(command -v wslpath)" ]]; then
        WIN_BUILD_DIR="$(wslpath -w "$SCRIPT_DIR")"
        cargo.exe install --path "$WIN_BUILD_DIR" --force
    else
        "$CARGO_CMD" install --path "$SCRIPT_DIR" --force
    fi
else
    echo -e "Installing flamelang from Cargo registry (crates.io)..."
    if ! "$CARGO_CMD" install --force flamelang; then
        echo -e "${YELLOW}Registry install not yet available or failed; installing latest from Git repository...${RESET}"
        if ! "$CARGO_CMD" install --git https://github.com/shoya-129/flame.git --force; then
            echo -e "\n${RED}Build failed.${RESET}"
            if [[ "$TARGET_IS_WINDOWS" != true && "$OS_TYPE" != "macos" ]]; then
                echo -e "If this build failed due to missing system headers (like dbus-1, libudev, or libclang), run:"
                echo -e "  ${GREEN}sudo apt-get update && sudo apt-get install -y build-essential pkg-config libdbus-1-dev libudev-dev libclang-dev clang${RESET}"
                echo -e "Then re-run the installer."
            fi
            exit 1
        fi
    fi
fi

# Ensure ONLY fmp command executable exists and remove any flamelang or flame
# 1. Search candidate directories for existing/compiled binaries
CANDIDATE_SEARCH_DIRS=(
    "$CARGO_BIN"
    "$HOME/.cargo/bin"
    "$PREFIX/bin"
    "$REAL_HOME/.cargo/bin"
    "/usr/local/bin"
    "$(dirname "$(command -v "$CARGO_CMD" 2>/dev/null || echo "")")"
    "/mnt/c/Users/$REAL_USER/.cargo/bin"
    "/mnt/c/Users/clash/.cargo/bin"
)

# If fmp or flamelang exists as a broken/dangling symlink anywhere, remove it
for D in "$CARGO_BIN" "$HOME/.cargo/bin" "$REAL_HOME/.cargo/bin" "/usr/local/bin" "$HOME/.local/bin" "$REAL_HOME/.local/bin"; do
    if [[ -L "$D/fmp" ]]; then
        rm -f "$D/fmp" 2>/dev/null || true
    fi
    if [[ -L "$D/flamelang" ]]; then
        rm -f "$D/flamelang" 2>/dev/null || true
    fi
done

FOUND_FLAMELANG_EXE=""
FOUND_FMP_EXE=""
FOUND_FLAMELANG_ELF=""
FOUND_FMP_ELF=""

for D in "${CANDIDATE_SEARCH_DIRS[@]}"; do
    if [[ -n "$D" && -d "$D" ]]; then
        if [[ -z "$FOUND_FLAMELANG_EXE" && -f "$D/flamelang.exe" ]]; then
            FOUND_FLAMELANG_EXE="$D/flamelang.exe"
        fi
        if [[ -z "$FOUND_FMP_EXE" && -f "$D/fmp.exe" ]]; then
            FOUND_FMP_EXE="$D/fmp.exe"
        fi
        if [[ -z "$FOUND_FLAMELANG_ELF" && -f "$D/flamelang" && ! -L "$D/flamelang" ]]; then
            FOUND_FLAMELANG_ELF="$D/flamelang"
        fi
        if [[ -z "$FOUND_FMP_ELF" && -f "$D/fmp" && ! -L "$D/fmp" ]]; then
            FOUND_FMP_ELF="$D/fmp"
        fi
    fi
done

# 2. Safely make fmp binary from flamelang.exe / flamelang, then delete flamelang binary files
if [[ -n "$FOUND_FLAMELANG_EXE" ]]; then
    mkdir -p "$CARGO_BIN" 2>/dev/null || true
    cp -f "$FOUND_FLAMELANG_EXE" "$CARGO_BIN/fmp.exe" 2>/dev/null || true
    chmod +x "$CARGO_BIN/fmp.exe" 2>/dev/null || true
    rm -f "$FOUND_FLAMELANG_EXE" 2>/dev/null || true
    rm -f "$CARGO_BIN/flamelang.exe" 2>/dev/null || true
    FOUND_FMP_EXE="$CARGO_BIN/fmp.exe"
fi

if [[ -n "$FOUND_FLAMELANG_ELF" ]]; then
    mkdir -p "$CARGO_BIN" 2>/dev/null || true
    cp -f "$FOUND_FLAMELANG_ELF" "$CARGO_BIN/fmp" 2>/dev/null || true
    chmod +x "$CARGO_BIN/fmp" 2>/dev/null || true
    rm -f "$FOUND_FLAMELANG_ELF" 2>/dev/null || true
    rm -f "$CARGO_BIN/flamelang" 2>/dev/null || true
    FOUND_FMP_ELF="$CARGO_BIN/fmp"
fi

# Clean up all flamelang and flame binaries/shims across candidate directories
for D in "${CANDIDATE_SEARCH_DIRS[@]}"; do
    if [[ -n "$D" && -d "$D" ]]; then
        rm -f "$D/flamelang" "$D/flamelang.exe" "$D/flamelang.cmd" "$D/flamelang.bat" 2>/dev/null || true
        rm -f "$D/flame" "$D/flame.exe" "$D/flame.cmd" "$D/flame.bat" 2>/dev/null || true
        rm -f "$D"/*.deleteme.* 2>/dev/null || true
    fi
done

# Windows shims (fmp.cmd / fmp.bat)
if [[ "$TARGET_IS_WINDOWS" == true || -f "$CARGO_BIN/fmp.exe" ]]; then
    cat << 'EOF' > "$CARGO_BIN/fmp.cmd"
@"%~dp0fmp.exe" %*
EOF
    cat << 'EOF' > "$CARGO_BIN/fmp.bat"
@"%~dp0fmp.exe" %*
EOF
    chmod +x "$CARGO_BIN/fmp.cmd" "$CARGO_BIN/fmp.bat" 2>/dev/null || true
fi

# Linux / WSL / macOS: Ensure executable 'fmp' (without extension) is present and in PATH
if [[ "$TARGET_IS_WINDOWS" != true || "$IS_WSL" == true || "$OS_TYPE" == "wsl" || "$OS_TYPE" == "linux" ]]; then
    # If no native ELF fmp exists, but fmp.exe exists, generate wrapper script
    if [[ ! -f "$CARGO_BIN/fmp" || -L "$CARGO_BIN/fmp" ]]; then
        rm -f "$CARGO_BIN/fmp" 2>/dev/null || true
        cat << 'EOF' > "$CARGO_BIN/fmp"
#!/bin/sh
DIR="$(cd "$(dirname "$0")" && pwd)"
if [ -x "$DIR/fmp.exe" ]; then
    exec "$DIR/fmp.exe" "$@"
elif command -v fmp.exe >/dev/null 2>&1; then
    exec fmp.exe "$@"
elif [ -f "/mnt/c/Users/clash/.cargo/bin/fmp.exe" ]; then
    exec "/mnt/c/Users/clash/.cargo/bin/fmp.exe" "$@"
fi
EOF
        chmod +x "$CARGO_BIN/fmp" 2>/dev/null || true
    fi

    # Distribute fmp to all system and user PATH directories
    for DEST in "$PREFIX/bin" "/usr/local/bin" "$HOME/.local/bin" "$REAL_HOME/.local/bin" "$HOME/.cargo/bin" "$REAL_HOME/.cargo/bin"; do
        if [[ -n "$DEST" && -d "$DEST" && "$DEST" != "$CARGO_BIN" ]]; then
            rm -f "$DEST/fmp" 2>/dev/null || true
            cp -f "$CARGO_BIN/fmp" "$DEST/fmp" 2>/dev/null || true
            chmod +x "$DEST/fmp" 2>/dev/null || true
            if [[ $EUID -eq 0 && -n "$SUDO_USER" ]]; then
                chown "$REAL_USER" "$DEST/fmp" 2>/dev/null || true
            fi
            rm -f "$DEST/flamelang" "$DEST/flamelang.exe" "$DEST/flame" "$DEST/flame.exe" 2>/dev/null || true
        elif [[ ! -d "$DEST" && ( "$DEST" == *".local/bin" || "$DEST" == *".cargo/bin" ) ]]; then
            mkdir -p "$DEST" 2>/dev/null || true
            cp -f "$CARGO_BIN/fmp" "$DEST/fmp" 2>/dev/null || true
            chmod +x "$DEST/fmp" 2>/dev/null || true
            if [[ $EUID -eq 0 && -n "$SUDO_USER" ]]; then
                chown -R "$REAL_USER" "$DEST" 2>/dev/null || true
            fi
        fi
    done

    # Try sudo for /usr/local/bin if not installed yet
    if [[ ! -f "/usr/local/bin/fmp" ]] && command -v sudo &>/dev/null; then
        sudo cp -f "$CARGO_BIN/fmp" "/usr/local/bin/fmp" 2>/dev/null || true
        sudo chmod +x "/usr/local/bin/fmp" 2>/dev/null || true
        sudo rm -f "/usr/local/bin/flamelang" "/usr/local/bin/flame" 2>/dev/null || true
    fi
fi

# 5. Determine and setup Blaze definition directories
echo -e "\n${BOLD}[2/3] Setting up Blaze standard library definition directory...${RESET}"

TARGET_DIRS=()

if [[ "$TARGET_IS_WINDOWS" == true ]]; then
    WIN_LOCALAPPDATA="$LOCALAPPDATA"
    if [[ -z "$WIN_LOCALAPPDATA" ]] && command -v powershell.exe &> /dev/null; then
        WIN_LOCALAPPDATA="$(powershell.exe -NoProfile -Command '$env:LOCALAPPDATA' 2>/dev/null | tr -d '\r')"
    fi
    WIN_PROGRAMFILES="$PROGRAMFILES"
    if [[ -z "$WIN_PROGRAMFILES" ]] && command -v powershell.exe &> /dev/null; then
        WIN_PROGRAMFILES="$(powershell.exe -NoProfile -Command '$env:ProgramFiles' 2>/dev/null | tr -d '\r')"
    fi

    if [[ -n "$WIN_PROGRAMFILES" ]]; then
        P_DIR="$(to_posix_path "$WIN_PROGRAMFILES/Blaze/std")"
        if [[ -n "$P_DIR" ]] && [ -w "$(dirname "$P_DIR")" ]; then
            TARGET_DIRS+=("$P_DIR")
        fi
    fi
    if [[ -n "$WIN_LOCALAPPDATA" ]]; then
        L_DIR="$(to_posix_path "$WIN_LOCALAPPDATA/Blaze/std")"
        if [[ -n "$L_DIR" ]]; then
            TARGET_DIRS+=("$L_DIR")
        fi
    fi
    if [[ -n "$WIN_USERPROFILE" ]]; then
        U_DIR="$(to_posix_path "$WIN_USERPROFILE/.blaze/std")"
        if [[ -n "$U_DIR" ]]; then
            TARGET_DIRS+=("$U_DIR")
        fi
    fi
    TARGET_DIRS+=("$HOME/.blaze/std")
else
    TARGET_DIRS+=("$HOME/.blaze/std")
    if [[ $EUID -eq 0 ]]; then
        TARGET_DIRS+=("/usr/local/share/blaze/std")
    fi
fi

# Locate source Blaze/std directory
SOURCE_BLAZE=""
if [[ -d "$SCRIPT_DIR/Blaze/std" ]]; then
    SOURCE_BLAZE="$SCRIPT_DIR/Blaze/std"
elif [[ -d "$SCRIPT_DIR/std" ]]; then
    SOURCE_BLAZE="$SCRIPT_DIR/std"
fi

CLEANUP_TEMP=""
if [[ -z "$SOURCE_BLAZE" || ! -d "$SOURCE_BLAZE" ]]; then
    echo -e "${YELLOW}Fetching Blaze standard library definitions from repository...${RESET}"
    TEMP_DIR="$(mktemp -d 2>/dev/null || mktemp -d -t 'flame_std')"
    CLEANUP_TEMP="$TEMP_DIR"
    if curl -fsSL "https://github.com/shoya-129/flame/archive/refs/heads/main.tar.gz" | tar -xz -C "$TEMP_DIR" 2>/dev/null; then
        if [[ -d "$TEMP_DIR/flame-main/Blaze/std" ]]; then
            SOURCE_BLAZE="$TEMP_DIR/flame-main/Blaze/std"
        elif [[ -d "$TEMP_DIR/flame-main/std" ]]; then
            SOURCE_BLAZE="$TEMP_DIR/flame-main/std"
        fi
    fi
fi

if [[ -n "$SOURCE_BLAZE" && -d "$SOURCE_BLAZE" ]]; then
    COPIED_COUNT=0
    PRIMARY_BLAZE_DIR=""

    for DEST in "${TARGET_DIRS[@]}"; do
        mkdir -p "$DEST" 2>/dev/null || true
        if [[ -d "$DEST" && -w "$DEST" ]]; then
            cp -r "$SOURCE_BLAZE"/* "$DEST/" 2>/dev/null || true
            echo -e "  Installed definitions to: ${GREEN}$DEST${RESET}"
            if [[ -z "$PRIMARY_BLAZE_DIR" ]]; then
                PRIMARY_BLAZE_DIR="$(dirname "$DEST")"
            fi
            COPIED_COUNT=$((COPIED_COUNT + 1))
        fi
    done

    if [[ $COPIED_COUNT -eq 0 ]]; then
        FALLBACK_DEST="$HOME/.blaze/std"
        mkdir -p "$FALLBACK_DEST"
        cp -r "$SOURCE_BLAZE"/* "$FALLBACK_DEST/"
        PRIMARY_BLAZE_DIR="$HOME/.blaze"
        echo -e "  Installed definitions to: ${GREEN}$FALLBACK_DEST${RESET}"
    fi
else
    echo -e "${YELLOW}Warning: Standard library definitions could not be located.${RESET}"
    echo -e "${YELLOW}Definitions can be initialized later via 'fmp update'.${RESET}"
fi

if [[ -n "$CLEANUP_TEMP" && -d "$CLEANUP_TEMP" ]]; then
    rm -rf "$CLEANUP_TEMP" 2>/dev/null || true
fi

# 6. Shell environment and permanent PATH persistence
echo -e "\n${BOLD}[3/3] Setting up environment and permanently persisting PATH...${RESET}"

if [[ "$TARGET_IS_WINDOWS" == true && -n "$(command -v powershell.exe)" ]]; then
    # Permanently append Cargo bin to Windows User PATH via PowerShell if not already present
    powershell.exe -NoProfile -ExecutionPolicy Bypass -Command '
        $target = [System.IO.Path]::Combine($env:USERPROFILE, ".cargo", "bin")
        $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
        if (-not ($userPath -split ";" -contains $target)) {
            $newPath = ($userPath.TrimEnd(";") + ";" + $target).TrimStart(";")
            [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        }
        $blazeDir = [System.IO.Path]::Combine($env:LOCALAPPDATA, "Blaze")
        [Environment]::SetEnvironmentVariable("BLAZE_HOME", $blazeDir, "User")
    ' 2>/dev/null || true
else
    # Linux / macOS shell rc file persistence
    RC_FILES=("$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.profile")
    if [[ -n "$REAL_HOME" && "$REAL_HOME" != "$HOME" ]]; then
        RC_FILES+=("$REAL_HOME/.bashrc" "$REAL_HOME/.zshrc" "$REAL_HOME/.profile")
    fi

    for RC in "${RC_FILES[@]}"; do
        if [[ -f "$RC" ]]; then
            if ! grep -q '\.cargo/bin' "$RC" 2>/dev/null; then
                echo -e "\n# Flame language and Cargo toolchain" >> "$RC"
                echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> "$RC"
            fi
            if ! grep -q '\.local/bin' "$RC" 2>/dev/null; then
                echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$RC"
            fi
            if ! grep -q 'BLAZE_HOME' "$RC" 2>/dev/null; then
                echo "export BLAZE_HOME=\"$PRIMARY_BLAZE_DIR\"" >> "$RC"
            fi
            if [[ $EUID -eq 0 && -n "$SUDO_USER" ]]; then
                chown "$REAL_USER" "$RC" 2>/dev/null || true
            fi
        fi
    done
fi

echo -e "\n${GREEN}${BOLD}✓ Flame and Blaze toolchain successfully installed!${RESET}\n"
echo -e "  Primary Command:  ${GREEN}fmp${RESET}"
echo -e "  Binary Location:  ${BLUE}$CARGO_BIN/fmp${RESET}"
echo -e "  Blaze Definitions:${BLUE}$PRIMARY_BLAZE_DIR/std${RESET}"

if ! command -v fmp &>/dev/null; then
    echo -e "\n${YELLOW}Notice: 'fmp' binary is ready at: ${GREEN}$CARGO_BIN/fmp${RESET}"
    echo -e "To use 'fmp' in your current terminal session, run:"
    echo -e "  ${GREEN}export PATH=\"$CARGO_BIN:\$PATH\"${RESET}"
    echo -e "or open a new terminal window.\n"
fi

echo -e "\n${BOLD}Quick Start:${RESET}"
echo -e "  Check version:    ${GREEN}fmp --version${RESET}"
echo -e "  CLI Help menu:    ${GREEN}fmp help${RESET}"
echo -e "  Update release:   ${GREEN}fmp update${RESET}"
echo -e "  Uninstall:        ${GREEN}fmp uninstall${RESET}\n"
