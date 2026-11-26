#!/bin/bash
# victory.sh [init | victory.exe arguments...]: runs victory.exe under Proton inside the Steam Linux Runtime, in a
# prefix of its own (VICTORY_STATE/compatdata). `victory.sh init` makes the prefix.
#   VICTORY_EXE     the exe to run (default: <VICTORY_REPO>/target/x86_64-pc-windows-msvc/diag/victory.exe)
#   VICTORY_REPO    the checkout (default: this script's repository)
#   VICTORY_PROTON  the Proton folder (default: Proton - Experimental in Steam)
#   VICTORY_MOUNT   extra host folders the runtime must see, colon-separated (e.g. the drive with the game)
#   VICTORY_STATE   where the prefix, logs and lock live (default: ~/.local/share/victory-proton)
HERE=$(cd "$(dirname "$(readlink -f "$0")")" && pwd)
W=${VICTORY_STATE:-$HOME/.local/share/victory-proton}; S=${STEAM_DIR:-$HOME/.local/share/Steam}
REPO=${VICTORY_REPO:-$(cd "$HERE/../.." && pwd)}
PROTON=${VICTORY_PROTON:-$S/steamapps/common/Proton - Experimental}
mkdir -p "$W/compatdata"
export STEAM_COMPAT_CLIENT_INSTALL_PATH=$S STEAM_COMPAT_DATA_PATH=$W/compatdata STEAM_COMPAT_INSTALL_PATH=$REPO
export STEAM_COMPAT_MOUNTS=$REPO:$W${VICTORY_MOUNT:+:$VICTORY_MOUNT}
export VICTORY_CONFIG=${VICTORY_CONFIG:-'C:\victory.toml'}
unset LD_LIBRARY_PATH; cd "$W" || exit 1
slr() { "$S/steamapps/common/SteamLinuxRuntime_4/_v2-entry-point" --verb=run -- "$PROTON/proton" "$@"; }
[ "$1" = init ] && { slr run cmd.exe /d /c ver; exit; }
# vkd3d-proton's d3d12 and dxgi, and Microsoft's d3dcompiler_47.dll next to the exe (Wine's own fails some shaders).
export WINEDLLOVERRIDES='d3d12,d3d12core,dxgi=n;d3dcompiler_47=n,b'
slr runinprefix "${VICTORY_EXE:-$REPO/target/x86_64-pc-windows-msvc/diag/victory.exe}" "$@"
