# Analisi dei Sistemi Simbolici in CAA Writer

**Data:** 2026-11-14  
**Versione:** 0.15.20  
**Autore:** Agente di sviluppo CAA Writer

---

## Executive Summary

L'analisi rivela **tre sistemi simbolici distinti** con copertura parziale:

| Sistema | Simboli | Formato | Utilizzo |
|---------|---------|---------|----------|
| **Dizionario nativo** | 88 | `svg/categoria.nome.svg` | Disambiguazione semantica |
| **Dizionario ARASAAC** | 20 | `arasaac.picto.XXXX` | Concept senza simbolo nativo |
| **Board/Template** | 811 | `arasaac.picto.XXXX` | Materiali CAA pronti |

**Copertura lessicale:** solo il **20%** dei symbol_ref del dizionario (4/20) è effettivamente presente nei board/template.

**Gap critici:**
- 19 concept senza alcun simbolo (né nativo né ARASAAC)
- 807 simboli ARASAAC usati nei materiali ma non referenziati nel dizionario
- 16 symbol_ref del dizionario non presenti nei materiali

---

## 1. Dizionario Core Italiano

### 1.1 Statistiche

- **Concept totali:** 127
- **Concept con simbolo nativo:** 88 (69.3%)
- **Concept con symbol_ref ARASAAC:** 20 (15.7%)
- **Concept senza simboli:** 19 (15.0%)

### 1.2 Formato dei Simboli Nativi

I simboli nativi usano il formato `svg/categoria.nome.svg`:

```
svg/fruit.peach.svg          → pesca (frutto)
svg/activity.fishing.svg     → pesca (attività)
svg/obj.pen.svg              → penna (scrittura)
svg/animal.feather.svg       → penna (uccello)
svg/furniture.school_desk.svg → banco (scuola)
```

**Vantaggio:** Disambiguazione semantica deterministica tramite cues contestuali.

### 1.3 Formato dei Symbol_ref ARASAAC

I reference ARASAAC usano il formato `arasaac.picto.XXXX`:

```
arasaac.picto.27541  → da solo
arasaac.picto.2418   → assaggiare
arasaac.picto.11685  → sintesi vocale
arasaac.picto.25269  → telefono cellulare
arasaac.picto.4751   → carta di credito
```

**Utilizzo:** Concept che non hanno un simbolo nativo appropriato.

### 1.4 Concept Senza Simboli (19)

| ID | Label | Categoria | Significato |
|----|-------|-----------|-------------|
| `breed.dachshund` | bassotto | animali | razza di cane |
| `vehicle.electric_scooter` | monopattino elettrico | trasporti | veicolo urbano |
| `time.clock_time` | tempo | tempo | scorrere del tempo |
| `weather.conditions` | tempo | natura | meteo |
| `food.apple_juice` | succo di mela | alimenti | bevanda |
| `activity.leisure` | tempo libero | attività | hobby |
| `action.ask_help` | chiedere aiuto | azioni | richiesta |
| `activity.act` | fare | azioni | verbo generico |
| `action.cook` | cucinare | azioni | preparare cibo |
| `animal.part` | parte di animale | animali | corpo animale |
| `furniture.desk` | scrivania | oggetti | mobile ufficio |
| `obj.stationery` | cancelleria | oggetti | materiale scuola |
| `place.part` | parte di un luogo | luoghi | edificio |
| `place.service` | servizio | luoghi | ufficio pubblico |
| `concept.unknown` | simbolo mancante | sistema | placeholder |
| `action.understand` | comprendere | azioni | capire |
| `state.uncertain` | non so | emozioni | dubbio |
| `school.primary` | scuola primaria | scuola | elementari |
| `state.capable` | bravo | emozioni | competenza |

**Raccomandazione:** Importare da ARASAAC i simboli per questi 19 concept.

---

## 2. Materiali CAA (Board e Template)

### 2.1 Statistiche

- **File totali:** 535 (30 board + 505 template)
- **Simboli ARASAAC unici:** 811
- **Asset totali:** ~3.256 SVG

### 2.2 Formato dei Simboli

Tutti i simboli nei materiali usano il formato ARASAAC:

```
assets/symbols/arasaac.picto.26997.svg  → giorno
assets/symbols/arasaac.picto.25704.svg  → momento
assets/symbols/arasaac.picto.7129.svg   → pianificare
assets/symbols/arasaac.object.phone.svg → telefono
```

### 2.3 Overlap con il Dizionario

**Scoperta critica:** Solo **4 su 20** (20%) dei symbol_ref del dizionario sono effettivamente presenti nei materiali.

**Symbol_ref presenti:**
1. `arasaac.picto.2418` → assaggiare
2. `arasaac.picto.11685` → sintesi vocale
3. `arasaac.picto.25269` → telefono cellulare
4. `arasaac.picto.4751` → carta di credito

**Symbol_ref assenti (16):**
- `arasaac.picto.27541` → da solo
- `arasaac.picto.2680` → macchina fotografica
- `arasaac.picto.4625` → fare colazione
- `arasaac.picto.11750` → essere in grado di
- ... e altri 12

---

## 3. Gap Analysis

### 3.1 Simboli ARASAAC Non Referenziati (807)

**Problema:** 807 simboli ARASAAC sono usati nei materiali ma non hanno un entry nel dizionario core.

**Esempi:**
- `arasaac.picto.26997` → giorno (usato in "Agenda e tempo")
- `arasaac.picto.3082` → scuola (usato in "La mia giornata all'infanzia")
- `arasaac.action.play` → giocare
- `arasaac.food.apple` → mela

**Impatto:** Questi simboli non sono accessibili tramite la ricerca semantica del dizionario.

**Soluzione:** Espandere il dizionario con entry per i simboli ARASAAC più frequenti.

### 3.2 Symbol_ref Non Implementati (16)

**Problema:** 16 symbol_ref del dizionario non corrispondono a simboli reali nei materiali.

**Cause possibili:**
1. Simboli mai scaricati da ARASAAC
2. ID ARASAAC errati nel dizionario
3. Materiali creati prima dell'aggiunta del symbol_ref

**Soluzione:** Verificare gli ID ARASAAC e scaricare i simboli mancanti.

### 3.3 Concept Senza Simbolo (19)

**Problema:** 19 concept non hanno né simbolo nativo né symbol_ref.

**Priorità di intervento:**

| Priorità | Concept | Motivazione |
|----------|---------|-------------|
| **Alta** | `action.cook` (cucinare) | Attività quotidiana |
| **Alta** | `action.ask_help` (chiedere aiuto) | Comunicazione essenziale |
| **Alta** | `food.apple_juice` (succo di mela) | Alimento comune |
| **Media** | `vehicle.electric_scooter` (monopattino) | Trasporto urbano |
| **Media** | `time.clock_time` (tempo) | Concetto temporale |
| **Bassa** | `breed.dachshund` (bassotto) | Razza specifica |

---

## 4. Raccomandazioni

### 4.1 Breve Termine (1-2 settimane)

1. **Importare i 19 concept mancanti da ARASAAC**
   - Script batch per download
   - Aggiornamento dizionario v8
   - Test di simbolizzazione

2. **Verificare i 16 symbol_ref non implementati**
   - Controllo ID ARASAAC
   - Download simboli mancanti
   - Aggiornamento materiali

3. **Espandere il dizionario con i 50 simboli ARASAAC più frequenti**
   - Priorità: scuola, cibo, azioni quotidiane
   - Aggiunta entry con symbol_ref
   - Test di ricerca e affinità

### 4.2 Medio Termine (1-2 mesi)

1. **Catalogo ARASAAC locale indicizzato**
   - Download di ~500 simboli frequenti
   - Indicizzazione FTS5 nel database
   - Ricerca ibrida: nativo → ARASAAC locale → ARASAAC online

2. **Mappatura completa dizionario-materiali**
   - Ogni simbolo nei materiali ha un entry nel dizionario
   - Ogni entry nel dizionario è usata in almeno un materiale
   - Report di coerenza automatico

3. **Espansione del dizionario a 200+ concept**
   - Nuove categorie: professioni, emozioni complesse, tecnologia
   - Traduzione inglese/spagnolo
   - Integrazione con sintesi vocale

### 4.3 Lungo Termine (3-6 mesi)

1. **Simboli nativi per tutti i concept core**
   - Creazione di simboli SVG originali
   - Stile coerente con ARASAAC
   - Licenza CC BY-NC-SA 4.0

2. **Integrazione con repertori multipli**
   - ARASAAC (italiano)
   - PCS (inglese, opzionale)
   - Widgit (inglese, opzionale)
   - Ricerca cross-repertorio

3. **Generazione simbolica neurale**
   - Modello addestrato su simboli esistenti
   - Generazione per concept nuovi
   - Validazione umana

---

## 5. Piano di Implementazione

### Fase 1: Completamento Core (settimane 1-2)

```bash
# 1. Scarica i 19 simboli mancanti
python3 dev-library/scripts/import_arasaac.py \
  --symbols dev-library/symbols/missing-19.json \
  --output resources/symbols/arasaac/

# 2. Aggiorna il dizionario
#    - Aggiungi symbol_ref mancanti
#    - Correggi ID errati
#    - Versione → v8

# 3. Test
caa-cli call "symbolize" '{"text":"Cucino la pasta."}'
caa-cli call "affinities" '{"word":"cucinare"}'
```

### Fase 2: Espansione Lessicale (settimane 3-4)

```bash
# 1. Identifica i 50 simboli ARASAAC più frequenti
python3 scripts/analyze_symbol_frequency.py

# 2. Aggiungi entry nel dizionario
#    - 50 nuove entry con symbol_ref
#    - Cues contestuali
#    - Categorie lessicali

# 3. Test di ricerca
caa-cli call "symbol.search" '{"query":"scuola"}'
caa-cli call "symbol.search" '{"query":"mangiare"}'
```

### Fase 3: Catalogo Locale (settimane 5-8)

```bash
# 1. Download batch ARASAAC (~500 simboli)
python3 scripts/download_arasaac_batch.py --limit 500

# 2. Indicizzazione database
#    - Tabella arasaac_catalog
#    - FTS5 per ricerca full-text
#    - Metadati: licenza, autore, URL

# 3. Ricerca ibrida
#    - 1. Nativo (svg/...)
#    - 2. ARASAAC locale
#    - 3. ARASAAC online (opzionale)
```

---

## 6. Metriche di Qualità

### 6.1 Copertura Lessicale

| Metrica | Attuale | Target (1 mese) | Target (3 mesi) |
|---------|---------|-----------------|-----------------|
| Concept con simbolo | 88/127 (69%) | 108/127 (85%) | 127/127 (100%) |
| Symbol_ref implementati | 4/20 (20%) | 20/20 (100%) | 20/20 (100%) |
| Simboli ARASAAC catalogati | 0 | 100 | 500 |
| Concept senza simbolo | 19 | 0 | 0 |

### 6.2 Coerenza Dizionario-Materiali

| Metrica | Attuale | Target |
|---------|---------|--------|
| Symbol_ref nei materiali | 4/20 (20%) | 20/20 (100%) |
| Simboli materiali nel dizionario | 4/811 (<1%) | 200/811 (25%) |
| Materiali con tutti i simboli | ~95% | 100% |

### 6.3 Qualità della Simbolizzazione

| Metrica | Attuale | Target |
|---------|---------|--------|
| Accuratezza disambiguazione | ~85% | 95% |
| Tempo di risposta | <500ms | <200ms |
| Soddisfazione utente | N/D | >4/5 |

---

## 7. Conclusioni

**Problema principale:** Tre sistemi simbolici paralleli con copertura parziale creano incoerenze tra dizionario e materiali.

**Soluzione:** Unificare i sistemi attraverso:
1. Importazione dei 19 symbol_ref mancanti
2. Espansione del dizionario con i simboli ARASAAC più frequenti
3. Catalogo ARASAAC locale indicizzato
4. Mappatura completa dizionario-materiali

**Benefici attesi:**
- Copertura lessicale 100% (127/127 concept)
- Ricerca semantica coerente su tutti i materiali
- Simbolizzazione più accurata e contestuale
- Base per espansione multilingua

---

## Appendice A: Comandi di Verifica

```bash
# Verifica symbol_ref implementati
python3 scripts/check_symbol_coverage.py

# Test di simbolizzazione
caa-cli call "symbolize" '{"text":"Il gatto dorme sul divano."}'

# Ricerca simboli
caa-cli call "symbol.search" '{"query":"scuola"}'

# Affinità semantiche
caa-cli call "affinities" '{"word":"pesca"}'

# Verifica coerenza libreria
python3 dev-library/tests/verify_library.py
```

## Appendice B: File Correlati

- `resources/dictionaries/it-core.json` → Dizionario core (v7)
- `dev-library/dictionaries/it-core-v8.json` → Dizionario aggiornato (v8)
- `dev-library/symbols/arasaac-index.json` → Catalogo ARASAAC
- `docs/LIBRARY_COMPARATIVE_ANALYSIS.md` → Analisi comparativa libreria
- `docs/DEV_LIBRARY_PACKAGE.md` → Pacchetto sviluppo

---

*Report generato automaticamente il 2026-11-14 da CAA Writer Development Agent*
