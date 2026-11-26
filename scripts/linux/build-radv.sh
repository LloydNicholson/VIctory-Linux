#!/bin/bash
# build-radv.sh [mesa tag]: builds Mesa's RADV (the AMD Vulkan driver) with mesa/radv-cu-mode.patch.txt into
# VICTORY_STATE/mesa-local (default ~/.local/share/victory-proton/mesa-local), for the launcher alone: set DRIVER_JSON in
# play.conf to the path this prints. The system's driver is not touched. Needs git, a C/C++ compiler, ninja, python3
# (meson, mako and pyyaml go into a venv here), and the usual RADV build dependencies (libdrm, libelf, zstd, wayland,
# xcb/xshmfence...; Mesa fetches expat itself if it is missing). Tag default: mesa-26.2.6, the version the patch is for.
set -e
HERE=$(cd "$(dirname "$(readlink -f "$0")")" && pwd)
W=${VICTORY_STATE:-$HOME/.local/share/victory-proton}
TAG=${1:-mesa-26.2.6}
SRC=$W/mesa-src
PREFIX=$W/mesa-local
if [ ! -d "$SRC/.git" ]; then
  git clone --depth 1 --branch "$TAG" https://gitlab.freedesktop.org/mesa/mesa.git "$SRC"
fi
cd "$SRC"
git checkout -q -- . && git clean -qfd src
git apply --check "$HERE/mesa/radv-cu-mode.patch.txt" && git apply "$HERE/mesa/radv-cu-mode.patch.txt"
[ -x .venv/bin/meson ] || { python3 -m venv .venv && .venv/bin/pip install -q meson mako pyyaml packaging; }
export PATH=$SRC/.venv/bin:$PATH
[ -f build/build.ninja ] || meson setup build -Dprefix="$PREFIX" -Dbuildtype=release -Db_ndebug=true \
  -Dvulkan-drivers=amd -Dgallium-drivers= -Dplatforms=x11,wayland -Dllvm=disabled -Dvideo-codecs= \
  -Dglx=disabled -Degl=disabled -Dgles1=disabled -Dgles2=disabled -Dopengl=false -Dgbm=disabled \
  -Dvulkan-layers= -Dtools= -Dbuild-tests=false
ninja -C build
meson install -C build --quiet
echo
echo "Built. In play.conf:"
echo "DRIVER_JSON=\"$PREFIX/share/vulkan/icd.d/radeon_icd.x86_64.json\""
echo "and add RADV_VICTORY_CU_MODE=1 VICTORY_HALF_VIEWS=1 to SWITCHES."
