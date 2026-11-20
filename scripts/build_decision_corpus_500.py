#!/usr/bin/env python3
"""Genera il corpus italiano di regressione e calibrazione (>500 casi).

Il corpus è sintetico e curato: le frasi sono scritte a mano per ogni
etichetta, non generate da un modello. Non contiene dati personali. Le
etichette attese derivano dal significato della frase, non dai punteggi del
modello. Resta comunque un corpus di regressione, non un benchmark clinico.
"""
from __future__ import annotations

import json
from pathlib import Path

INTENTS: dict[str, list[str]] = {
    "richiesta": [
        "Vorrei bere dell'acqua.",
        "Puoi darmi la merenda?",
        "Mi passi il libro, per favore?",
        "Vorrei andare in bagno.",
        "Mi presti la matita?",
        "Posso avere un bicchiere?",
        "Per favore, apri la finestra.",
        "Vorrei un pezzo di torta.",
        "Vorrei il mio zaino.",
        "Posso sedermi qui?",
    ],
    "rifiuto": [
        "Non voglio andare fuori.",
        "No, grazie.",
        "Non mi piace questo.",
        "Lasciami stare.",
        "Non lo voglio.",
        "Basta, non ne voglio più.",
        "Non voglio mangiare.",
        "Non ci vado.",
        "No, adesso no.",
        "Non mi va.",
    ],
    "domanda": [
        "Dove sono i miei scarponi?",
        "Quando arriva il pullman?",
        "Perché piove?",
        "Chi ha preso la mia penna?",
        "Come si chiama questo animale?",
        "Quanto costa?",
        "Posso uscire?",
        "Dov'è la biblioteca?",
        "Che ora è?",
        "Hai visto il mio telefono?",
    ],
    "saluto": [
        "Ciao, come stai?",
        "Buongiorno a tutti.",
        "Buonasera!",
        "Ciao, ci vediamo domani.",
        "Salve!",
        "A domani!",
        "Buonanotte.",
        "Ciao a tutti!",
        "Benvenuto!",
        "Piacere di conoscerti.",
    ],
    "affermazione": [
        "Oggi è lunedì.",
        "La palla è rossa.",
        "Il gatto dorme sul divano.",
        "Ho due sorelle.",
        "La scuola inizia alle otto.",
        "Questo è il mio quaderno.",
        "Il treno è in ritardo.",
        "Mi chiamo Luca.",
        "Abito a Roma.",
        "La porta è aperta.",
    ],
    "bisogno": [
        "Ho sete.",
        "Ho fame.",
        "Ho sonno.",
        "Devo andare in bagno.",
        "Ho freddo.",
        "Ho caldo.",
        "Mi serve una pausa.",
        "Ho bisogno di sedermi.",
        "Sono stanco.",
        "Mi fa male la testa.",
    ],
    "preferenza": [
        "Preferisco il gelato al cioccolato.",
        "Mi piace di più questo.",
        "Scelgo la maglietta blu.",
        "Vorrei il succo, non l'acqua.",
        "Preferisco giocare fuori.",
        "Mi piace la pizza margherita.",
        "Meglio il libro che il fumetto.",
        "Scelgo questo colore.",
        "Preferisco stare con mia sorella.",
        "Mi piace più il cane del gatto.",
    ],
    "ringraziamento": [
        "Grazie mille per l'aiuto.",
        "Grazie!",
        "Ti ringrazio molto.",
        "Grazie per il regalo.",
        "Sei stato gentile, grazie.",
        "Grazie della pazienza.",
        "Grazie per avermi aspettato.",
        "Molte grazie.",
        "Grazie di cuore.",
        "Ti sono grato.",
    ],
    "scusa": [
        "Scusa, non l'ho fatto apposta.",
        "Mi dispiace.",
        "Chiedo scusa.",
        "Scusami per il ritardo.",
        "Non volevo disturbare, scusa.",
        "Mi dispiace molto.",
        "Ti chiedo perdono.",
        "Scusa il rumore.",
        "Mi scuso per l'errore.",
        "Perdonami.",
    ],
    "richiesta_aiuto": [
        "Mi aiuti ad aprire?",
        "Ho bisogno di aiuto.",
        "Puoi aiutarmi con il compito?",
        "Non riesco, mi dai una mano?",
        "Chiamo la maestra?",
        "Mi accompagni in bagno?",
        "Aiutami a portare la borsa.",
        "Puoi spiegarmi questa parola?",
        "Ho bisogno di assistenza.",
        "Mi aiuti a trovare il libro?",
    ],
    "commento": [
        "Questo film è bellissimo.",
        "La minestra è buona.",
        "Che bella giornata!",
        "Il libro è noioso.",
        "Questo gioco è divertente.",
        "La tua casa è grande.",
        "Il cane è molto simpatico.",
        "Questa canzone mi piace.",
        "Il disegno è colorato.",
        "La lezione è stata interessante.",
    ],
    "espressione_emotiva": [
        "Sono felicissimo!",
        "Che paura!",
        "Sono così triste.",
        "Mi sento arrabbiato.",
        "Sono sorpreso!",
        "Che gioia!",
        "Sono preoccupato.",
        "Mi sento solo.",
        "Sono orgoglioso di te.",
        "Che rabbia!",
    ],
}

EMOTIONS: dict[str, list[str]] = {
    "gioia": [
        "Sono felice.",
        "Che bello!",
        "Sono contentissimo.",
        "Mi sento allegro.",
        "Grande gioia!",
        "Rido di gusto.",
        "Sono soddisfatto.",
        "Che meraviglia!",
        "Sono entusiasta.",
        "Mi sento bene.",
    ],
    "tristezza": [
        "Mi sento triste.",
        "Piango.",
        "Sono giù.",
        "Ho il magone.",
        "Mi manca mia nonna.",
        "Sono abbattuto.",
        "Mi sento solo.",
        "Che tristezza.",
        "Sono deluso.",
        "Mi dispiace tanto.",
    ],
    "rabbia": [
        "Sono arrabbiato.",
        "Mi fa infuriare.",
        "Che rabbia!",
        "Non sopporto più.",
        "Sono furioso.",
        "Mi hai fatto arrabbiare.",
        "Basta!",
        "Sono irritato.",
        "Mi sale la rabbia.",
        "Sono indignato.",
    ],
    "paura": [
        "Ho paura del buio.",
        "Mi spaventa.",
        "Ho paura.",
        "Che spavento!",
        "Sono terrorizzato.",
        "Temo il temporale.",
        "Mi fa paura il cane.",
        "Ho il batticuore.",
        "Sono impaurito.",
        "Non mi sento al sicuro.",
    ],
    "sorpresa": [
        "Non me lo aspettavo!",
        "Che sorpresa!",
        "Sono stupito.",
        "Davvero?",
        "Non ci credo!",
        "Wow!",
        "Sono meravigliato.",
        "Che colpo!",
        "Non l'avrei mai detto.",
        "Sono sbalordito.",
    ],
    "preoccupazione": [
        "Sono preoccupato per domani.",
        "Temo che sia tardi.",
        "Sono in ansia.",
        "Mi preoccupa la visita.",
        "Non so come andrà.",
        "Sono inquieto.",
        "Ho un pensiero fisso.",
        "Temo di sbagliare.",
        "Sono agitato.",
        "Mi sento in apprensione.",
    ],
    "frustrazione": [
        "Non ci riesco mai!",
        "Che fastidio!",
        "Sono frustrato.",
        "Non funziona niente.",
        "Mi arrendo.",
        "È impossibile!",
        "Non ne posso più.",
        "Mi sento bloccato.",
        "Che rabbia non riuscire.",
        "Non riesco a finire.",
    ],
    "soddisfazione": [
        "Sono soddisfatto del lavoro.",
        "Ce l'ho fatta!",
        "Sono orgoglioso.",
        "Bel risultato!",
        "Sono contento del mio disegno.",
        "Ho finito tutto.",
        "Sono fiero di me.",
        "Ottimo lavoro!",
        "Mi sento realizzato.",
        "Sono appagato.",
    ],
    "incertezza": [
        "Non sono sicuro.",
        "Forse.",
        "Non lo so.",
        "Non saprei.",
        "Ho dei dubbi.",
        "Può essere.",
        "Non ne sono certo.",
        "Devo pensarci.",
        "Non ho capito bene.",
        "Sono indeciso.",
    ],
}

CATEGORIES: dict[str, list[str]] = {
    "persone": ["La maestra spiega.", "Il bambino gioca.", "Mia madre cucina.", "Il dottore visita.", "Luca corre."],
    "animali": ["Il cane corre.", "Il gatto dorme.", "L'uccello vola.", "Il pesce nuota.", "La mucca mangia."],
    "alimenti": ["La mela è rossa.", "Mangio il pane.", "La pasta è pronta.", "Il formaggio è buono.", "La banana è gialla."],
    "bevande": ["Bevo l'acqua.", "Il succo è freddo.", "Prendo un tè.", "Il latte è caldo.", "Vorrei una spremuta."],
    "oggetti": ["La sedia è di legno.", "Il tavolo è grande.", "La chiave è sul tavolo.", "Il telefono suona.", "La lampada è accesa."],
    "luoghi": ["Vado al parco.", "La scuola è vicina.", "Il negozio apre.", "Siamo in cucina.", "La biblioteca è chiusa."],
    "azioni": ["Il bambino corre.", "La ragazza salta.", "Mangio la mela.", "Scrivo una frase.", "Apro la porta."],
    "emozioni": ["Provo gioia.", "Sento tristezza.", "Ho paura.", "Sono arrabbiato.", "Mi sento sereno."],
    "bisogni": ["Ho fame.", "Ho sete.", "Devo dormire.", "Ho bisogno di aiuto.", "Devo andare in bagno."],
    "abbigliamento": ["Indosso la maglietta.", "I pantaloni sono blu.", "Metto le scarpe.", "Il cappotto è caldo.", "I calzini sono sporchi."],
    "materiale_scolastico": ["Uso la matita.", "Il quaderno è nuovo.", "La gomma cancella.", "Il righello è lungo.", "Lo zaino è pesante."],
    "trasporti": ["Il treno parte.", "L'autobus arriva.", "La macchina è rossa.", "La bici è nuova.", "L'aereo vola."],
    "natura": ["Il sole splende.", "Piove forte.", "L'albero è alto.", "Il mare è calmo.", "La montagna è bianca."],
    "tempo": ["Domani è lunedì.", "Oggi piove.", "Ieri era domenica.", "La settimana finisce.", "Sono le tre."],
    "corpo": ["Mi lavo le mani.", "Ho mal di testa.", "Il braccio fa male.", "Mi tocco il naso.", "Le gambe corrono."],
    "igiene": ["Mi lavo i denti.", "Faccio la doccia.", "Uso il sapone.", "Mi pettino.", "Mi asciugo le mani."],
    "salute": ["Il dottore mi visita.", "Prendo la medicina.", "Ho la febbre.", "Vado in farmacia.", "Mi sento meglio."],
    "gioco": ["Giochiamo a palla.", "Costruisco una torre.", "Facciamo un puzzle.", "Gioco a nascondino.", "Uso i colori."],
    "relazioni_sociali": ["Abbraccio mia sorella.", "Gioco con mio amico.", "Parliamo insieme.", "Aiuto il compagno.", "Saluto la vicina."],
}

BOARDS: dict[str, list[str]] = {
    "bisogni": ["Ho sete.", "Ho fame.", "Devo andare in bagno.", "Ho sonno.", "Mi serve aiuto.", "Ho freddo."],
    "azioni": ["Corro.", "Salto.", "Mangio.", "Scrivo.", "Apro la porta.", "Disegno."],
    "cibo": ["Mangio la pasta.", "La mela è buona.", "Vorrei il pane.", "Il formaggio è fresco.", "La minestra è calda.", "Prendo una banana."],
    "persone": ["La maestra parla.", "Mia madre arriva.", "Il bambino gioca.", "Il dottore entra.", "Luca sorride.", "La nonna cucina."],
    "luoghi": ["Vado a scuola.", "Siamo al parco.", "Il negozio è aperto.", "Entro in casa.", "La biblioteca è qui.", "Andiamo al mare."],
    "emozioni": ["Sono felice.", "Mi sento triste.", "Ho paura.", "Sono arrabbiato.", "Che sorpresa!", "Sono tranquillo."],
    "attivita": ["Gioco a palla.", "Leggo un libro.", "Disegno.", "Faccio i compiti.", "Ascolto la musica.", "Guardo la TV."],
    "routine": ["Mi alzo.", "Faccio colazione.", "Mi lavo i denti.", "Vado a scuola.", "Ceno.", "Vado a letto."],
    "corpo": ["Mi fa male la testa.", "Ho mal di pancia.", "Mi lavo le mani.", "Il braccio è rotto.", "Mi tocco il naso.", "Le gambe sono stanche."],
    "scuola": ["La maestra spiega.", "Apro il quaderno.", "Scrivo la data.", "Faccio l'intervallo.", "La campanella suona.", "Studio matematica."],
}

OBJECTIVES: dict[str, list[str]] = {
    "comprensione": ["Ascolta la storia e rispondi.", "Leggi e scegli l'immagine giusta.", "Comprendi la consegna.", "Indica cosa succede."],
    "denominazione": ["Come si chiama questo animale?", "Dai il nome all'oggetto.", "Nomina le parti del corpo.", "Dimmi che cos'è."],
    "associazione": ["Unisci la parola all'immagine.", "Trova la coppia giusta.", "Associa il suono al simbolo.", "Collega ogni animale al suo verso."],
    "categorizzazione": ["Metti insieme i frutti.", "Separa gli animali dagli oggetti.", "Raggruppa per colore.", "Scegli cosa non appartiene."],
    "sequenziamento": ["Metti in ordine le immagini.", "Racconta prima e dopo.", "Ordina i passaggi della ricetta.", "Completa la sequenza."],
    "consapevolezza_fonologica": ["Indica l'immagine che inizia con MA.", "Trova la parola che fa rima.", "Conta le sillabe.", "Ascolta il suono iniziale."],
    "lettura": ["Leggi la frase.", "Trova la parola scritta.", "Leggi ad alta voce.", "Completa la parola mancante."],
    "scrittura": ["Scrivi il nome.", "Copia la frase.", "Completa le lettere.", "Scrivi una didascalia."],
    "calcolo": ["Quanto fa 2 più 3?", "Conta gli oggetti.", "Scegli il numero giusto.", "Risolvi l'addizione."],
    "confronto": ["Qual è più grande?", "Confronta i due disegni.", "Trova la differenza.", "Quale pesa di più?"],
    "ordinamento": ["Metti dal più piccolo al più grande.", "Ordina i numeri.", "Disponi in fila.", "Metti in ordine di altezza."],
    "classificazione_logica": ["Trova l'intruso.", "Scegli cosa non c'entra.", "Completa la categoria.", "Quale non appartiene?"],
    "comprensione_temporale": ["Cosa succede prima?", "Metti in ordine mattina e sera.", "Indica ieri e oggi.", "Racconta la giornata."],
    "comprensione_spaziale": ["Metti sopra il tavolo.", "Trova a destra.", "Posiziona dentro la scatola.", "Indica davanti e dietro."],
}

EXERCISES: dict[str, list[str]] = {
    "scelta_multipla": ["Scegli la risposta giusta.", "Indica l'opzione corretta.", "Seleziona una sola risposta."],
    "abbinamento": ["Unisci parola e immagine.", "Abbina ogni animale al suo verso.", "Collega le coppie."],
    "ordinamento": ["Metti in ordine le immagini.", "Ordina i numeri.", "Disponi i passaggi in sequenza."],
    "completamento": ["Completa la frase.", "Scrivi la parola mancante.", "Riempi gli spazi."],
    "classificazione": ["Metti insieme i frutti.", "Separa gli animali.", "Raggruppa per categoria."],
    "sequenza_temporale": ["Cosa viene prima?", "Ordina la giornata.", "Metti in ordine gli eventi."],
    "associazione_parola_immagine": ["Unisci la parola al disegno.", "Trova l'immagine della parola.", "Abbina il nome all'oggetto."],
    "associazione_suono_immagine": ["Ascolta e scegli l'immagine.", "Unisci il suono al simbolo.", "Abbina il verso all'animale."],
    "composizione_frase": ["Costruisci una frase.", "Metti le parole in ordine.", "Componi la frase con i simboli."],
}

SOCIAL: dict[str, list[str]] = {
    "aspettare_il_turno": ["Devo aspettare il mio turno.", "Quando parla un compagno ascolto."],
    "chiedere_aiuto": ["Se non riesco, chiedo aiuto.", "Chiedo aiuto alla maestra."],
    "affrontare_cambiamenti": ["Oggi c'è una supplente.", "La routine cambia e mi preparo."],
    "gestire_conflitti": ["Se litigo, parlo con calma.", "Chiedo scusa e troviamo un accordo."],
    "prepararsi_a_una_visita": ["Domani vado dal dottore.", "Mi preparo per la visita."],
    "rispettare_regole_condivise": ["In classe alzo la mano.", "Rispetto le regole del gioco."],
    "esprimere_bisogni": ["Dico che ho bisogno di una pausa.", "Spiego cosa mi serve."],
    "comunicare_emozioni": ["Dico che sono triste.", "Esprimo la mia gioia."],
    "partecipare_ad_attivita_collettive": ["Gioco con il gruppo.", "Partecipo all'attività della classe."],
}

ACTIONS: dict[str, list[str]] = {
    "append": ["Questo pulsante aggiunge la parola alla frase.", "Aggiungi il simbolo al messaggio."],
    "speak": ["Questo pulsante legge la frase ad alta voce.", "Pronuncia il messaggio composto."],
    "toggle": ["Questo pulsante seleziona la cella per un esercizio.", "Attiva la scelta della cella."],
    "navigate": ["Questo pulsante apre la pagina degli animali.", "Vai alla pagina della merenda."],
    "back": ["Questo pulsante torna indietro.", "Torna alla pagina precedente."],
    "home": ["Questo pulsante va alla pagina iniziale.", "Apri la home della tabella."],
    "clear": ["Questo pulsante svuota la composizione.", "Cancella tutto il messaggio."],
}

LAYOUTS: dict[str, list[str]] = {
    "tabella_comunicativa": ["Voglio una tabella con i bisogni.", "Crea una tabella comunicativa con 20 celle."],
    "sequenza_temporale": ["Voglio una sequenza prima e dopo.", "Crea una sequenza temporale."],
    "agenda_visiva": ["Voglio un'agenda della giornata.", "Crea un'agenda visiva settimanale."],
    "scheda_esercizi": ["Voglio una scheda di esercizi.", "Crea una scheda con gli esercizi."],
    "storia_caa": ["Voglio una storia con i pittogrammi.", "Crea una storia CAA."],
    "storia_sociale": ["Voglio una storia sociale.", "Crea una storia sociale sulla visita."],
    "tabella_scelta": ["Voglio una tabella per scegliere.", "Crea una tabella di scelta."],
    "documento_narrativo": ["Voglio un documento narrativo.", "Crea un racconto."],
}

# ---------------------------------------------------------------- extra
# Seconda ondata di casi curati: frasi nuove, stesse etichette. Servono ad
# ampliare la base di calibrazione, non a modificare il modello.

EXTRA_INTENTS: dict[str, list[str]] = {
    "richiesta": ["Mi porti un bicchiere d'acqua?", "Vorrei un foglio per disegnare."],
    "rifiuto": ["Non mi interessa.", "No, non oggi."],
    "domanda": ["Di chi è questa giacca?", "Come arrivo alla stazione?"],
    "saluto": ["Ciao, buona giornata!", "Salve a tutti!"],
    "affermazione": ["Il cielo è azzurro.", "Oggi abbiamo educazione fisica."],
    "bisogno": ["Ho bisogno del bagno.", "Mi serve un fazzoletto."],
    "preferenza": ["Preferisco la musica al silenzio.", "Scelgo il gioco dei puzzle."],
    "ringraziamento": ["Grazie per la merenda.", "Ti ringrazio di cuore."],
    "scusa": ["Scusa per il disturbo.", "Mi dispiace di aver gridato."],
    "richiesta_aiuto": ["Puoi aiutarmi a leggere?", "Ho bisogno di una mano con lo zaino."],
    "commento": ["Che bella sorpresa!", "Questa storia è avvincente."],
    "espressione_emotiva": ["Sono al settimo cielo!", "Mi sento giù di morale."],
}

EXTRA_EMOTIONS: dict[str, list[str]] = {
    "gioia": ["Sono raggiante.", "Che felicità!"],
    "tristezza": ["Ho voglia di piangere.", "Mi sento a pezzi."],
    "rabbia": ["Sono nero di rabbia.", "Mi hai fatto perdere la pazienza."],
    "paura": ["Ho i brividi.", "Temo il buio."],
    "sorpresa": ["Che incredibile sorpresa!", "Sono rimasto di stucco."],
    "preoccupazione": ["Sono in pensiero per te.", "Non riesco a stare tranquillo."],
    "frustrazione": ["Non riesco a risolvere niente.", "Che sfortuna!"],
    "soddisfazione": ["Sono molto contento del risultato.", "Sono fiero di ciò che ho fatto."],
    "incertezza": ["Non sono convinto.", "Forse sì, forse no."],
}

EXTRA_CATEGORIES: dict[str, list[str]] = {
    "persone": ["Il vigile dirige il traffico."],
    "animali": ["La farfalla vola."],
    "alimenti": ["Il pomodoro è maturo."],
    "bevande": ["Il caffè è amaro."],
    "oggetti": ["La forchetta è sul tavolo."],
    "luoghi": ["Il museo è aperto."],
    "azioni": ["La bambina canta."],
    "emozioni": ["Sento allegria."],
    "bisogni": ["Ho bisogno di riposare."],
    "abbigliamento": ["Indosso il cappello."],
    "materiale_scolastico": ["Il pennarello è rosso."],
    "trasporti": ["Il tram si ferma."],
    "natura": ["Il fiume scorre."],
    "tempo": ["La notte è lunga."],
    "corpo": ["Mi fa male il ginocchio."],
    "igiene": ["Mi lavo la faccia."],
    "salute": ["La ferita guarisce."],
    "gioco": ["Facciamo una gara."],
    "relazioni_sociali": ["Saluto il mio vicino."],
}

EXTRA_BOARDS: dict[str, list[str]] = {
    "bisogni": ["Ho bisogno di acqua."],
    "azioni": ["Cammino."],
    "cibo": ["Mangio la minestra."],
    "persone": ["La nonna arriva."],
    "luoghi": ["Vado al supermercato."],
    "emozioni": ["Sono sereno."],
    "attivita": ["Faccio una passeggiata."],
    "routine": ["Mi vesto."],
    "corpo": ["Mi fa male la schiena."],
    "scuola": ["Leggo il libro."],
}

EXTRA_OBJECTIVES: dict[str, list[str]] = {
    "comprensione": ["Ascolta e indica la risposta."],
    "denominazione": ["Nomina gli oggetti sul tavolo."],
    "associazione": ["Collega il numero alla quantità."],
    "categorizzazione": ["Metti insieme gli animali."],
    "sequenziamento": ["Ordina le azioni della mattina."],
    "consapevolezza_fonologica": ["Trova la parola che inizia con S."],
    "lettura": ["Leggi la parola scritta."],
    "scrittura": ["Scrivi il tuo nome."],
    "calcolo": ["Conta fino a dieci."],
    "confronto": ["Qual è più lungo?"],
    "ordinamento": ["Metti in ordine crescente."],
    "classificazione_logica": ["Scegli l'elemento diverso."],
    "comprensione_temporale": ["Cosa fai dopo pranzo?"],
    "comprensione_spaziale": ["Metti il libro sotto il tavolo."],
}

EXTRA_EXERCISES: dict[str, list[str]] = {
    "scelta_multipla": ["Scegli fra tre immagini."],
    "abbinamento": ["Abbina ogni numero alla sua quantità."],
    "ordinamento": ["Metti in ordine dal più corto al più lungo."],
    "completamento": ["Completa la parola con la sillaba giusta."],
    "classificazione": ["Dividi gli oggetti in due gruppi."],
    "sequenza_temporale": ["Ordina le immagini della giornata."],
    "associazione_parola_immagine": ["Collega la parola alla figura."],
    "associazione_suono_immagine": ["Scegli l'immagine dopo aver ascoltato."],
    "composizione_frase": ["Metti le parole in ordine per formare la frase."],
}

EXTRA_SOCIAL: dict[str, list[str]] = {
    "aspettare_il_turno": ["Aspetto che finisca il mio compagno."],
    "chiedere_aiuto": ["Chiedo aiuto quando non capisco."],
    "affrontare_cambiamenti": ["Accetto un cambiamento di programma."],
    "gestire_conflitti": ["Se sono arrabbiato, respiro e parlo."],
    "prepararsi_a_una_visita": ["Metto lo zaino per la visita."],
    "rispettare_regole_condivise": ["Aspetto il mio turno in fila."],
    "esprimere_bisogni": ["Chiedo una pausa quando sono stanco."],
    "comunicare_emozioni": ["Dico come mi sento."],
    "partecipare_ad_attivita_collettive": ["Collaboro con il gruppo."],
}

EXTRA_ACTIONS: dict[str, list[str]] = {
    "append": ["Aggiungi la parola scelta alla frase."],
    "speak": ["Fai ascoltare il messaggio."],
    "toggle": ["Seleziona questa cella."],
    "navigate": ["Portami alla pagina della famiglia."],
    "back": ["Ritorna alla schermata precedente."],
    "home": ["Torna all'inizio della tabella."],
    "clear": ["Azzera il messaggio."],
}

EXTRA_LAYOUTS: dict[str, list[str]] = {
    "tabella_comunicativa": ["Crea una tabella per comunicare a scuola."],
    "sequenza_temporale": ["Prepara una sequenza prima-dopo."],
    "agenda_visiva": ["Crea l'agenda della mattina."],
    "scheda_esercizi": ["Prepara una scheda con gli esercizi di lettura."],
    "storia_caa": ["Crea una storia con i simboli."],
    "storia_sociale": ["Prepara una storia sociale sulla fila."],
    "tabella_scelta": ["Crea una tabella per scegliere l'attività."],
    "documento_narrativo": ["Scrivi un racconto illustrato."],
}

for label, sentences in EXTRA_INTENTS.items():
    INTENTS[label].extend(sentences)
for label, sentences in EXTRA_EMOTIONS.items():
    EMOTIONS[label].extend(sentences)
for label, sentences in EXTRA_CATEGORIES.items():
    CATEGORIES[label].extend(sentences)
for label, sentences in EXTRA_BOARDS.items():
    BOARDS[label].extend(sentences)
for label, sentences in EXTRA_OBJECTIVES.items():
    OBJECTIVES[label].extend(sentences)
for label, sentences in EXTRA_EXERCISES.items():
    EXERCISES[label].extend(sentences)
for label, sentences in EXTRA_SOCIAL.items():
    SOCIAL[label].extend(sentences)
for label, sentences in EXTRA_ACTIONS.items():
    ACTIONS[label].extend(sentences)
for label, sentences in EXTRA_LAYOUTS.items():
    LAYOUTS[label].extend(sentences)

DISAMBIGUATION = [
    ("pesca-frutto", "Ho mangiato una pesca.", "pesca",
     [("frutto", "frutto (alimento)"), ("attivita", "attività di pesca (catturare pesci)")], "frutto"),
    ("pesca-attivita", "Vado a pesca.", "pesca",
     [("frutto", "frutto (alimento)"), ("attivita", "attività di pesca (catturare pesci)")], "attivita"),
    ("amo-amore", "Amo la pizza.", "Amo",
     [("amore", "amore (sentimento)"), ("amo", "amo da pesca (oggetto per pescare)")], "amore"),
    ("amo-oggetto", "Ho comprato un amo.", "amo",
     [("amore", "amore (sentimento)"), ("amo", "amo da pesca (oggetto per pescare)")], "amo"),
    ("riso-cibo", "Mangio il riso.", "riso",
     [("alimento", "riso (alimento)"), ("ridere", "riso (risata)")], "alimento"),
    ("riso-risata", "Il suo riso è contagioso.", "riso",
     [("alimento", "riso (alimento)"), ("ridere", "riso (risata)")], "ridere"),
    ("porta-stanza", "Chiudi la porta.", "porta",
     [("porta_casa", "porta (di una stanza)"), ("portare", "porta (verbo portare)")], "porta_casa"),
    ("porta-verbo", "La bambina porta un libro.", "porta",
     [("porta_casa", "porta (di una stanza)"), ("portare", "porta (verbo portare)")], "portare"),
    ("piano-strumento", "Suono il piano.", "piano",
     [("pianoforte", "pianoforte (strumento musicale)"), ("avverbio", "piano (lentamente)")], "pianoforte"),
    ("piano-avverbio", "Cammina piano.", "piano",
     [("pianoforte", "pianoforte (strumento musicale)"), ("avverbio", "piano (lentamente)")], "avverbio"),
    ("penna-scrivere", "Scrivo con la penna.", "penna",
     [("penna_scrivere", "penna (per scrivere)"), ("piuma", "penna (piuma di uccello)")], "penna_scrivere"),
    ("penna-piuma", "L'uccello ha perso una penna.", "penna",
     [("penna_scrivere", "penna (per scrivere)"), ("piuma", "penna (piuma di uccello)")], "piuma"),
    ("pesca-matura", "La pesca matura è dolce.", "pesca",
     [("frutto", "frutto (alimento)"), ("attivita", "attività di pesca (catturare pesci)")], "frutto"),
    ("pesca-mare", "Andiamo al mare per la pesca.", "pesca",
     [("frutto", "frutto (alimento)"), ("attivita", "attività di pesca (catturare pesci)")], "attivita"),
    ("amo-ti", "Ti amo tanto.", "amo",
     [("amore", "amore (sentimento)"), ("amo", "amo da pesca (oggetto per pescare)")], "amore"),
    ("amo-lenza", "L'amo è attaccato alla lenza.", "amo",
     [("amore", "amore (sentimento)"), ("amo", "amo da pesca (oggetto per pescare)")], "amo"),
    ("riso-cucina", "Il riso è pronto in cucina.", "riso",
     [("alimento", "riso (alimento)"), ("ridere", "riso (risata)")], "alimento"),
    ("riso-contagioso", "Il riso di Marco è contagioso.", "riso",
     [("alimento", "riso (alimento)"), ("ridere", "riso (risata)")], "ridere"),
    ("porta-apri", "Apri la porta, per favore.", "porta",
     [("porta_casa", "porta (di una stanza)"), ("portare", "porta (verbo portare)")], "porta_casa"),
    ("porta-borsa", "Marco porta la borsa.", "porta",
     [("porta_casa", "porta (di una stanza)"), ("portare", "porta (verbo portare)")], "portare"),
    ("penna-compito", "Uso la penna per il compito.", "penna",
     [("penna_scrivere", "penna (per scrivere)"), ("piuma", "penna (piuma di uccello)")], "penna_scrivere"),
    ("penna-bianca", "L'uccello ha una penna bianca.", "penna",
     [("penna_scrivere", "penna (per scrivere)"), ("piuma", "penna (piuma di uccello)")], "piuma"),
    ("piano-giorno", "Suono il piano ogni giorno.", "piano",
     [("pianoforte", "pianoforte (strumento musicale)"), ("avverbio", "piano (lentamente)")], "pianoforte"),
    ("pesca-estate", "La pesca è un frutto estivo.", "pesca",
     [("frutto", "frutto (alimento)"), ("attivita", "attività di pesca (catturare pesci)")], "frutto"),
    ("pesca-pescatore", "Il pescatore torna dalla pesca.", "pesca",
     [("frutto", "frutto (alimento)"), ("attivita", "attività di pesca (catturare pesci)")], "attivita"),
    ("amo-lenza-2", "L'amo è legato alla lenza.", "amo",
     [("amore", "amore (sentimento)"), ("amo", "amo da pesca (oggetto per pescare)")], "amo"),
    ("riso-piatto", "Il riso bianco è nel piatto.", "riso",
     [("alimento", "riso (alimento)"), ("ridere", "riso (risata)")], "alimento"),
    ("riso-pubblico", "Il riso del pubblico era contagioso.", "riso",
     [("alimento", "riso (alimento)"), ("ridere", "riso (risata)")], "ridere"),
    ("porta-garage", "Chiudi la porta del garage.", "porta",
     [("porta_casa", "porta (di una stanza)"), ("portare", "porta (verbo portare)")], "porta_casa"),
    ("porta-valigia", "Mio padre porta la valigia.", "porta",
     [("porta_casa", "porta (di una stanza)"), ("portare", "porta (verbo portare)")], "portare"),
    ("penna-quaderno", "Uso la penna per il quaderno.", "penna",
     [("penna_scrivere", "penna (per scrivere)"), ("piuma", "penna (piuma di uccello)")], "penna_scrivere"),
    ("penna-caduta", "La penna dell'uccello è caduta.", "penna",
     [("penna_scrivere", "penna (per scrivere)"), ("piuma", "penna (piuma di uccello)")], "piuma"),
    ("pesca-zio", "Vado a pesca con mio zio.", "pesca",
     [("frutto", "frutto (alimento)"), ("attivita", "attività di pesca (catturare pesci)")], "attivita"),
    ("pesca-dolce", "La pesca è dolce.", "pesca",
     [("frutto", "frutto (alimento)"), ("attivita", "attività di pesca (catturare pesci)")], "frutto"),
    ("amo-nuovo", "Ho comprato un amo nuovo.", "amo",
     [("amore", "amore (sentimento)"), ("amo", "amo da pesca (oggetto per pescare)")], "amo"),
]


def span(text: str, word: str) -> dict:
    index = text.find(word)
    if index < 0:
        raise ValueError(f"parola {word!r} assente in {text!r}")
    return {"text": word, "start": index, "end": index + len(word)}


def add_cases(cases: list[dict], task: str, label: str, sentences: list[str]) -> None:
    for index, text in enumerate(sentences, start=1):
        cases.append({
            "id": f"{task}-{label}-{index}",
            "task": task,
            "text": text,
            "expected": label,
        })


def main() -> None:
    root = Path(__file__).resolve().parent.parent
    cases: list[dict] = []

    for label, sentences in INTENTS.items():
        add_cases(cases, "communication_intent", label, sentences)
    for label, sentences in EMOTIONS.items():
        add_cases(cases, "expressed_emotion", label, sentences)
    for label, sentences in CATEGORIES.items():
        add_cases(cases, "semantic_category", label, sentences)
    for label, sentences in BOARDS.items():
        add_cases(cases, "board_category", label, sentences)
    for label, sentences in OBJECTIVES.items():
        add_cases(cases, "learning_objective", label, sentences)
    for label, sentences in EXERCISES.items():
        add_cases(cases, "exercise_type", label, sentences)
    for label, sentences in SOCIAL.items():
        add_cases(cases, "social_story_topic", label, sentences)
    for label, sentences in ACTIONS.items():
        add_cases(cases, "interactive_action", label, sentences)
    for label, sentences in LAYOUTS.items():
        add_cases(cases, "layout_template", label, sentences)
    for case_id, text, word, candidates, expected in DISAMBIGUATION:
        cases.append({
            "id": f"disambiguation-{case_id}",
            "task": "pictogram_disambiguation",
            "text": text,
            "target": span(text, word),
            "candidates": [{"id": cid, "label": label} for cid, label in candidates],
            "expected": expected,
        })

    # Split deterministico per calibrazione/validazione.
    for case in cases:
        bucket = sum(ord(ch) for ch in case["id"]) % 10
        case["split"] = "calibration" if bucket < 7 else "validation"

    payload = {
        "version": "1",
        "language": "it",
        "description": (
            "Corpus sintetico curato per regressione e calibrazione. Le frasi "
            "sono scritte a mano; le etichette derivano dal significato, non dal "
            "modello. Split 70/30 deterministico per id."
        ),
        "cases": cases,
    }
    target = root / "tests" / "decision_corpus_500.json"
    target.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    calibration = sum(1 for case in cases if case["split"] == "calibration")
    print(f"scritti {len(cases)} casi in {target} (calibrazione {calibration}, validazione {len(cases) - calibration})")


if __name__ == "__main__":
    main()
