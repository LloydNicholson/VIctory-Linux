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
# Screenshots while you play (play-shots/), minutes apart; 0: none.
SHOT_EVERY_MIN=3
# scripts/play.ps1's switches (no ray tracing), with VICTORY_CARD_STEPS=0: on Linux (and on some Windows cards) card
# steps hand back wrong skinned meshes (stretched slabs over people, faces lost; 7900XTX issue #13), at little speed.
# VICTORY_TOLD_HOLES=1: placeholders known without asking Wine, which cannot answer (115 -> 108 ms a frame).
# VICTORY_HALF_VIEWS=1 (mouths move) is left out: on Linux the main characters' mouths come out distorted with it.
SWITCHES="VICTORY_TEXTURE_NO_WAIT=1 VICTORY_RELAY=2 VICTORY_WIDE_BACK=1 VICTORY_CARD_REHASH_RUNS=1 VICTORY_JOIN_COPIES=1
VICTORY_INDIRECT_WINDOW=1 VICTORY_RELAY_COPY=1 VICTORY_CARD_STEPS=0 VICTORY_INDIRECT_BATCH=2 VICTORY_MAP_THREADS=1
VICTORY_TITLE_PRIORITIES=0 VICTORY_CARD_AWAKE=1 VICTORY_SHOW_AFTER=1 VICTORY_AMBIENT=0.8 VICTORY_D3D12_DRED=light
VICTORY_THIN=1 VICTORY_JOIN_PARTS=8 VICTORY_UPLOAD_AHEAD=1 VICTORY_TOLD_HOLES=1"
