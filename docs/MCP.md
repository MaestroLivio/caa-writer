# Server MCP: stato reale nel progetto

## Riepilogo

**Nessun server MCP è stato configurato o utilizzato** durante lo sviluppo di CAA Writer. Le 88
immagini del repertorio incluso sono state disegnate a mano come SVG originali, senza servizi di
generazione, senza download da repertori di terzi e senza chiamate di rete.

Questa pagina esiste perché il capitolato richiede di valutare i server MCP disponibili e di
distinguere chiaramente ciò che è stato usato da ciò che è solo previsto.

## Valutazione dei servizi indicati nel capitolato

| Servizio | Natura | Stato nel progetto | Note |
|---|---|---|---|
| SVGMaker MCP | server MCP per generare SVG | non configurato, non usato | richiede account e credenziali; le credenziali non vanno nel repository |
| CleanVector MCP | API/MCP con generazione di illustrazioni vettoriali | non configurato, non usato | servizio remoto a pagamento; incompatibile con il requisito offline per l'uso quotidiano |
| SVG.new MCP | servizio remoto di generazione SVG | non configurato, non usato | richiede rete e credenziali |
| Svg/icons MCP | catalogo di icone | non configurato, non usato | le icone di terzi hanno licenze da verificare singolarmente |

Non è stata presa per buona nessuna dichiarazione di funzionalità di questi servizi: non essendo
stati collegati, non è possibile affermare quali strumenti espongano realmente.

## Perché i pittogrammi sono originali

1. **Licenze pulite.** Disegnare i simboli in casa evita di incorporare repertori con clausole
   incompatibili (ARASAAC è CC BY-NC-SA e non può stare in una distribuzione commerciale).
2. **Qualità controllata.** Geometrie semplici, sfondo trasparente, leggibilità a 48 px, nessun
   testo incorporato, variante monocromatica ottenibile per filtro.
3. **Riproducibilità.** `scripts/generate_icons.py` e i file in `resources/symbol-packs/core/svg/`
   sono versionati: chiunque può ricostruire e verificare il risultato.

## Se in futuro si vorrà usare un MCP

La generazione via servizio è prevista come **funzione facoltativa dell'applicazione**, non come
dipendenza di sviluppo:

1. L'utente configura il servizio e fornisce la propria credenziale; la credenziale resta locale e
   non entra nel repository né nel frontend.
2. Viene inviato **solo il concetto astratto** e una descrizione generica, mai il documento.
3. L'SVG ricevuto passa la stessa validazione degli import locali: XML valido, `viewBox`, nessuno
   script, nessun evento inline, nessun riferimento esterno, dimensione massima.
4. Il simbolo nasce con stato `da verificare`; serve l'approvazione di un professionista prima
   dell'uso in materiali destinati a persone.
5. Dopo l'approvazione il simbolo è disponibile **offline** come qualsiasi altro.

Questa parte non è implementata: l'applicazione oggi importa SVG già presenti sul disco
(`import_svg_symbol`, `scripts/import_symbols.py`). Non esiste alcuna chiamata di rete per generare
simboli.

## Generazione delle icone dell'applicazione

Le icone (PNG, ICO, ICNS) sono prodotte localmente da `scripts/generate_icons.py`, che rasterizza
un disegno geometrico originale e scrive i contenitori PNG/ICO/ICNS senza dipendenze esterne. Non
sono state usate icone di terzi.
