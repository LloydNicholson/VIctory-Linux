#!/bin/bash
# play-shot.sh [timed|mark]: one screenshot of the game while you play, into the running play session's folder
# (VICTORY_STATE/play-shots/<start>/, with index.txt: time, the window title's fps and draws, the log's line count).
# "mark" (default; bind it to a key, e.g. a KDE custom shortcut) always takes it and says so; "timed" (play.sh) only
# when the game window is the active one, so nothing is brought forward. Needs wmctrl, xprop, spectacle, ImageMagick.
W=${VICTORY_STATE:-$HOME/.local/share/victory-proton}; kind=${1:-mark}
D=$(cat "$W/play-shots/current" 2>/dev/null); [ -d "$D" ] || { notify-send -a Victory "Victory" "No play session is running." 2>/dev/null; exit 1; }
id=$(wmctrl -l | awk '/VIctory/{print $1; exit}'); [ -n "$id" ] || exit 1
if [ "$kind" = timed ]; then
  act=$(xprop -root _NET_ACTIVE_WINDOW 2>/dev/null | awk '{print $NF}')
  [ -n "$act" ] && [ $((act)) -eq $((id)) ] || exit 0
fi
t=$(date +%H%M%S); f="$D/$t${kind:+-$kind}.jpg"; tmp="$D/.x$$.png"
spectacle -b -n -f -o "$tmp" >/dev/null 2>&1 && magick "$tmp" -quality 85 "$f"; rm -f "$tmp"
title=$(wmctrl -l | awk -v i="$id" '$1==i{$1=$2=$3=""; sub(/^ +/,""); print}')
L=$(cat "$D/log.txt" 2>/dev/null); n=$(wc -l < "$L" 2>/dev/null)
echo "$t $kind log-line=$n | $title" >> "$D/index.txt"
[ "$kind" = mark ] && notify-send -a Victory -t 1500 "Victory" "Screenshot saved." 2>/dev/null
exit 0
