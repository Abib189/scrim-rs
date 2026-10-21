#!/bin/zsh
# run.sh OUT — Fix L2's final set: per speed, three runs with the camera far away, one with it on the
# work and one paused by a wheel turn, each on a fresh app (release build, control 7979) and scratch
# HOME. LAUNCH is a script that starts the app on a fresh HOME and waits for it (IE's ie-launch.sh).
OUT=$1; HERE=$(dirname $0); mkdir -p $OUT
swaps() { vm_stat | awk '/Swapouts/ {gsub("\\.","",$2); print $2}'; }
run() { # name speed camera sequence
  $LAUNCH $OUT/home $OUT/$1.app.log >/dev/null || { echo "$1 not up"; return; }
  sleep 1; a=$(swaps); l=$(sysctl -n vm.loadavg)
  python3 $HERE/l2.py 7979 $OUT/$1.json $2 $3 $4 2>&1 | tail -2
  b=$(swaps); echo "{\"load\": \"$l\", \"swapouts\": $((b-a))}" > $OUT/$1.conditions.json
}
for sp in fast normal slow; do
  for i in 1 2 3; do run $sp-far-$i $sp far brief; done
  run $sp-target $sp target brief
  run $sp-paused $sp paused brief
done
pkill -f "release/figcraft --control 7979"
