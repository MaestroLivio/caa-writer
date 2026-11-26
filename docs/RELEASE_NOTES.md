# CAA Writer 0.16.3 — bundle macOS autosufficiente

Data: 26 novembre 2026

## Problema trovato

L'app macOS 0.16.2 non era completamente autonoma: `caa-writer`, `caa-cli` e
`caa-mcp` avevano un collegamento diretto a
`/opt/homebrew/opt/dav1d/lib/libdav1d.7.dylib`. La libreria era installata sul
computer usato per compilare, ma non dentro l'app. Su un altro Mac quel
percorso non esiste; inoltre il caricamento della dipendenza poteva provocare
una richiesta di accesso a un volume esterno.

## Correzione

- La libreria viene inclusa in `Contents/Frameworks/libdav1d.7.dylib`.
- Tutti e tre gli eseguibili la cercano con `@rpath` relativo al bundle.
- La release usa `scripts/sign_macos.sh` per includere la libreria, correggere
  i riferimenti e firmare nuovamente l'app.
- Lo script verifica che gli eseguibili non dipendano da `/opt/homebrew`,
  `/Volumes` o directory utente e che `libdav1d` sia davvero nel bundle.

## Verifiche

- Applicato lo script a una copia del bundle 0.16.2 scaricato dalla release:
  `codesign --verify --deep --strict` valido; `otool -L` mostra
  `@rpath/libdav1d.7.dylib` per `caa-writer`, `caa-cli` e `caa-mcp`; la libreria
  è presente in `Contents/Frameworks`.
- Il workflow release `v0.16.3` esegue lo stesso script sul nuovo bundle e
  interrompe la pubblicazione se trova percorsi Homebrew, `/Volumes` o utenti.

Il prompt di accesso al volume non è stato riprodotto manualmente in questa
sessione; la dipendenza esterna è stata però osservata direttamente con
`otool -L` nel bundle 0.16.2 e rimossa nella copia di verifica.
