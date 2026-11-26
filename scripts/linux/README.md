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
- `VICTORY_HALF_VIEWS=1` (mouths move) is off: on Linux the two main characters' mouths come out distorted and out of
  time with it, while other characters' are right. On AMDVLK they are right, on RADV (also Mesa 27.0-devel) not.

## Known on Linux

- **Main characters' mouths** with `VICTORY_HALF_VIEWS=1` (or 7900XTX PR #114's version): distorted and mistimed.
  Ruled out so far: a bad checkpoint save, card steps, relay stage 2, landing echo and the other copy shortcuts (all
  off together still distorts), the card's handling of 16-bit views (test `heap_half_tests`), the wave form
  (`VICTORY_HOST_WAVE=32` distorts too). AMD's own driver (AMDVLK 2025.Q2.1) draws them right: it points at RADV.
- `VICTORY_LAND_ECHO=0` moves face faults between characters rather than fixing them.
- Recording a frame (`VICTORY_RECORD`) works under Proton (the recorder hands lists to vkd3d-proton with their own
  method tables) and needs ~6 GB of RAM more than playing.
