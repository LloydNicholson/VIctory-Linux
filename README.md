# VIctory-Linux

A fork of [VIctory-7900XTX](https://github.com/avatarmanz/VIctory-7900XTX), itself a fork of
[VIctory](https://github.com/Marvisak/VIctory), a compatibility layer that runs an Xbox Series title from files the
user supplies. This one is about **running it on Linux**: the Windows build of `victory.exe` under Proton, with
vkd3d-proton on Mesa's RADV driver, on an AMD Radeon card. It is playable, not finished.

**This repository contains no game files, code or assets.** You need your own copy of the game. It is not affiliated
with or endorsed by the game's developer or publisher.

## Where it stands (2026-11-27)

Tested on one PC, with `main` (VIctory-7900XTX's `main` up to renderer port 6, and this fork's Linux work):

| | |
|---|---|
| CPU | AMD Ryzen 7 5700X (8 cores, 16 threads) |
| RAM | **32 GB** |
| GPU | AMD Radeon RX 9060 XT, 16 GB (RDNA 4) |
| OS | Nobara Linux 44 (Fedora), kernel 7.3, KDE Plasma 6.7 on Wayland, 1920x1080 |
| Driver | Mesa 26.2.6 (RADV) |
| Proton | Proton-CachyOS 11.0 (20261015), vkd3d-proton 1.1-5656 |
| Profile | Series X |

| | |
|---|---|
| Menus and title screens | 30 fps, while a "PREPARING SHADERS nn%" bar makes the pipelines earlier runs used |
| Prologue, prison scene | 86-91 ms a frame (about 11 fps), ~4,700 draws (timed with card steps on) |
| Crowded prison room | ~120 ms (8 fps), ~10,600 draws |
| Boat checkpoint ("Just Like Paradise") | 101-116 ms with the stock settings; **about 70 ms (14 fps)** with card steps, renderer ports 2, 5 and 6 and a 1080p output (`scripts/linux/play.conf.example.sh` says how) |
| Walking about on the first island | ~81-93 ms (11-12 fps) with the example switches; 106.8 ms before the two 16 GB-card switches (`VICTORY_CARD_BUFFERS_MIN_MB=1`, `VICTORY_WINDOWS_BAR_MB=4096`) |
| Opening cutscenes | 100-180 ms |
| Where the time goes | the thread that prepares submissions, not the card (the card is about half busy); the wineserver takes ~6% of one core |
| Memory | **32 GB is enough to play**: across six measured runs, with desktop apps open, at least 5.3 GB stayed free and swap grew by about 3 GB (6 GB on a first run with cold caches). |

Fixed since 2026-11-26:

- **Trees and palms no longer drop out** while you move about (whole tree models gone for runs of frames, in
  binoculars and in normal play, on Windows too). Raw views of buffers whose view did not begin on 16 bytes were made
  null, so the title's tree culling read zeros; they now begin at the view's own dword (`VICTORY_TWIN_EXACT`, on by
  default; offered upstream as [avatarmanz/VIctory-7900XTX#437](https://github.com/avatarmanz/VIctory-7900XTX/pull/437)).
  With `VICTORY_SHOW_AFTER=0` (in the example switches) trees also stop losing their leaves where they come into view.
- **Phone callers can be heard.** The title plays their voices through the audio DSP's short "phone speaker"
  convolution, for which the runtime gave silence. `VICTORY_DSP_PASS=4 VICTORY_DSP_PASS_GAIN=4` (in the example
  switches) passes those voices through dry; the speaker's filtering itself is not done yet.
- **An on-screen performance HUD** (Belobug's): `VICTORY_PANEL=1` or F3, sized by `VICTORY_PANEL_SCALE` and
  `VICTORY_PANEL_STEPS`.

What is known to be wrong on Linux:

- **Black ground in places outdoors** (sand under the mangroves solid black, things on it lit) with the Series X
  profile: the 16 GB card fills up and tens of thousands of textures get no copy on it. Started with
  `--console series-s` the ground is right (the card holds ~7 GB, the title renders at 1280x720 and upscales); on
  Series S, `VICTORY_FFX_SCALE` and a larger `VICTORY_OUTPUT` stop the title at start, so leave them off there.

- **Mouths need a workaround in RADV for now.** On the stock driver, `VICTORY_HALF_VIEWS=1` makes mouths move but the
  main characters' come out distorted and some faces shiny. The face network's first pass is a console shader whose
  lanes rely on the console's in-wave order across a group barrier; the translation does not keep that order, so the
  result depends on how the card schedules the workgroup (AMDVLK happens to match the console, RADV does not). RADV's
  barriers themselves are correct. Until the translation keeps the order, `scripts/linux/build-radv.sh` builds a RADV
  with `RADV_VICTORY_CU_MODE` (`scripts/linux/mesa/radv-cu-mode.patch.txt`, used by the launcher alone): with it such
  shaders run with the whole workgroup on one compute unit, and faces and mouths come out right at no measurable cost.
  Without it, leave `VICTORY_HALF_VIEWS` off: faces look natural but mouths do not move.
- `VICTORY_CARD_STEPS=2` (play.ps1's value) hands back wrong skinned meshes (stretched slabs over people, faces lost):
  use `0`, as the Linux launcher does.
- Glowing eyes in some close-ups; textures that appear late while the title streams.
- A first start after a build, driver or Proton change has nothing recorded to prepare yet and stutters in places;
  later starts prepare it behind the bar.
- Once, a start stopped about 7 s in on a breakpoint in the title's own code, before the title screens; starting
  again went through.

## How to run it

You need Linux with Steam (for the Steam Linux Runtime and a Proton, e.g. Proton-CachyOS), Mesa's RADV, a CPU with
AVX2, Rust (the toolchain is pinned), LLVM's `clang-cl`, `lld-link` and `llvm-lib`, the MSVC CRT and Windows SDK
unpacked by [xwin](https://github.com/Jake-Shadle/xwin), Microsoft's `d3dcompiler_47.dll`, and your own dump of the title.

```bash
git clone https://github.com/LloydNicholson/VIctory-Linux
cd VIctory-Linux
source scripts/linux/env.sh
cargo build --locked --profile diag -p victory --features diagnostics
```

If cargo says it `can't find crate for core` for `x86_64-pc-windows-msvc`, the Rust it found has no Windows
standard library: `rustup target add x86_64-pc-windows-msvc`, or keep a separate rustup for this and set
`VICTORY_RUST_HOME=<dir>` (holding `rustup/` and `cargo/`) before sourcing `env.sh`.

Then, **in a new shell without `env.sh`**, the page-mapping helper, and the compiler DLL next to the exe:

```bash
sh crates/victory-hle/unix/build.sh target/x86_64-pc-windows-msvc/diag
cp ~/.cache/winetricks/d3dcompiler_47/d3dcompiler_47.dll target/x86_64-pc-windows-msvc/diag/
```

Make the Proton prefix once with `scripts/linux/victory.sh init`, copy `scripts/linux/play.conf.example.sh` to
`~/.local/share/victory-proton/play.conf` and set your paths, and start with `scripts/linux/play.sh` or a desktop
launcher made from `scripts/linux/victory.desktop.example.txt`. The details, and what each piece is for:
[scripts/linux/README.md](scripts/linux/README.md).

**Wait for the shader bar before you start.** On the first screens a "PREPARING SHADERS nn%" bar makes the shaders
earlier runs needed (about two minutes from a cold cache on the PC above). Stay on the title screens until it is gone,
then press Enter: what it has not finished, the game makes as it draws, and the opening cutscene stutters. The very
first start (or the first after a build, Mesa or Proton change) has nothing recorded yet, so it stutters regardless;
the next start prepares what that one used.

## What this fork adds to VIctory-7900XTX

- **Memory holes known without asking Wine** (`VICTORY_TOLD_HOLES=1`, in the Linux launcher's switches): Wine cannot
  answer the question that tells a placeholder's holes, so each was asked about again every frame, ~20,000 times; the
  runtime now says where its placeholders are. 115 -> 108 ms a frame on the boat checkpoint.
- **Title pages mapped without the wineserver** (`victory_pages.so`, a Wine unix library that moves a section's pages
  into place with `mremap`): the wineserver walks every view on every map, which cost about 4.7 s a frame; ~0.2 s after.
- **Trees that stay put** (`VICTORY_TWIN_EXACT`, on) and **phone callers you can hear** (`VICTORY_DSP_PASS=4`): see above.
- **Radeon and vkd3d-proton fixes**: no reads at address 0 from heap-picked loads or empty root views, an empty
  pipeline library made with a null blob, pipelines whose compile never returns not waited for (64 MB worker stacks),
  one back edge per loop (dxil-spirv recursed without end), waits that notice a lost device.
- **Picture fixes from other forks**: a pipeline for the render target format actually bound (heads were missing),
  texture views the card wrote copied from the card (plastic-looking skin), boot images that changed after recording
  uploaded in the same submission.
- **Shaders prepared while the title boots**, behind a "PREPARING SHADERS nn%" bar, keyed on the Mesa and Proton in use.
- **The 16-bit face-network views** (`VICTORY_HALF_VIEWS=1`, off by default: see above).
- **The frame recorder under Proton** (`VICTORY_RECORD`), and `VICTORY_HOST_WAVE=32` to compare a card's two wave forms.
- **A Linux launcher** (`scripts/linux/`): build checks, a lock against overlapping runs, logs, and screenshots while
  you play, for finding faults.

Branches: `main` has all of it and follows VIctory-7900XTX's `main`; work happens on `dev` and lands on `main`. `pr/*` hold the Linux fixes one by one on VIctory-7900XTX's `main`, ready to offer
upstream. VIctory-7900XTX's own README, about its Windows build and what it changed:
[docs/README-7900XTX.md](docs/README-7900XTX.md); its findings: [docs/findings.md](docs/findings.md); upstream's
commands and switches: [docs/README-fxceless.md](docs/README-fxceless.md).

## Credits

This is a tree of other people's work with more on top. The history is kept, and commits taken from elsewhere name
the commit and its author.

- **[VIctory](https://github.com/Marvisak/VIctory)** by Marvisak and its contributors: the loader, the runtime
  services, the Direct3D 12 back end and the shader recompiler.
- **[VIctory-7900XTX](https://github.com/avatarmanz/VIctory-7900XTX)** by avatarmanz: the fast tree this one runs on,
  and the reporters on its tracker whose Linux findings (issues #13, #59, #111) pointed the way.
- **[fxceless/victory-fork](https://github.com/fxceless/victory-fork)**, **[DexNom/VIctory](https://github.com/DexNom/VIctory)**,
  **[Belobug/VIctory-Belobug-AMD](https://github.com/Belobug/VIctory-Belobug-AMD)**,
  **[FQY55kai/ZJOU-VICTORY](https://github.com/FQY55kai/ZJOU-VICTORY)**,
  **[myokex/victory-amd](https://github.com/myokex/victory-amd)**, **Geckomacabre**, **cxy0072019-sketch** and
  **tippythetaidum** (the d16 face fix), as credited in VIctory-7900XTX.
- **[XCL](https://github.com/ivall/xcl)** (GPL-2.0-only) was read for how the same title behaves under another layer;
  no code of it is here.
- This fork's changes were made with [Claude Code](https://claude.com/claude-code).

## License

GPL-3.0-or-later, as VIctory. See [LICENSE](LICENSE); the one file that is not covered is named in
[docs/asset-policy.md](docs/asset-policy.md).
