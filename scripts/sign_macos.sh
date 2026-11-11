#!/usr/bin/env bash
# Firma ad-hoc un bundle macOS dopo che Tauri ha aggiunto le risorse.
#
# Tauri firma il binario linker-signed, ma le risorse copiate dopo invalidano
# la firma: macOS mostra «danneggiata». Questo script rimuove i file AppleDouble,
# pulisce gli attributi estesi e rifirma l'intero bundle.
set -euo pipefail
APP="${1:?Uso: scripts/sign_macos.sh \"/percorso/CAA Writer.app\"}"
find "$APP" -name '._*' -delete
xattr -cr "$APP"
codesign --force --deep --sign - --timestamp=none "$APP"
codesign --verify --deep --strict --verbose=2 "$APP"
