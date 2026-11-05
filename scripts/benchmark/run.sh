#!/usr/bin/env bash
# Benchmark del motore CAA Writer.
#
# Esegue i test di prestazione e il corpus semantico in modalità release,
# scrivendo l'output in benchmark-output/ (esclusa da Git).
#
# Uso:
#   bash scripts/benchmark/run.sh
#   CAA_BENCH_RUNS=3 bash scripts/benchmark/run.sh   # più ripetizioni
#
# NOTA: queste misure riguardano la macchina su cui viene eseguito lo script.
# Non sostituiscono la verifica richiesta su Windows 10/11 con 4 GB di RAM.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT="$ROOT/benchmark-output"
RUNS="${CAA_BENCH_RUNS:-1}"

mkdir -p "$OUT"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
LOG="$OUT/bench-$STAMP.log"

echo "CAA Writer — benchmark del motore" | tee "$LOG"
echo "piattaforma: $(uname -srm)" | tee -a "$LOG"
echo "run: $RUNS" | tee -a "$LOG"

cd "$ROOT/src-tauri"

for run in $(seq 1 "$RUNS"); do
  echo "" | tee -a "$LOG"
  echo "=== run $run/$RUNS: prestazioni ===" | tee -a "$LOG"
  # shellcheck disable=SC2086
  cargo test --release --test performance -- --nocapture 2>&1 | tee -a "$LOG"

  echo "" | tee -a "$LOG"
  echo "=== run $run/$RUNS: corpus semantico ===" | tee -a "$LOG"
  cargo test --release --test semantic_corpus -- --nocapture 2>&1 | tee -a "$LOG"
done

if [ -f "$ROOT/tests/last_report.json" ]; then
  cp "$ROOT/tests/last_report.json" "$OUT/report-$STAMP.json"
  echo "" | tee -a "$LOG"
  echo "report copiato in $OUT/report-$STAMP.json" | tee -a "$LOG"
fi

echo "" | tee -a "$LOG"
echo "Log completo: $LOG"
echo "Ricorda: i numeri valgono per questa macchina, non per il requisito Windows 4 GB."
