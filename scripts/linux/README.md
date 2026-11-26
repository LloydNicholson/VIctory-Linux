# Running VIctory-Linux on Linux (Proton, vkd3d-proton, RADV)

Everything here was worked out on one PC (see the top of the repository README). It runs the Windows build of
`victory.exe` under Proton; nothing is native Linux except a small helper library for page mapping.

## Build (on Linux, no Windows needed)

1. Unpack the MSVC CRT and Windows SDK with [xwin](https://github.com/Jake-Shadle/xwin) into `~/.local/share/xwin-sdk`
   (or set `XWIN_DIR`), and have `clang-cl`, `lld-link` and `llvm-lib` (LLVM) installed.
2. Build the exe:
   ```bash
   source scripts/linux/env.sh
   cargo build --locked --profile diag -p victory --features diagnostics
   ```
3. In a **new shell without `env.sh`** (with it, the library comes out broken, about 2.7 KB instead of about 9.9 KB):
   ```bash
   sh crates/victory-hle/unix/build.sh target/x86_64-pc-windows-msvc/diag
   ```
   This builds `victory_pages.so`, which lets title pages be mapped without asking the wineserver (about 4.7 s a frame
   down to about 0.2 s on the tree this was first measured on).
4. Copy Microsoft's `d3dcompiler_47.dll` (e.g. from `winetricks d3dcompiler_47`'s cache) next to `victory.exe`.
   Without it Wine's own compiler fails the indirect clamp, every indirect draw is dropped, and the game runs at ~3 fps.

## Play

1. `scripts/linux/victory.sh init` once, to make the Proton prefix (in `~/.local/share/victory-proton`).
2. Copy `play.conf.example.sh` to `~/.local/share/victory-proton/play.conf` and set `REPO`, `GAME`, `MOUNT` and `PROTON`.
3. Copy `victory.desktop.example.txt` to `~/.local/share/applications/victory.desktop` (and to the desktop) and fix its `Exec` paths.

`play.sh` checks the build and its two companion files, holds `game.lock` while you play (so scripted test runs
keep off), keeps the last 10 logs in `play-logs/`, and takes a screenshot every few minutes while the game window
is the active one (`play-shots/<session>/` with `index.txt`: fps and draws from the window title). Bind
`play-shot.sh mark` to a key to take one whenever something looks wrong.

The first start after a build, driver or Proton change records which pipelines the game uses; later starts make them
behind a "PREPARING SHADERS nn%" bar on the first screens, so the opening cutscene does not stutter.

## Switches (in play.conf)

`scripts/play.ps1`'s set, except:

- `VICTORY_CARD_STEPS=0`: with `2`, skinned meshes come back wrong (stretched slabs over people, faces lost); seen on
  Linux and on some Windows cards (7900XTX issue #13). Costs little.
- `VICTORY_HALF_VIEWS=1` (mouths move) is off unless the launcher uses the patched RADV (below): on the stock RADV the
  two main characters' mouths come out distorted with it.

## Known on Linux

- **Main characters' mouths** with `VICTORY_HALF_VIEWS=1` on the stock RADV: distorted (some faces shiny). Found:
  RADV runs compute shaders that use no shared memory in WGP mode, one workgroup's waves on both CUs of a WGP with a
  vector L0 cache each, and on RDNA 4 emits no L0 invalidation at a workgroup-scope acquire; the face network's
  shaders pass values between waves through a buffer across group barriers and read stale ones. Shown on a frame
  replay of the first diverging pass (bit-identical to the stock driver unpatched; matching a CPU emulation of the
  console code, and AMDVLK, with the patch) and in game (faces and mouths right).
  **The fix, used by the launcher alone:** `scripts/linux/build-radv.sh` builds Mesa 26.2.6's RADV with
  `mesa/radv-cu-mode.patch.txt` into `~/.local/share/victory-proton/mesa-local` (the system's driver is not touched), then
  in play.conf: `DRIVER_JSON=<the path it prints>` and `RADV_VICTORY_CU_MODE=1 VICTORY_HALF_VIEWS=1` in SWITCHES.
  `RADV_VICTORY_CU_MODE=1` puts only compute shaders with a workgroup-scope acquire over storage memory in CU mode
  (`=2`: every compute shader); unset, the patched driver behaves as the stock one. Cost: none measurable (~106 ms a
  frame either way). The first start after building prepares shaders again behind the bar.
- `VICTORY_LAND_ECHO=0` moves face faults between characters rather than fixing them.
- Recording a frame (`VICTORY_RECORD`) works under Proton (the recorder hands lists to vkd3d-proton with their own
  method tables) and needs ~6 GB of RAM more than playing.
