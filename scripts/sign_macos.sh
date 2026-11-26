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

BIN_DIR="$APP/Contents/MacOS"
FRAMEWORKS="$APP/Contents/Frameworks"
BINS=("$BIN_DIR/caa-writer" "$BIN_DIR/caa-cli" "$BIN_DIR/caa-mcp")

DAV1D_PATH=""
for BIN in "${BINS[@]}"; do
  [ -f "$BIN" ] || continue
  FOUND="$(otool -L "$BIN" | awk '/libdav1d.*\.dylib/ {print $1; exit}')"
  [ -n "$FOUND" ] || continue
  if [ "$FOUND" != "@rpath/libdav1d.7.dylib" ]; then
    DAV1D_PATH="$FOUND"
    break
  fi
done

if [ -n "$DAV1D_PATH" ]; then
  if [ ! -f "$DAV1D_PATH" ]; then
    echo "errore: $DAV1D_PATH non esiste; installa dav1d (brew install dav1d)" >&2
    exit 1
  fi
  mkdir -p "$FRAMEWORKS"
  cp -f "$DAV1D_PATH" "$FRAMEWORKS/libdav1d.7.dylib"
  chmod u+w "$FRAMEWORKS/libdav1d.7.dylib"
  install_name_tool -id "@rpath/libdav1d.7.dylib" "$FRAMEWORKS/libdav1d.7.dylib"
fi

for BIN in "${BINS[@]}"; do
  [ -f "$BIN" ] || continue
  FOUND="$(otool -L "$BIN" | awk '/libdav1d.*\.dylib/ {print $1; exit}')"
  if [ -n "$FOUND" ] && [ "$FOUND" != "@rpath/libdav1d.7.dylib" ]; then
    install_name_tool -change "$FOUND" "@rpath/libdav1d.7.dylib" "$BIN"
  fi
  if [ -f "$FRAMEWORKS/libdav1d.7.dylib" ] &&
    ! otool -l "$BIN" | grep -q "path @executable_path/../Frameworks"; then
    install_name_tool -add_rpath "@executable_path/../Frameworks" "$BIN"
  fi
done

find "$APP" -name '._*' -delete
xattr -cr "$APP"
codesign --force --deep --sign - --timestamp=none "$APP"
codesign --verify --deep --strict --verbose=2 "$APP"

# Nessun eseguibile distribuito può dipendere dalla macchina di build,
# dalla cartella di sviluppo o da Homebrew.
for BIN in "${BINS[@]}"; do
  [ -f "$BIN" ] || continue
  DEPS="$(otool -L "$BIN")"
  if echo "$DEPS" | grep -Eqi '/(opt/homebrew|Volumes/|Users/)'; then
    echo "errore: dipendenza esterna al bundle in $BIN:" >&2
    echo "$DEPS" | grep -Ei '/(opt/homebrew|Volumes/|Users/)' >&2
    exit 1
  fi
  if echo "$DEPS" | grep -q '@rpath/libdav1d.7.dylib' &&
    [ ! -f "$FRAMEWORKS/libdav1d.7.dylib" ]; then
    echo "errore: libdav1d.7.dylib non è inclusa nel bundle" >&2
    exit 1
  fi
done

# Controlla anche le librerie/framework annidati nel bundle: l'assenza di
# riferimenti esterni deve valere per l'intera app, non solo per i tre entrypoint.
while IFS= read -r -d '' MACHO; do
  file -b "$MACHO" | grep -q 'Mach-O' || continue
  DEPS="$(otool -L "$MACHO")"
  if echo "$DEPS" | grep -Eqi '/(opt/homebrew|Volumes/|Users/)'; then
    echo "errore: dipendenza esterna al bundle in $MACHO:" >&2
    echo "$DEPS" | grep -Ei '/(opt/homebrew|Volumes/|Users/)' >&2
    exit 1
  fi
done < <(find "$APP/Contents" -type f -print0)
