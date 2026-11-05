#!/usr/bin/env bash
# Installa il motore vocale neurale (Piper, CPU) e la voce italiana usata da
# CAA Writer. Richiede rete solo per l'installazione: la lettura è offline.
#
# Voce: it_IT-serena-medium (22,05 kHz, CC BY 4.0), derivata da
# committa/serena-synthetic-it-27h. L'attribuzione è obbligatoria.
set -euo pipefail
cd "$(dirname "$0")/.."

VOICE="it_IT-serena-medium"
BASE="https://huggingface.co/committa/${VOICE}/resolve/main"
EN_VOICE="en_GB-cori-high"
EN_BASE="https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_GB/cori/high"

python3 -m venv .voice-venv
.voice-venv/bin/pip install --upgrade pip >/dev/null
.voice-venv/bin/pip install piper-tts

mkdir -p resources/voice
if [ ! -f "resources/voice/${VOICE}.onnx" ]; then
  curl -L --fail -o "resources/voice/${VOICE}.onnx" "${BASE}/${VOICE}.onnx"
fi
if [ ! -f "resources/voice/${VOICE}.onnx.json" ]; then
  curl -L --fail -o "resources/voice/${VOICE}.onnx.json" "${BASE}/${VOICE}.onnx.json"
fi

if [ ! -f "resources/voice/${EN_VOICE}.onnx" ]; then
  curl -L --fail -o "resources/voice/${EN_VOICE}.onnx" "${EN_BASE}/${EN_VOICE}.onnx"
fi
if [ ! -f "resources/voice/${EN_VOICE}.onnx.json" ]; then
  curl -L --fail -o "resources/voice/${EN_VOICE}.onnx.json" "${EN_BASE}/${EN_VOICE}.onnx.json"
fi

echo "Voci installate in resources/voice/": $VOICE, $EN_VOICE
echo "Licenza voce italiana: CC BY 4.0 — derivata da committa/serena-synthetic-it-27h"
echo "Licenza voce inglese: vedere MODEL_CARD in rhasspy/piper-voices"
echo "Verifica: echo 'Ciao, questa è una prova.' | .voice-venv/bin/piper -m resources/voice/${VOICE}.onnx -c resources/voice/${VOICE}.onnx.json -f /tmp/prova.wav"
