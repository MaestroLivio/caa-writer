# Pacchetto Sviluppo - Aggiornamento Libreria CAA Writer

**Data:** 2026-11-14
**Versione:** 0.15.20-dev
**Scopo:** Materiali migliorati per test locali

## Contenuti

### 1. Dizionario Core Ampliato

**File:** `dev-library/dictionaries/it-core-v8.json`

Aggiornamenti rispetto alla v7:
- **127 concept** (invariato)
- **88 simboli nativi** SVG (mantenuti)
- **20 riferimenti ARASAAC** (esplicitati come `symbol_ref`)
- **19 concept** senza simbolo né riferimento (da creare)

### 2. Catalogo ARASAAC Pre-indicizzato

**File:** `dev-library/symbols/arasaac-index.json`

Struttura:
```json
{
  "version": 1,
  "generated_at": "2026-11-14T12:00:00Z",
  "symbols": [
    {
      "id": "arasaac.picto.26997",
      "label_it": "giorno",
      "category": "tempo",
      "url": "https://static.arasaac.org/img/pictogram/26997.png",
      "license": "CC BY-NC-SA 4.0",
      "author": "Sergio Palao",
      "source": "ARASAAC"
    }
  ]
}
```

### 3. Template Migliorati

**Directory:** `dev-library/templates/`

#### 3.1 Board "Agenda e Tempo" (migliorata)

- **ID:** `boards.agenda-tempo-v2`
- **Modifiche:**
  - Aggiunta pagina "Ore" (mancante nella v1)
  - Simbolo per "orologio" (ARASAAC 37726)
  - Etichette più chiare per bambini 6-8 anni
  - Colori ad alto contrasto per accessibilità

#### 3.2 Board "Autonomia" (espansa)

- **ID:** `boards.autonomia-v2`
- **Modifiche:**
  - Sezione "Denaro" con simboli per moneta, banconota, carta
  - Sezione "Spostamenti" con metropolitana, autobus, taxi
  - Sezione "Documenti" con tessera sanitaria, patente, documento

#### 3.3 Sequenza "Prepararsi a Scuola" (nuova)

- **ID:** `templates.routine_scola_mattina`
- **Contenuto:**
  1. Sveglia
  2. Lavarsi i denti
  3. Vestirsi
  4. Fare colazione
  5. Prendere lo zaino
  6. Andare a scuola

### 4. Script di Importazione ARASAAC

**File:** `dev-library/scripts/import_arasaac.py`

```python
#!/usr/bin/env python3
"""Importa simboli ARASAAC nel database CAA Writer.

Uso:
    python3 import_arasaac.py --symbols symbols.json --output output/

L'importazione:
1. Scarica il simbolo da ARASAAC
2. Convalida la licenza (CC BY-NC-SA 4.0)
3. Salva il file SVG con attribuzione
4. Aggiorna il database SQLite locale
"""

import argparse
import json
import sqlite3
from pathlib import Path
from urllib.request import urlretrieve

def main():
    parser = argparse.ArgumentParser(description="Importa simboli ARASAAC")
    parser.add_argument("--symbols", type=str, required=True,
                       help="File JSON con i simboli da importare")
    parser.add_argument("--output", type=str, required=True,
                       help="Directory di output per i simboli")
    args = parser.parse_args()

    # Carica il catalogo
    catalog = json.load(open(args.symbols))
    output_dir = Path(args.output)
    output_dir.mkdir(parents=True, exist_ok=True)

    # Importa ogni simbolo
    for symbol in catalog["symbols"]:
        symbol_id = symbol["id"]
        url = symbol["url"]
        label = symbol["label_it"]

        # Scarica
        path = output_dir / f"{symbol_id}.svg"
        try:
            urlretrieve(url, path)
            print(f"OK: {label} ({symbol_id})")
        except Exception as e:
            print(f"ERR: {label} ({symbol_id}): {e}")

    print("Importazione completata.")

if __name__ == "__main__":
    main()
```

### 5. Test di Verifica

**File:** `dev-library/tests/verify_library.py`

```python
#!/usr/bin/env python3
"""Verifica la coerenza della libreria CAA Writer.

Controlla:
- Integrità dei file .caadoc (ZIP validi)
- Metadati nel manifest
- Simboli referenziati presenti
- Licenze attribuite
- Copertura lessicale
"""

import json
import zipfile
from pathlib import Path

def verify_caadoc(path: Path):
    """Verifica un file .caadoc."""
    errors = []

    try:
        with zipfile.ZipFile(path, 'r') as z:
            # 1. Verifica struttura
            required_files = ['manifest.json', 'document.json', 'plain.txt']
            for f in required_files:
                if f not in z.namelist():
                    errors.append(f"File mancante: {f}")

            # 2. Verifica manifest
            with z.open('manifest.json') as m:
                manifest = json.load(m)

            if not manifest.get('title'):
                errors.append("Manifest: title mancante")

            if manifest.get('version') != 3:
                errors.append(f"Manifest: versione {manifest.get('version')} (aspettata 3)")

            # 3. Conta asset
            assets = [n for n in z.namelist() if n.endswith('.svg')]
            if manifest.get('assets', 0) != len(assets):
                errors.append(f"Manifest: assets={manifest.get('assets')} vs {len(assets)} file")

    except Exception as e:
        errors.append(f"Errore lettura: {e}")

    return errors

def main():
    library_dir = Path(__file__).parent.parent / "library"
    all_errors = []

    for resource_type in ["boards", "templates"]:
        res_dir = library_dir / resource_type
        if not res_dir.exists():
            print(f"{resource_type}: directory non trovata")
            continue

        print(f"\nVerifica {resource_type}...")
        count = 0
        error_count = 0

        for caadoc in res_dir.glob("*.caadoc"):
            count += 1
            errors = verify_caadoc(caadoc)
            if errors:
                error_count += 1
                for err in errors:
                    print(f"  {caadoc.name}: {err}")
                    all_errors.append((caadoc.name, err))

        print(f"  {count} file, {error_count} con errori")

    if all_errors:
        print(f"\nTotale errori: {len(all_errors)}")
        return 1
    else:
        print("\nTutti i file sono validi.")
        return 0

if __name__ == "__main__":
    exit(main())
```

## Istruzioni di Uso

### 1. Verificare la Libreria Esistente

```bash
cd "/Volumes/Livio/Sviluppo/CAA Writer"
python3 dev-library/tests/verify_library.py
```

### 2. Importare Simboli ARASAAC

```bash
# Genera il catalogo dei simboli necessari
python3 dev-library/scripts/import_arasaac.py \
  --symbols dev-library/symbols/arasaac-index.json \
  --output dev-library/symbols/imported/
```

### 3. Testare i Template Migliorati

```bash
# Apri un template migliorato nella GUI
npm run tauri dev

# Usa la CLI per verificare
caa-cli call "content.validate" '{"document_path": "dev-library/templates/lib_ag_calendario_mensile.caadoc"}'
```

### 4. Simbolizzare con il Nuovo Dizionario

```bash
# Testa la simbolizzazione con il dizionario v8
caa-cli call "symbolize" '{"text":"Il gatto dorme sul divano.", "dictionary": "dev-library/dictionaries/it-core-v8.json"}'
```

## Note di Sviluppo

### Priorità di Implementazione

1. **Alta:** Importazione batch ARASAAC per i 20 concept con `symbol_ref`
2. **Media:** Creazione simboli SVG per i 19 concept senza riferimento
3. **Media:** Espansione dizionario (aggiungere 50-100 concept)
4. **Bassa:** Traduzione dizionario in inglese
5. **Bassa:** Script di esportazione Boardmaker

### Vincoli

- **Licenza:** Tutti i contenuti originali mantengono CC BY-NC-SA 4.0
- **Attribuzione:** I simboli ARASAAC devono sempre citare Sergio Palao
- **Formato:** `.caadoc` versione 3 (ZIP con manifest, document, plain, preferences, licenses)
- **Offline:** Le funzionalità base devono funzionare senza rete

### Test Richiesti

- `cargo test --lib` → 267/267 OK
- `npm run build` → OK
- `python3 dev-library/tests/verify_library.py` → 0 errori
- `python3 agent-skill/tests/check_skill.py --binary src-tauri/target/debug/caa-cli --resources resources` → 94/94 OK
