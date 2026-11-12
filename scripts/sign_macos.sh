#!/usr/bin/env bash
# Prepara e firma ad-hoc un bundle macOS dopo che Tauri ha aggiunto le risorse.
#
# Tauri firma il binario linker-signed, ma le risorse copiate dopo invalidano
# la firma: macOS mostra «danneggiata». Inoltre il supporto AVIF collega
# libdav1d di Homebrew con un percorso assoluto: su un Mac senza Homebrew
# l'app non parte. Questo script copia la libreria dentro il bundle, riscrive
# il percorso come @rpath, pulisce i file AppleDouble e rifirma tutto.
set -euo pipefail
APP="${1:?Uso: scripts/sign_macos.sh \"/percorso/CAA Writer.app\"}"

BIN="$APP/Contents/MacOS/caa-writer"
FRAMEWORKS="$APP/Contents/Frameworks"

if [ -f "$BIN" ]; then
  DAV1D_PATH="$(otool -L "$BIN" | awk '/libdav1d.*\.dylib/ {print $1; exit}')"
  if [ -n "$DAV1D_PATH" ]; then
    if [ "$DAV1D_PATH" = "@rpath/libdav1d.7.dylib" ]; then
      if [ ! -f "$FRAMEWORKS/libdav1d.7.dylib" ]; then
        echo "errore: il binario usa @rpath/libdav1d.7.dylib ma la libreria non è nel bundle" >&2
        exit 1
      fi
    else
      if [ ! -f "$DAV1D_PATH" ]; then
        echo "errore: $DAV1D_PATH non esiste; installa dav1d (brew install dav1d)" >&2
        exit 1
      fi
      mkdir -p "$FRAMEWORKS"
      cp -f "$DAV1D_PATH" "$FRAMEWORKS/libdav1d.7.dylib"
      chmod u+w "$FRAMEWORKS/libdav1d.7.dylib"
      install_name_tool -id "@rpath/libdav1d.7.dylib" "$FRAMEWORKS/libdav1d.7.dylib"
      install_name_tool -change "$DAV1D_PATH" "@rpath/libdav1d.7.dylib" "$BIN"
    fi
    if ! otool -l "$BIN" | grep -q "path @executable_path/../Frameworks"; then
      install_name_tool -add_rpath "@executable_path/../Frameworks" "$BIN"
    fi
  fi
fi

find "$APP" -name '._*' -delete
xattr -cr "$APP"
codesign --force --deep --sign - --timestamp=none "$APP"
codesign --verify --deep --strict --verbose=2 "$APP"

# La libreria deve essere raggiungibile solo dall'interno del bundle.
if otool -L "$BIN" | grep -q "/opt/homebrew"; then
  echo "errore: il binario dipende ancora da una libreria Homebrew" >&2
  exit 1
fi
