#!/bin/bash
cd "/Volumes/Livio/Sviluppo/CAA Writer" || exit 1
for i in $(seq 1 40); do
  .image-venv/bin/python tutorial/build_clips.py >> tutorial/build.log 2>&1
  n=$(ls tutorial/clips/*.mp4 2>/dev/null | wc -l | tr -d ' ')
  echo "giri $i: clip pronti $n" >> tutorial/build.log
  [ "$n" -ge 10 ] && break
  sleep 45
done
