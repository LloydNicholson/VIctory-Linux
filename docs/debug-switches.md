# Debug switches for garbage pixels (NaN, infinities, flares)

Diagnostics builds only, all off unless set. They were made to find the terrarium
glass's blocks and star flares (a DPP move fetching from inactive lanes, fixed in
translator version 2) and work for any program that writes bad values.

## Quick start

`VICTORY_DEBUG_GLASS=<folder>[@x,y,w,h]` sets the three switches below at once
(each only where it is not set itself):

- `VICTORY_CAPTURE=<folder>`: presented frames as pictures; a file `go` appearing in
  the folder (the clip key, Meta+F8, makes it) lists the next three frames.
- `VICTORY_DUMP_TARGETS=1`: every target the listed frames draw into, as pictures; the
  log's `target values` lines count NaN and infinite texels (`nan_texels`, `inf_texels`).
- `VICTORY_TEXEL_TRACE=x,y,w,h`: before every draw into a half-float target that holds
  the box, the box is copied aside; `frame-<n>-texels.txt` says per draw (in recording
  order per command list, with the draw's shaders) how many texels are NaN, infinite or
  65,000 and more. The draw that turns the box bad lies between the last clean and the
  first bad line of a list. Default box: the middle of a 2560x1440 picture.

Unattended runs to the house shot after the save loads: `tools/glassrun.sh <name>
[VAR=v ...]` in `~/.local/share/victory-proton` (fresh copy of the saves, Enter through
the menus, trigger 6 s into the world, score with `tools/glassscore.py`).

## Finding the shader and the instruction

Shaders are named by the 16-digit hash the logs and listings give them. Named programs
are translated afresh in every run and not cached (the cache key stays the same).

- `VICTORY_SHADER_NAMES=<hash>,...`: log the names in those containers (entry names).
- `VICTORY_CLAMP_OPS=<hash>@<from>-<to>@<prefixes>@<limit>`: bound the VGPR results of
  instructions at byte offsets `from..to` (hex) whose name starts with one of `prefixes`
  to `[-limit, limit]`. It changes only values that blow up and never hides the
  program, so bisecting the range is reliable (the method that found the glass bug).
  Float prefixes: `v_mul_legacy_f32,v_mul_f32,v_fma_f32,v_fmac_f32,v_mac_f32,v_add_f32,
  v_sub_f32,v_subrev_f32,v_fma_mix_f32,v_max_f32,v_min_f32,v_mad_f32,v_rcp,v_rsq,v_exp,
  v_log,v_sqrt`.
- `VICTORY_DPP_OWN=<hash>`: the program's DPP lane moves give each lane its own value.
- `VICTORY_DPP_INACTIVE=0`: DPP moves fetching from inactive lanes as before the fix.
- `VICTORY_ZERO_INPUT=<hash>@s<slot>+<dword>|@dim<N>|@heap|@off<hex>...`: image reads
  from those table places, of that dimension (3: cubes), from the heap, or at that
  instruction give 0. Beware: zeroing a factor hides the surface (alpha, transmittance).
- `VICTORY_NAN_PROBE=<hash>@<from>-<to>` (with `VICTORY_NAN_PROBE_ON=<prefix>` or
  `!<prefixes>`, `VICTORY_NAN_PROBE_NAN=1`, `VICTORY_NAN_PROBE_LIMIT=<size>`,
  `VICTORY_NAN_PROBE_NEG=1`): discard a pixel at the first bad result. Harmless
  infinities in ordinary code make it discard whole surfaces: prefer `VICTORY_CLAMP_OPS`.
- `VICTORY_CLAMP_PIXEL=<hash>[:<cap>|:drop<limit>]`: bound a program's colour outputs
  (a workaround, not a fix).
- `VICTORY_EXACT_FLOAT=<hash>[:<bits>]`, `VICTORY_FTZ=<hash>`: the console's float rules
  exactly / 32-bit denormals flushed, for that program.

## Inputs and state

- `VICTORY_PICK_STORY=<hash>@s<slot>+<dword>@<user data>@<adds>`: the heap slots a
  material record names (32-bit words at the buffer's address + the user data value +
  each add) and what our heap holds there, in the frame listing.
- `VICTORY_TITLE_DUMP=<hash>@s<slot>+<dword>@<bytes>`: the title memory under a placed
  view of that program's table, written to the capture folder.
- `VICTORY_RAY_OFF=<trace,resolve,activate,bins|all>`: leave those ray-tracing passes
  out while the structure builder does not run.

Offline: `victory shader-info --disasm` disassembles containers; the frame listing
(`frame-<n>-ops.txt`) names each draw's table places as `s<slot>+<dword>`.
