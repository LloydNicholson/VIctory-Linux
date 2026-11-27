#!/bin/bash
# play.sh [conf]: starts the game for play (for a desktop launcher). conf: see play.conf.example.sh (default
# VICTORY_STATE/play.conf). Holds VICTORY_STATE/game.lock while the game runs, so test runs (and agents) keep off, and
# refuses to start while it is held. Log: VICTORY_STATE/play-logs/<start>.log (last 10 kept). Screenshots every
# SHOT_EVERY_MIN while the game window is the active one: VICTORY_STATE/play-shots/<start>/ (see play-shot.sh).
HERE=$(cd "$(dirname "$(readlink -f "$0")")" && pwd)
W=${VICTORY_STATE:-$HOME/.local/share/victory-proton}; mkdir -p "$W"
CONF=${1:-$W/play.conf}
say() { notify-send -a Victory -i applications-games "Victory" "$1" 2>/dev/null || echo "$1" >&2; }
[ -r "$CONF" ] || { say "No launcher settings at $CONF (copy scripts/linux/play.conf.example.sh)."; exit 1; }
# shellcheck source=/dev/null
. "$CONF"
[ -f "$EXE" ] || { say "No build at $EXE. Build it first."; exit 1; }
for f in victory_pages.so d3dcompiler_47.dll; do
  [ -e "$(dirname "$EXE")/$f" ] || { say "$f is missing next to $EXE (without it the game is slow or broken)."; exit 1; }
done
# victory_pages.so built in a shell with env.sh sourced comes out broken (~2.7 KB; a good one is ~9.9 KB).
[ "$(stat -c %s "$(dirname "$EXE")/victory_pages.so")" -gt 5000 ] || { say "victory_pages.so next to $EXE looks broken (rebuild it without env.sh sourced)."; exit 1; }
if [ -e "$W/game.lock" ]; then say "Not started: the game slot is in use ($(head -c 200 "$W/game.lock"))."; exit 1; fi
if ps -eo args | grep -qE '^Z:.*victory\.exe run'; then say "Not started: a game is already running."; exit 1; fi
echo "You are playing (desktop launcher, $NAME), from $(date +%T)" > "$W/game.lock"
trap 'grep -qF "You are playing (desktop launcher" "$W/game.lock" 2>/dev/null && rm -f "$W/game.lock"' EXIT
mkdir -p "$W/play-logs"; LOG="$W/play-logs/$(date +%Y%m%d-%H%M%S).log"
ls -1t "$W"/play-logs/*.log 2>/dev/null | tail -n +11 | xargs -r rm -f
for kv in $SWITCHES; do export "$kv"; done
export VICTORY_REPO="$REPO" VICTORY_PROTON="$PROTON" VICTORY_EXE="$EXE" VICTORY_STATE="$W"
# DRIVER_JSON in the conf: a Vulkan driver of our own (the patched RADV that build-radv.sh makes) instead of the system's.
if [ -n "$DRIVER_JSON" ]; then
  [ -r "$DRIVER_JSON" ] || { say "No driver at $DRIVER_JSON (DRIVER_JSON in $CONF)."; exit 1; }
  DRIVER_LIB=$(sed -n 's/.*"library_path": *"\([^"]*\)".*/\1/p' "$DRIVER_JSON")
  [ -r "$DRIVER_LIB" ] || { say "The driver $DRIVER_JSON names is missing: $DRIVER_LIB"; exit 1; }
  export VK_ICD_FILENAMES="$DRIVER_JSON" VK_DRIVER_FILES="$DRIVER_JSON"
  STACK="radv-local $(stat -c %Y "$DRIVER_LIB")"
else
  STACK="radv $(stat -c %Y /usr/lib64/libvulkan_radeon.so /usr/lib/x86_64-linux-gnu/libvulkan_radeon.so 2>/dev/null | head -1)"
fi
# A driver or Proton change changes this, so the startup loading bar makes the pipelines again.
export VICTORY_HOST_STACK="$STACK proton $(cat "$VICTORY_PROTON/version" 2>/dev/null)"
export VICTORY_CACHE_DIR="$CACHE_DIR" VICTORY_DATA_DIR="$DATA_DIR" VICTORY_GAME_DIR="$GAME" VICTORY_MOUNT="$MOUNT"
say "Starting $NAME. The first screens are up in about 10 s."
SHOTS="$W/play-shots/$(basename "$LOG" .log)"; mkdir -p "$SHOTS"; echo "$SHOTS" > "$W/play-shots/current"
echo "$LOG" > "$SHOTS/log.txt"
ls -1dt "$W"/play-shots/2*/ 2>/dev/null | tail -n +11 | xargs -r rm -rf
if [ "${SHOT_EVERY_MIN:-3}" != 0 ]; then
  ( sleep 60; while ps -eo args | grep -qE '^Z:.*victory\.exe run'; do "$HERE/play-shot.sh" timed >/dev/null 2>&1; sleep $(( ${SHOT_EVERY_MIN:-3} * 60 )); done ) &
  SHOOTER=$!
fi
# RUN_ARGS in the conf (e.g. "--console series-s"): extra arguments for the run; unset: none.
"$HERE/victory.sh" run "$GAME\\game_scarlett_master.exe" ${RUN_ARGS} --execute > "$LOG" 2>&1
rc=$?; echo "exit $rc" >> "$LOG"
[ -n "$SHOOTER" ] && kill "$SHOOTER" 2>/dev/null
rm -f "$W/play-shots/current"
[ $rc -ne 0 ] && grep -qa 'unhandled exception\|device removed' "$LOG" && say "The game stopped with an error (exit $rc). Log: $LOG"
exit $rc
