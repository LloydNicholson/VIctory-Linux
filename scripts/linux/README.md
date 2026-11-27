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
- `VICTORY_SHOW_AFTER=0` (play.ps1 has `1`): with `1`, a texture new to its slot shows a grey 1x1 stand-in until its
  upload has run (a frame, up to four), and trees lose all their leaves for those frames where they come into view or
  at a cut (frame by frame at the boat checkpoint's mangroves). `0` shows none (stand-ins 90,117 -> 0 in a run), at the
  same fps. `VICTORY_SHOW_NEW_LATER` (on by default), the guard for textures that in-flight lists may read, stays.
- Two more for a 16 GB card with resizable BAR on (RX 9060 XT, 2026-11-27; leave them out on smaller cards, where
  they take the textures' room). The frame is held up by the CPU there (the card about half busy):
  - `VICTORY_CARD_BUFFERS_MIN_MB=1`: title buffers of 1-4 MiB get their copy in the card's memory too (default on
    16 GB: 4 MiB and up), so the card looks for what shaders wrote in them, not the thread that prepares
    submissions (11.8 -> 3.0 ms of it a frame): 106.8 -> 93.0 ms on the deck, ~100 MB more of the card.
  - `VICTORY_WINDOWS_BAR_MB=4096`: our copies of loose title memory go in the card's memory, written through the
    BAR, instead of the PC's memory shared with the card: that falls 7.0 -> 3.5 GB, the card rises 11.5 -> 13.8 GB,
    83.6 -> 81.2 ms in the kitchen. About 1 GB fewer textures then get a copy on the card: drop it if textures go
    blurry or black outdoors.
- Port 2 (`VICTORY_PORT_DESCRIPTORS=1`) alone, with 5 and 6 on: 89.7 -> 87.5 ms in the house, but ~20 warnings a
  second of a view record "found by the fallback": drop it if textures flash.

## Known on Linux

- **Main characters' mouths** with `VICTORY_HALF_VIEWS=1` on the stock RADV: distorted (some faces shiny). The first
  face pass (CS_FormulaOperation) diverges: on the console its lanes run a switch of formula cases in program order
  inside a wave and later cases read what earlier ones stored in the same iteration, with one group barrier per loop;
  the translated shader runs per thread, so those reads race and the result depends on the card's scheduling (RADV
  wave64, RADV wave32 and AMDVLK all differ; AMDVLK's matches a CPU emulation of the console code). RADV's barriers are
  not at fault: standalone tests (storage and 16/32-bit texel buffers, GLSL and memory-model barriers) find no stale
  reads. The proper fix belongs in the translation (keep the in-wave order, e.g. a group barrier between the cases).
  **Workaround, used by the launcher alone:** `scripts/linux/build-radv.sh` builds Mesa 26.2.6's RADV with
  `mesa/radv-cu-mode.patch.txt` into `~/.local/share/victory-proton/mesa-local` (the system's driver is not touched);
  then in play.conf `DRIVER_JSON=<the path it prints>` and `RADV_VICTORY_CU_MODE=1 VICTORY_HALF_VIEWS=1` in SWITCHES.
  `RADV_VICTORY_CU_MODE=1` runs compute shaders with a workgroup-scope acquire over storage memory with the whole
  workgroup on one CU (`=2`: every compute shader); unset, the patched driver behaves as the stock one. In game: faces
  and mouths right, ~106 ms a frame either way. The first start after building prepares shaders again behind the bar.
- `VICTORY_LAND_ECHO=0` moves face faults between characters rather than fixing them.
- Recording a frame (`VICTORY_RECORD`) works under Proton (the recorder hands lists to vkd3d-proton with their own
  method tables) and needs ~6 GB of RAM more than playing.
