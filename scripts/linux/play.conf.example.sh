# play.conf (a shell file play.sh sources): what play.sh runs. Copy to ~/.local/share/victory-proton/play.conf (or anywhere) and edit.
NAME="VIctory-Linux"
REPO="$HOME/Apps/VIctory-Linux"
EXE="$REPO/target/x86_64-pc-windows-msvc/diag/victory.exe"
# Shader cache and save data inside the Proton prefix.
CACHE_DIR='C:\victory-cache'
DATA_DIR='C:\victory-data'
PROTON="$HOME/.local/share/Steam/compatibilitytools.d/Proton-CachyOS Latest"
# The game folder as Windows sees it (Z: is /), and the host folder it lives on (shared into the runtime).
GAME='Z:\path\to\the\game'
MOUNT=/path/to/the/drive
# The Vulkan driver: empty for the system's. For moving mouths, build the patched RADV (scripts/linux/build-radv.sh),
# put the path it prints here, and add RADV_VICTORY_CU_MODE=1 VICTORY_HALF_VIEWS=1 to SWITCHES below.
DRIVER_JSON=""
# Screenshots while you play (play-shots/), minutes apart; 0: none.
SHOT_EVERY_MIN=3
# scripts/play.ps1's switches (no ray tracing), with VICTORY_CARD_STEPS=0: on Linux (and on some Windows cards) card
# steps hand back wrong skinned meshes (stretched slabs over people, faces lost; 7900XTX issue #13), at little speed.
# VICTORY_TOLD_HOLES=1: placeholders known without asking Wine, which cannot answer (115 -> 108 ms a frame).
# VICTORY_CARD_STEPS=1: the title's small fills of upload memory done on the card (106 -> 88 ms); 2 is the one that
# breaks skinned meshes. Renderer ports 2, 5 and 6 (VICTORY_PORT_DESCRIPTORS/CBV/GUARD): 79.7 -> 69.7 ms. Port 1
# (VICTORY_PORT_CONSTANTS=1) leaves the picture black under Proton: keep it off.
# VICTORY_OUTPUT=1920x1080 VICTORY_RENDER_SCALE=1.0 (not set here): the title renders for a 1080p output, 90 -> 80 ms,
# but its menus' buttons come out oversized; VICTORY_RENDER_SCALE=0.5 alone keeps them but smears (the upscaler).
# VICTORY_HALF_VIEWS=1 (mouths move) is left out: on the stock RADV the main characters' mouths come out distorted with
# it (see DRIVER_JSON above for the fix).
SWITCHES="VICTORY_TEXTURE_NO_WAIT=1 VICTORY_RELAY=2 VICTORY_WIDE_BACK=1 VICTORY_CARD_REHASH_RUNS=1 VICTORY_JOIN_COPIES=1
VICTORY_INDIRECT_WINDOW=1 VICTORY_RELAY_COPY=1 VICTORY_CARD_STEPS=1 VICTORY_INDIRECT_BATCH=2 VICTORY_MAP_THREADS=1
VICTORY_TITLE_PRIORITIES=0 VICTORY_CARD_AWAKE=1 VICTORY_SHOW_AFTER=1 VICTORY_AMBIENT=0.8 VICTORY_D3D12_DRED=light
VICTORY_THIN=1 VICTORY_JOIN_PARTS=8 VICTORY_UPLOAD_AHEAD=1 VICTORY_TOLD_HOLES=1
VICTORY_PORT_DESCRIPTORS=1 VICTORY_PORT_CBV=1 VICTORY_PORT_GUARD=1"
