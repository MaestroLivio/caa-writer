# Dipendenze e licenze

Aggiornato manualmente in questa fase; rigenerabile con `scripts/audit_licenses.sh`.

## Rust (src-tauri)
| Crate | Licenza | Ruolo |
|---|---|---|
| tauri 2 | MIT OR Apache-2.0 | Framework desktop |
| tauri-plugin-dialog 2 | MIT OR Apache-2.0 | Dialoghi file |
| rusqlite 0.37 (`bundled`) | MIT; SQLite: Public Domain | Database locale FTS5 |
| zip 2 | MIT | Contenitore `.caadoc`, DOCX |
| sha2 0.10 | MIT OR Apache-2.0 | Hash asset |
| roxmltree 0.20 | MIT OR Apache-2.0 | Validazione/sanificazione SVG |
| serde / serde_json | MIT OR Apache-2.0 | Serializzazione |
| unicode-normalization | MIT OR Apache-2.0 | Normalizzazione accenti |
| time 0.3 | MIT OR Apache-2.0 | Timestamp |
| thiserror 2 | MIT OR Apache-2.0 | Errori |
| image 0.25 (`avif-native` su macOS arm64) | MIT OR Apache-2.0 | Importazione immagini, AVIF |
| dav1d (libdav1d.7.dylib, incluso nel bundle macOS) | BSD-2-Clause | Decodifica AVIF; libreria di terzi copiata in `Contents/Frameworks` |
| tempfile (dev) | MIT OR Apache-2.0 | Test |

## Frontend
| Pacchetto | Licenza | Ruolo |
|---|---|---|
| svelte 5 | MIT | UI |
| vite 7 | MIT | Build |
| typescript 5 | Apache-2.0 | Tipi |
| prosemirror-* | MIT | Motore editor |
| @tauri-apps/api | MIT OR Apache-2.0 | Bridge |

## Servizi esterni opzionali (MCP, generazione SVG)
Nessuna credenziale è inclusa. Sono usati solo in fase di sviluppo o su
richiesta esplicita dell'utente, con connessione Internet.
Vedere `docs/MCP.md`: nella sessione corrente **nessun server MCP è stato
configurato o utilizzato**; i pittogrammi `core` sono originali, disegnati
a mano in SVG.
