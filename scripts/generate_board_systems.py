#!/usr/bin/env python3
"""Genera i 30 sistemi comunicativi CAA multilivello in resources/boards/.

Ogni sistema e' un albero di pagine navigabili con celle che parlano, compongono
frasi o portano ad altre pagine. Il contenuto e' scritto a mano tema per tema:
lo script non inventa parole a partire da liste generiche, si limita a
normalizzare, numerare le celle, aggiungere le celle di uscita (Indietro e Home)
e validare il risultato.

Caratteristiche:
  * deterministico: stesso input -> stessi byte, stesso ordine di chiavi;
  * idempotente: la cartella resources/boards viene riscritta per intero, quindi
    i file orfani vengono rimossi;
  * validazione integrata: schema, id, celle, azioni, raggiungibilita' e
    presenza di un terzo livello per ogni sistema;
  * nessuna dipendenza esterna oltre alla libreria standard.

Uso:
    python3 scripts/generate_board_systems.py
"""

from __future__ import annotations

import json
import re
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT_DIR = ROOT / "resources" / "boards"

SYSTEM_ID_RE = re.compile(r"^boards\.[a-z0-9_-]+$")
AGE_THEMES = {"infanzia", "primaria", "secondaria", "adulto"}
ACTIONS = {"speak", "append", "navigate", "back", "home"}

MIN_SYSTEMS = 30
MIN_PAGES = 5
MIN_CELLS_PER_PAGE = 8
MIN_TOTAL_CELLS = 900
MIN_SPEAK_CELLS = 30


def system(slug: str, title: str, description: str, age: str, pages: list[tuple[str, str, str]]) -> dict:
    return {
        "slug": slug,
        "title": title,
        "description": description,
        "ageTheme": age,
        "pages": pages,
    }


SYSTEMS: list[dict] = [
    system(
        "bisogni-essenziali",
        "Bisogni essenziali",
        "Sistema per esprimere bisogni fisici, malessere, richieste di aiuto e conforto, con un livello dedicato alle parti del corpo.",
        "adulto",
        [
            ("home", "Bisogni essenziali", """
Corpo|corpo|Apri i bisogni del corpo|navigate|corpo
Mi sento male|malessere|Apri come mi sento|navigate|malessere
Aiuto|aiuto|Apri le richieste di aiuto|navigate|aiuto
Conforto|conforto|Apri le cose che mi danno conforto|navigate|conforto
Oggetti personali|oggetti|Apri i miei oggetti personali|navigate|oggetti
Frasi utili|frasi|Apri le frasi utili|navigate|frasi
Ho bisogno|ho_bisogno|Ho bisogno di qualcosa|append
Sto bene|sto_bene|Sto bene|speak
"""),
            ("corpo", "Bisogni del corpo", """
Sete|sete|Ho sete|append
Fame|fame|Ho fame|append
Sonno|sonno|Ho sonno|append
Bagno|bagno|Devo andare in bagno|append
Freddo|freddo|Ho freddo|append
Caldo|caldo|Ho caldo|append
Dettagli sul corpo|dettagli_corpo|Apri i dettagli sul corpo|navigate|corpo-dettagli
"""),
            ("corpo-dettagli", "Dettagli sul corpo", """
Testa|testa|Mi fa male la testa|speak
Pancia|pancia|Ho mal di pancia|speak
Schiena|schiena|Mi fa male la schiena|speak
Gambe|gambe|Mi fanno male le gambe|speak
Braccia|braccia|Mi fanno male le braccia|speak
Pelle|pelle|Mi prude la pelle|speak
Non sto bene|non_sto_bene|Non sto bene|speak
"""),
            ("malessere", "Come mi sento", """
Nausea|nausea|Ho nausea|speak
Vertigini|vertigini|Ho le vertigini|speak
Febbre|febbre|Ho la febbre|speak
Tosse|tosse|Ho la tosse|speak
Respiro|respiro|Faccio fatica a respirare|speak
Stanchezza|stanchezza|Sono molto stanco|speak
"""),
            ("aiuto", "Chiedere aiuto", """
Aiuto|aiuto_chiamata|Ho bisogno di aiuto|speak
Chiama qualcuno|chiamare|Chiama qualcuno, per favore|speak
Acqua|acqua_aiuto|Mi porti dell'acqua, per favore|speak
Medicine|medicine|Mi servono le medicine|speak
Medico|medico_aiuto|Vorrei vedere un medico|speak
Non capisco|non_capisco_aiuto|Non capisco, puoi ripetere?|speak
Adesso|adesso|Ho bisogno di aiuto adesso|speak
"""),
            ("conforto", "Conforto", """
Coperta|coperta|Vorrei una coperta|speak
Cuscino|cuscino|Vorrei un cuscino|speak
Luce|luce|Puoi abbassare la luce?|speak
Silenzio|silenzio|Vorrei un po' di silenzio|speak
Compagnia|compagnia|Resta un po' con me|speak
Riposo|riposo|Ho bisogno di riposare|speak
"""),
            ("oggetti", "Oggetti personali", """
Occhiali|occhiali|Dove sono i miei occhiali?|speak
Telefono|telefono|Dov'è il mio telefono?|speak
Bastone|bastone|Mi passi il bastone?|speak
Bottiglia|bottiglia|Vorrei la bottiglia dell'acqua|speak
Fazzoletto|fazzoletto|Mi serve un fazzoletto|speak
Campanello|campanello|Suono il campanello|speak
"""),
            ("frasi", "Frasi e richieste", """
Voglio|voglio|Voglio|append
Ho bisogno di|ho_bisogno_di|Ho bisogno di|append
Non voglio|non_voglio|Non voglio|append
Per favore|per_favore|Per favore|append
Sto male|sto_male|Sto male|speak
Mi sento meglio|meglio|Mi sento meglio|speak
Puoi aiutarmi?|aiutarmi|Puoi aiutarmi?|speak
Grazie|grazie|Grazie per l'aiuto|speak
"""),
        ],
    ),
    system(
        "alimentazione",
        "Alimentazione",
        "Cibi, bevande, pasti, gusti e consistenze per scegliere cosa mangiare e segnalare allergie o preferenze.",
        "adulto",
        [
            ("home", "Alimentazione", """
Cibi|cibi|Apri i cibi|navigate|cibi
Bevande|bevande|Apri le bevande|navigate|bevande
Pasti|pasti|Apri i pasti|navigate|pasti
Gusti|gusti|Apri i gusti|navigate|gusti
Cucinare|cucina|Apri le azioni in cucina|navigate|cucina
Frasi|frasi|Apri le frasi per mangiare|navigate|frasi
Voglio mangiare|voglio_mangiare|Voglio mangiare|append
Non ho fame|non_ho_fame|Non ho fame|speak
"""),
            ("cibi", "Cibi", """
Pane|pane|Vorrei del pane|append
Pasta|pasta|Vorrei della pasta|append
Riso|riso|Vorrei del riso|append
Carne|carne|Vorrei della carne|append
Pesce|pesce|Vorrei del pesce|append
Formaggio|formaggio|Vorrei del formaggio|append
Consistenze|consistenze|Apri le consistenze|navigate|consistenze
"""),
            ("consistenze", "Consistenze", """
Morbido|morbido|Vorrei qualcosa di morbido|speak
Duro|duro|È troppo duro per me|speak
Liquido|liquido|Preferisco cibo liquido|speak
Frullato|frullato|Vorrei un frullato|speak
Senza sale|senza_sale|Vorrei senza sale|speak
Piccante|piccante|Non posso mangiare piccante|speak
Troppo caldo|troppo_caldo|È troppo caldo, aspetto|speak
"""),
            ("bevande", "Bevande", """
Acqua|acqua|Vorrei dell'acqua|append
Succo|succo|Vorrei un succo|append
Latte|latte|Vorrei del latte|append
Caffè|caffe|Vorrei un caffè|append
Tè|te|Vorrei un tè|append
Bibita|bibita|Vorrei una bibita|append
Ghiaccio|ghiaccio|Senza ghiaccio, per favore|speak
"""),
            ("pasti", "Pasti", """
Colazione|colazione|Faccio colazione|speak
Pranzo|pranzo|È ora di pranzo|speak
Cena|cena|Cosa c'è per cena?|speak
Merenda|merenda|Vorrei una merenda|speak
Porzione|porzione|Una porzione piccola|speak
Ancora|ancora|Vorrei ancora un po'|speak
Basta|basta|Basta così, grazie|speak
"""),
            ("gusti", "Gusti", """
Mi piace|mi_piace|Mi piace|append
Non mi piace|non_mi_piace|Non mi piace|append
Buono|buono|È buono|speak
Saporito|saporito|È molto saporito|speak
Dolce|dolce|Preferisco il dolce|speak
Salato|salato|È troppo salato|speak
Amaro|amaro|Questo è amaro|speak
"""),
            ("cucina", "In cucina", """
Cuocere|cuocere|Devo cuocere|speak
Tagliare|tagliare|Mi aiuti a tagliare?|speak
Scaldare|scaldare|Posso scaldare?|speak
Frigorifero|frigorifero|Metto in frigorifero|speak
Piatto|piatto|Mi passi un piatto?|speak
Posate|posate|Mi servono le posate|speak
Tovagliolo|tovagliolo|Vorrei un tovagliolo|speak
"""),
            ("frasi", "Frasi e richieste", """
Voglio mangiare|voglio_mangiare_frase|Voglio mangiare|append
Ho fame|ho_fame|Ho fame|speak
Non ho fame|non_ho_fame_frase|Non ho fame|speak
Sono a dieta|a_dieta|Sono a dieta|speak
Sono allergico|allergico|Sono allergico a un alimento|speak
Posso avere|posso_avere|Posso avere|append
Sono sazio|sazio|Sono sazio, grazie|speak
Buon appetito|buon_appetito|Buon appetito|speak
"""),
        ],
    ),
    system(
        "frutta-verdura",
        "Frutta e verdura",
        "Vocabolario di frutta, verdura, stagioni e mercato, con frasi per comprare, assaggiare e segnalare allergie.",
        "primaria",
        [
            ("home", "Frutta e verdura", """
Frutta|frutta|Apri la frutta|navigate|frutta
Verdura|verdura|Apri la verdura|navigate|verdura
Mercato|mercato|Apri il mercato|navigate|mercato
Preparazione|preparazione|Apri la preparazione|navigate|preparazione
Gusto|gusto|Apri il gusto|navigate|gusto
Frasi|frasi|Apri le frasi|navigate|frasi
Vorrei della frutta|vorrei_frutta|Vorrei della frutta|append
È fresco|fresco_home|È fresco|speak
"""),
            ("frutta", "Frutta", """
Mela|mela|Vorrei una mela|append
Banana|banana|Vorrei una banana|append
Arancia|arancia|Vorrei un'arancia|append
Pera|pera|Vorrei una pera|append
Fragola|fragola|Vorrei delle fragole|append
Uva|uva|Vorrei dell'uva|append
Altri frutti|altri_frutti|Apri gli altri frutti|navigate|frutta-dettagli
"""),
            ("frutta-dettagli", "Altri frutti", """
Ananas|ananas|Vorrei l'ananas|speak
Melone|melone|Vorrei del melone|speak
Anguria|anguria|Vorrei dell'anguria|speak
Kiwi|kiwi|Vorrei un kiwi|speak
Limone|limone|Vorrei un limone|speak
Ciliegie|ciliegie|Vorrei delle ciliegie|speak
Prugna|prugna|Vorrei una prugna|speak
"""),
            ("verdura", "Verdura", """
Insalata|insalata|Vorrei dell'insalata|append
Pomodoro|pomodoro|Vorrei un pomodoro|append
Carota|carota|Vorrei una carota|append
Zucchina|zucchina|Vorrei una zucchina|append
Broccoli|broccoli|Vorrei dei broccoli|append
Patata|patata|Vorrei una patata|append
Cipolla|cipolla|Preferisco senza cipolla|speak
"""),
            ("mercato", "Mercato", """
Comprare|comprare|Vorrei comprare|append
Un chilo|un_chilo|Un chilo, per favore|speak
Maturo|maturo|È maturo?|speak
Fresco|fresco|È fresco?|speak
Biologico|biologico|È biologico?|speak
Prezzo|prezzo|Quanto costa?|speak
Stagioni|stagioni|Apri le stagioni|navigate|stagioni
"""),
            ("stagioni", "Stagioni", """
Primavera|primavera|In primavera|speak
Estate|estate|In estate|speak
Autunno|autunno|In autunno|speak
Inverno|inverno|In inverno|speak
Di stagione|di_stagione|È di stagione?|speak
Surgelato|surgelato|Va bene anche surgelato|speak
Serra|serra|Viene dalla serra|speak
"""),
            ("preparazione", "Preparazione", """
Lavare|lavare|Devo lavare la verdura|speak
Pelare|pelare|Mi aiuti a pelare?|speak
Tagliare|tagliare_frutta|Taglio a pezzi|speak
Cuocere|cuocere_verdura|Lo cuocio al vapore|speak
Crudo|crudo|Lo preferisco crudo|speak
Cotto|cotto|Lo preferisco cotto|speak
Servire|servire|Lo servo in tavola|speak
"""),
            ("gusto", "Gusto", """
Mi piace|mi_piace_frutta|Mi piace la frutta|speak
Non mi piace|non_mi_piace_verdura|Non mi piace la verdura|speak
Dolce|dolce_frutta|È dolce|speak
Acido|acido|È troppo acido|speak
Maturo|maturo_gusto|È molto maturo|speak
Sodo|sodo|È ancora sodo|speak
Assaggiare|assaggiare|Posso assaggiare?|speak
"""),
            ("frasi", "Frasi e richieste", """
Vorrei della frutta|vorrei_frutta_frase|Vorrei della frutta|append
Vorrei della verdura|vorrei_verdura|Vorrei della verdura|append
È fresco?|fresco_domanda|È fresco?|speak
Quanto costa?|quanto_costa|Quanto costa?|speak
Non mi piace|non_mi_piace_frase|Non mi piace|speak
Sono allergico|allergico_frutta|Sono allergico a un frutto|speak
Posso assaggiare?|posso_assaggiare|Posso assaggiare?|speak
Grazie|grazie_frutta|Grazie, molto gentile|speak
"""),
        ],
    ),
    system(
        "igiene",
        "Igiene personale",
        "Routine di igiene per mani, denti, doccia, capelli e viso, con prodotti e richieste di aiuto o privacy.",
        "primaria",
        [
            ("home", "Igiene personale", """
Mani|mani|Apri il lavaggio delle mani|navigate|mani
Denti|denti|Apri la cura dei denti|navigate|denti
Doccia|doccia|Apri la doccia|navigate|doccia
Capelli|capelli|Apri la cura dei capelli|navigate|capelli
Viso|viso|Apri la cura del viso|navigate|viso
Frasi|frasi|Apri le frasi|navigate|frasi
Devo lavarmi|devo_lavarmi|Devo lavarmi|append
Sono pulito|sono_pulito|Sono pulito|speak
"""),
            ("mani", "Mani", """
Acqua|acqua_mani|Apro l'acqua|speak
Sapone|sapone|Prendo il sapone|speak
Strofinare|strofinare|Strofino le mani|speak
Risciacquare|risciacquare|Risciacquo|speak
Asciugare|asciugare|Asciugo le mani|speak
Unghie|unghie|Taglio le unghie|speak
"""),
            ("denti", "Denti", """
Spazzolino|spazzolino|Prendo lo spazzolino|speak
Dentifricio|dentifricio|Metto il dentifricio|speak
Spazzolare|spazzolare|Spazzolo i denti|speak
Risciacquare|risciacquare_denti|Risciacquo la bocca|speak
Filo|filo|Uso il filo interdentale|speak
Dentista|dentista|Ho un appuntamento dal dentista|speak
Due minuti|due_minuti|Spazzolo per due minuti|speak
"""),
            ("doccia", "Doccia", """
Doccia|doccia_azione|Faccio la doccia|speak
Shampoo|shampoo|Uso lo shampoo|speak
Bagnoschiuma|bagnoschiuma|Uso il bagnoschiuma|speak
Temperatura|temperatura|Regolo la temperatura|speak
Salvietta|salvietta|Prendo la salvietta|speak
Asciugamano|asciugamano|Mi passi l'asciugamano?|speak
Prodotti|prodotti|Apri i prodotti per la cura|navigate|prodotti
"""),
            ("prodotti", "Prodotti per la cura", """
Deodorante|deodorante|Uso il deodorante|speak
Crema|crema|Metto la crema|speak
Rasoio|rasoio|Uso il rasoio|speak
Spazzola|spazzola|Uso la spazzola|speak
Cotton fioc|cotton_fioc|Uso un cotton fioc|speak
Sapone liquido|sapone_liquido|Uso il sapone liquido|speak
Asciugacapelli|asciugacapelli|Uso l'asciugacapelli|speak
"""),
            ("capelli", "Capelli", """
Lavare|lavare_capelli|Lavo i capelli|speak
Pettinare|pettinare|Mi pettino|speak
Asciugare|asciugare_capelli|Asciugo i capelli|speak
Tagliare|tagliare_capelli|Vorrei tagliare i capelli|speak
Spazzola|spazzola_capelli|Dove è la spazzola?|speak
Elastico|elastico|Mi serve un elastico|speak
Specchio|specchio|Mi guardo allo specchio|speak
"""),
            ("viso", "Viso", """
Lavare|lavare_viso|Lavo il viso|speak
Asciugare|asciugare_viso|Asciugo il viso|speak
Crema|crema_viso|Metto la crema sul viso|speak
Barba|barba|Devo fare la barba|speak
Trucco|trucco|Metto il trucco|speak
Specchio|specchio_viso|Ho bisogno dello specchio|speak
Brufolo|brufolo|Ho un brufolo|speak
"""),
            ("frasi", "Frasi e richieste", """
Devo lavarmi|devo_lavarmi_frase|Devo lavarmi|append
Mi aiuti?|mi_aiuti_igiene|Mi aiuti a lavarmi?|speak
Non voglio|non_voglio_igiene|Non voglio farlo adesso|speak
Fa male|fa_male_igiene|Fa male|speak
Sono pronto|sono_pronto|Sono pronto|speak
Mi passi l'asciugamano?|passa_asciugamano|Mi passi l'asciugamano?|speak
Ho bisogno di privacy|privacy|Ho bisogno di privacy|speak
Grazie|grazie_igiene|Grazie per l'aiuto|speak
"""),
        ],
    ),
    system(
        "vestirsi",
        "Vestirsi",
        "Capi, taglie, meteo e occasioni per scegliere cosa indossare e chiedere aiuto nella vestizione.",
        "primaria",
        [
            ("home", "Vestirsi", """
Vestiti|vestiti|Apri i vestiti|navigate|vestiti
Meteo|meteo|Apri il meteo|navigate|meteo
Occasioni|occasioni|Apri le occasioni|navigate|occasioni
Vestizione|vestizione|Apri la vestizione|navigate|vestizione
Colori|colori|Apri i colori|navigate|colori
Frasi|frasi|Apri le frasi|navigate|frasi
Voglio indossare|voglio_indossare|Voglio indossare|append
Mi aiuti a vestirmi|aiuto_vestirmi_home|Mi aiuti a vestirmi?|speak
"""),
            ("vestiti", "Vestiti", """
Maglietta|maglietta|Vorrei la maglietta|append
Pantaloni|pantaloni|Vorrei i pantaloni|append
Felpa|felpa|Vorrei la felpa|append
Giacca|giacca|Vorrei la giacca|append
Scarpe|scarpe|Vorrei le scarpe|append
Calze|calze|Vorrei le calze|append
Biancheria|biancheria|Mi serve la biancheria|speak
Taglie|taglie_nav|Apri le taglie|navigate|taglie
"""),
            ("taglie", "Taglie", """
Piccola|piccola|Va bene una taglia piccola|speak
Media|media|Prendo una taglia media|speak
Grande|grande|Mi serve una taglia grande|speak
Stretta|stretta|È troppo stretta|speak
Larga|larga|È troppo larga|speak
Lunga|lunga|È troppo lunga|speak
Corta|corta|È troppo corta|speak
"""),
            ("meteo", "Meteo", """
Caldo|caldo_meteo|Fa caldo|speak
Freddo|freddo_meteo|Fa freddo|speak
Pioggia|pioggia|Sta piovendo|speak
Vento|vento|C'è vento|speak
Neve|neve|Nevica|speak
Sole|sole|C'è il sole|speak
Nuvoloso|nuvoloso|È nuvoloso|speak
"""),
            ("occasioni", "Occasioni", """
Casa|casa_occ|Per stare in casa|speak
Scuola|scuola_occ|Per andare a scuola|speak
Lavoro|lavoro_occ|Per andare al lavoro|speak
Festa|festa_occ|Per una festa|speak
Sport|sport_occ|Per fare sport|speak
Cerimonia|cerimonia|Per una cerimonia|speak
Mare|mare_occ|Per andare al mare|speak
"""),
            ("vestizione", "Vestizione", """
Indossare|indossare|Indosso il capo|speak
Togliere|togliere|Voglio togliermi questo|speak
Allacciare|allacciare|Mi aiuti ad allacciare?|speak
Abbottonare|abbottonare|Devo abbottonare|speak
Cerniera|cerniera|Chiudi la cerniera|speak
Aiuto|aiuto_vest|Mi serve aiuto|speak
Specchio|specchio_vest|Mi guardo allo specchio|speak
"""),
            ("colori", "Colori", """
Bianco|bianco|Preferisco il bianco|speak
Nero|nero|Preferisco il nero|speak
Blu|blu|Mi piace il blu|speak
Rosso|rosso|Vorrei il rosso|speak
Verde|verde|Il verde va bene|speak
Chiaro|chiaro|Un colore chiaro|speak
Scuro|scuro|Un colore scuro|speak
"""),
            ("frasi", "Frasi e richieste", """
Voglio indossare|voglio_indossare_frase|Voglio indossare|append
Mi aiuti a vestirmi|aiuto_vestirmi_frase|Mi aiuti a vestirmi?|speak
Non mi piace|non_mi_piace_vest|Non mi piace questo capo|speak
Mi sta bene|mi_sta_bene|Mi sta bene|speak
Fa freddo|fa_freddo_vest|Fa freddo, mi copro|speak
Posso togliermi?|posso_togliere|Posso togliermi la giacca?|speak
Dov'è|dove_vest|Dov'è il mio capo?|speak
Grazie|grazie_vest|Grazie, mi aiuti molto|speak
"""),
        ],
    ),
    system(
        "emozioni",
        "Emozioni",
        "Riconoscere e nominare le emozioni, leggere i segnali del corpo, chiedere regolazione e bisogni emotivi.",
        "secondaria",
        [
            ("home", "Emozioni", """
Emozioni|emozioni_home|Apri le emozioni|navigate|emozioni
Corpo|corpo_emoz|Apri i segnali del corpo|navigate|corpo
Regolazione|regolazione_home|Apri la regolazione|navigate|regolazione
Intensità|intensita_home|Apri l'intensità|navigate|intensita
Situazioni|situazioni_home|Apri le situazioni|navigate|situazioni
Frasi|frasi_emoz|Apri le frasi|navigate|frasi
Mi sento|mi_sento|Mi sento|append
Ho bisogno di una pausa|pausa_home|Ho bisogno di una pausa|speak
"""),
            ("emozioni", "Emozioni", """
Felice|felice|Sono felice|speak
Triste|triste|Sono triste|speak
Arrabbiato|arrabbiato|Sono arrabbiato|speak
Ansioso|ansioso|Sono ansioso|speak
Spaventato|spaventato|Ho paura|speak
Sorpreso|sorpreso|Sono sorpreso|speak
Calmo|calmo|Sono calmo|speak
"""),
            ("corpo", "Segnali del corpo", """
Cuore|cuore|Il cuore batte forte|speak
Respiro|respiro_emoz|Il respiro è corto|speak
Stomaco|stomaco_emoz|Ho lo stomaco stretto|speak
Tensione|tensione|Sento tensione|speak
Lacrime|lacrime|Mi vengono le lacrime|speak
Energia|energia|Non ho energia|speak
Voce|voce|La voce trema|speak
"""),
            ("regolazione", "Regolazione", """
Respirare|respirare|Faccio respiri profondi|speak
Pausa|pausa_emoz|Ho bisogno di una pausa|speak
Contare|contare|Conto fino a dieci|speak
Camminare|camminare|Faccio una camminata|speak
Parlare|parlare_emoz|Ho bisogno di parlare|speak
Abbraccio|abbraccio|Vorrei un abbraccio|speak
Bisogni|bisogni_emotivi_nav|Apri i bisogni emotivi|navigate|bisogni-emotivi
"""),
            ("bisogni-emotivi", "Bisogni emotivi", """
Sicurezza|sicurezza_emotiva|Ho bisogno di sicurezza|speak
Ascolto|ascolto|Ho bisogno di ascolto|speak
Spazio|spazio|Ho bisogno di spazio|speak
Compagnia|compagnia_emotiva|Vorrei compagnia|speak
Riposo|riposo_emotivo|Ho bisogno di riposo|speak
Rassicurazione|rassicurazione|Mi serve rassicurazione|speak
Rispetto|rispetto_emotivo|Chiedo rispetto|speak
"""),
            ("intensita", "Intensità", """
Poco|poco|Mi sento poco coinvolto|speak
Abbastanza|abbastanza|Sono abbastanza teso|speak
Molto|molto|Sono molto agitato|speak
Troppo|troppo|È troppo per me|speak
Di più|di_piu|Vorrei sentirmi meglio|speak
Di meno|di_meno|Vorrei meno pressione|speak
Al limite|al_limite|Sono al limite|speak
"""),
            ("situazioni", "Situazioni", """
Casa|casa_emoz|A casa mi sento così|speak
Lavoro|lavoro_emoz|Al lavoro mi sento così|speak
Scuola|scuola_emoz|A scuola mi sento così|speak
Amici|amici_emoz|Con gli amici mi sento così|speak
Famiglia|famiglia_emoz|In famiglia mi sento così|speak
Da solo|da_solo_emoz|Da solo mi sento così|speak
In gruppo|in_gruppo|In gruppo mi sento così|speak
"""),
            ("frasi", "Frasi e richieste", """
Mi sento|mi_sento_frase|Mi sento|append
Sono felice|sono_felice|Sono felice|speak
Sono triste|sono_triste|Sono triste|speak
Sono arrabbiato|sono_arrabbiato|Sono arrabbiato|speak
Ho paura|ho_paura|Ho paura|speak
Ho bisogno di|ho_bisogno_emoz|Ho bisogno di|append
Lasciami stare|lasciami_stare|Lasciami stare, per favore|speak
Mi aiuti?|mi_aiuti_emoz|Mi aiuti a calmarmi?|speak
"""),
        ],
    ),
    system(
        "benessere-dolore",
        "Benessere e dolore",
        "Localizzare il dolore, descriverne le caratteristiche, chiedere farmaci, riposo o un medico.",
        "adulto",
        [
            ("home", "Benessere e dolore", """
Dolore|dolore_home|Apri il dolore|navigate|dolore
Sintomi|sintomi_home|Apri i sintomi|navigate|sintomi
Farmaci|farmaci_home|Apri i farmaci|navigate|farmaci
Riposo|riposo_home|Apri il riposo|navigate|riposo
Medico|medico_home|Apri il medico|navigate|medico
Frasi|frasi_benessere|Apri le frasi|navigate|frasi
Mi fa male|mi_fa_male|Mi fa male|append
Sto meglio|sto_meglio_home|Sto meglio|speak
"""),
            ("dolore", "Dolore", """
Testa|testa_dolore|Mi fa male la testa|append
Schiena|schiena_dolore|Mi fa male la schiena|append
Pancia|pancia_dolore|Mi fa male la pancia|append
Gambe|gambe_dolore|Mi fanno male le gambe|append
Braccia|braccia_dolore|Mi fanno male le braccia|append
Denti|denti_dolore|Mi fa male un dente|append
Dettagli|dettagli_dolore_nav|Apri i dettagli del dolore|navigate|dolore-dettagli
"""),
            ("dolore-dettagli", "Dettagli del dolore", """
Forte|forte|Il dolore è forte|speak
Lieve|lieve|Il dolore è lieve|speak
Continuo|continuo|Il dolore è continuo|speak
Intermittente|intermittente|Il dolore va e viene|speak
Bruciore|bruciore|Sento bruciore|speak
Fitta|fitta|Sento una fitta|speak
Intorpidimento|intorpidimento|Sento intorpidimento|speak
"""),
            ("sintomi", "Sintomi", """
Febbre|febbre_sint|Ho la febbre|speak
Nausea|nausea_sint|Ho nausea|speak
Vertigini|vertigini_sint|Ho vertigini|speak
Tosse|tosse_sint|Ho la tosse|speak
Stanchezza|stanchezza_sint|Sono stanco|speak
Respiro|respiro_sint|Faccio fatica a respirare|speak
Brividi|brividi|Ho i brividi|speak
"""),
            ("farmaci", "Farmaci", """
Antidolorifico|antidolorifico|Mi serve un antidolorifico|speak
Compressa|compressa|Devo prendere una compressa|speak
Sciroppo|sciroppo|Mi serve lo sciroppo|speak
Crema|crema_farmaco|Devo mettere la crema|speak
Dose|dose|Qual è la dose?|speak
Orario|orario_farmaco|A che ora devo prenderlo?|speak
Effetto|effetto|Ha avuto effetto?|speak
"""),
            ("riposo", "Riposo", """
Letto|letto|Vorrei andare a letto|speak
Cuscino|cuscino_riposo|Mi serve un cuscino|speak
Buio|buio|Vorrei il buio|speak
Silenzio|silenzio_riposo|Vorrei silenzio|speak
Posizione|posizione_riposo|Mi aiuti a cambiare posizione?|speak
Coperta|coperta_riposo|Vorrei una coperta|speak
Sveglia|sveglia|Imposto la sveglia|speak
"""),
            ("medico", "Medico", """
Appuntamento|appuntamento_medico|Vorrei un appuntamento|speak
Visita|visita_medica|Devo fare una visita|speak
Ricetta|ricetta|Mi serve una ricetta|speak
Esame|esame_medico|Devo fare un esame|speak
Ospedale|ospedale|Devo andare in ospedale|speak
Farmacia|farmacia|Vado in farmacia|speak
Chiamare|chiamare_medico|Chiamate un medico|speak
"""),
            ("frasi", "Frasi e richieste", """
Mi fa male|mi_fa_male_frase|Mi fa male|append
Il dolore è forte|dolore_forte|Il dolore è forte|speak
Ho bisogno di un medico|serve_medico|Ho bisogno di un medico|speak
Mi serve un farmaco|serve_farmaco|Mi serve un farmaco|speak
Non riesco a dormire|non_dormo|Non riesco a dormire|speak
Sto meglio|sto_meglio_frase|Sto meglio|speak
Chiamate aiuto|chiamate_aiuto|Chiamate aiuto, per favore|speak
Grazie|grazie_benessere|Grazie per l'aiuto|speak
"""),
        ],
    ),
    system(
        "famiglia",
        "Famiglia",
        "Persone di famiglia, parentele, attività condivise e frasi affettive per stare in contatto.",
        "primaria",
        [
            ("home", "Famiglia", """
Persone|persone_famiglia|Apri le persone|navigate|persone
Attività|attivita_famiglia|Apri le attività|navigate|attivita
Casa|casa_famiglia|Apri gli spazi di casa|navigate|casa
Emozioni|emozioni_famiglia|Apri le emozioni|navigate|emozioni
Incontri|incontri|Apri gli incontri|navigate|incontri
Frasi|frasi_famiglia|Apri le frasi|navigate|frasi
Voglio vedere|voglio_vedere|Voglio vedere|append
Ti voglio bene|ti_voglio_bene_home|Ti voglio bene|speak
"""),
            ("persone", "Persone", """
Mamma|mamma|Vorrei vedere la mamma|speak
Papà|papa|Vorrei vedere il papà|speak
Fratello|fratello|Dov'è mio fratello?|speak
Sorella|sorella|Dov'è mia sorella?|speak
Nonna|nonna|Vorrei chiamare la nonna|speak
Nonno|nonno|Vorrei chiamare il nonno|speak
Parentele|parentele_nav|Apri le altre parentele|navigate|parentele
"""),
            ("parentele", "Altre parentele", """
Zia|zia|Vorrei sentire la zia|speak
Zio|zio|Vorrei sentire lo zio|speak
Cugino|cugino|Dov'è mio cugino?|speak
Cugina|cugina|Dov'è mia cugina?|speak
Suocera|suocera|Parlo con mia suocera|speak
Suocero|suocero|Parlo con mio suocero|speak
Nipote|nipote|Gioco con mio nipote|speak
"""),
            ("attivita", "Attività insieme", """
Mangiare insieme|mangiare_insieme|Mangiamo insieme|speak
Parlare|parlare_famiglia|Parliamo un po'|speak
Guardare foto|guardare_foto|Guardiamo le foto|speak
Cucinare|cucinare_famiglia|Cuciniamo insieme|speak
Passeggiare|passeggiare_famiglia|Facciamo una passeggiata|speak
Giocare|giocare_famiglia|Giochiamo insieme|speak
Telefonare|telefonare|Telefono a casa|speak
"""),
            ("casa", "Spazi di casa", """
Cucina|cucina_famiglia|Ci vediamo in cucina|speak
Salotto|salotto|Sono in salotto|speak
Camera|camera_famiglia|Sono in camera|speak
Giardino|giardino|Sono in giardino|speak
Tavolo|tavolo_famiglia|Ci sediamo a tavola|speak
Divano|divano|Sono sul divano|speak
Porta|porta_famiglia|Apri la porta, per favore|speak
"""),
            ("emozioni", "Emozioni in famiglia", """
Affetto|affetto|Vi voglio bene|speak
Nostalgia|nostalgia|Ho nostalgia di casa|speak
Gioia|gioia|Sono felice di vedervi|speak
Preoccupazione|preoccupazione|Sono preoccupato|speak
Gratitudine|gratitudine|Vi sono grato|speak
Vicinanza|vicinanza|Mi sento vicino a voi|speak
Litigio|litigio_famiglia|Abbiamo litigato|speak
"""),
            ("incontri", "Incontri", """
Arrivare|arrivare|Sono arrivato|speak
Salutare|salutare_famiglia|Saluto tutti|speak
Abbracciare|abbracciare_famiglia|Vi abbraccio|speak
Invitare|invitare|Vi invito a casa|speak
Restare|restare|Resto ancora un po'|speak
Andare via|andare_via|Devo andare via|speak
Arrivederci|arrivederci_famiglia|Arrivederci, a presto|speak
"""),
            ("frasi", "Frasi e richieste", """
Voglio vedere|voglio_vedere_frase|Voglio vedere|append
Mi manchi|mi_manchi|Mi manchi|speak
Ti voglio bene|ti_voglio_bene_frase|Ti voglio bene|speak
Come stai?|come_stai_famiglia|Come stai?|speak
Ci vediamo presto|ci_vediamo|Ci vediamo presto|speak
Ho bisogno di parlare|bisogno_parlare|Ho bisogno di parlare|speak
Grazie|grazie_famiglia|Grazie di tutto|speak
A presto|a_presto_famiglia|A presto|speak
"""),
        ],
    ),
    system(
        "casa",
        "Casa",
        "Stanze, oggetti, faccende, manutenzione e sicurezza per orientarsi e chiedere aiuto in casa.",
        "adulto",
        [
            ("home", "Casa", """
Stanze|stanze_home|Apri le stanze|navigate|stanze
Oggetti|oggetti_home|Apri gli oggetti|navigate|oggetti
Faccende|faccende_home|Apri le faccende|navigate|faccende
Sicurezza|sicurezza_home|Apri la sicurezza|navigate|sicurezza
Ospiti|ospiti_home|Apri gli ospiti|navigate|ospiti
Frasi|frasi_casa_home|Apri le frasi|navigate|frasi
Dove si trova|dove_si_trova|Dove si trova|append
Mi scusi|mi_scusi_home|Mi scusi|speak
"""),
            ("stanze", "Stanze", """
Cucina|cucina_casa|Vado in cucina|speak
Bagno|bagno_casa|Vado in bagno|speak
Camera|camera_casa|Vado in camera|speak
Salotto|salotto_casa|Vado in salotto|speak
Ingresso|ingresso|Sono in ingresso|speak
Balcone|balcone|Vado sul balcone|speak
Cantina|cantina|Scendo in cantina|speak
"""),
            ("oggetti", "Oggetti", """
Tavolo|tavolo_casa|Mi serve il tavolo|speak
Sedia|sedia|Mi serve una sedia|speak
Letto|letto_casa|Vado a letto|speak
Armadio|armadio|Apro l'armadio|speak
Lampada|lampada|Accendo la lampada|speak
Tappeto|tappeto|Il tappeto è sporco|speak
Chiavi|chiavi_casa|Dove sono le chiavi?|speak
"""),
            ("faccende", "Faccende", """
Pulire|pulire|Devo pulire|speak
Lavare|lavare_casa|Devo lavare|speak
Stendere|stendere|Stendo i panni|speak
Spazzare|spazzare|Spazzo il pavimento|speak
Riordinare|riordinare|Riordino la stanza|speak
Buttare|buttare|Butto la spazzatura|speak
Manutenzione|manutenzione_nav|Apri la manutenzione|navigate|manutenzione
"""),
            ("manutenzione", "Manutenzione", """
Riparare|riparare|Devo riparare qualcosa|speak
Perdita|perdita|C'è una perdita|speak
Lampadina|lampadina|La lampadina è bruciata|speak
Presa|presa|La presa non funziona|speak
Tubo|tubo|Il tubo perde|speak
Attrezzi|attrezzi|Mi servono gli attrezzi|speak
Tecnico|tecnico|Devo chiamare un tecnico|speak
"""),
            ("sicurezza", "Sicurezza", """
Porta chiusa|porta_chiusa|La porta è chiusa|speak
Finestra|finestra_sicurezza|Chiudo la finestra|speak
Gas|gas|Sento odore di gas|speak
Fuoco|fuoco|C'è del fuoco|speak
Allarme|allarme|Suona l'allarme|speak
Chiave|chiave|Ho perso la chiave|speak
Uscita|uscita_casa|Dov'è l'uscita?|speak
"""),
            ("ospiti", "Ospiti", """
Campanello|campanello_casa|Suona il campanello|speak
Accogliere|accogliere|Accolgo gli ospiti|speak
Offrire|offrire|Offro da bere|speak
Sedersi|sedersi|Prego, si sieda|speak
Conversare|conversare_casa|Conversiamo un po'|speak
Accompagnare|accompagnare|Accompagno alla porta|speak
Arrivederci|arrivederci_casa|Arrivederci, grazie|speak
"""),
            ("frasi", "Frasi e richieste", """
Dove si trova|dove_si_trova_frase|Dove si trova|append
Posso aprire la finestra?|aprire_finestra|Posso aprire la finestra?|speak
Fa caldo|fa_caldo_casa|Fa caldo|speak
Ho bisogno di aiuto|aiuto_casa|Ho bisogno di aiuto|speak
Il rubinetto perde|rubinetto_perde|Il rubinetto perde|speak
Chiudi la porta|chiudi_porta|Chiudi la porta, per favore|speak
Mi scusi|mi_scusi_casa|Mi scusi per il disturbo|speak
Grazie|grazie_casa|Grazie per l'ospitalità|speak
"""),
        ],
    ),
    system(
        "scuola-primaria",
        "Scuola primaria",
        "Materie, materiali, attività e regole della classe, con una pagina per lettura e scrittura.",
        "primaria",
        [
            ("home", "Scuola primaria", """
Materie|materie_primaria|Apri le materie|navigate|materie
Materiali|materiali|Apri i materiali|navigate|materiali
Attività|attivita_scuola|Apri le attività|navigate|attivita
Compagni|compagni|Apri i compagni|navigate|compagni
Regole|regole|Apri le regole|navigate|regole
Frasi|frasi_scuola|Apri le frasi|navigate|frasi
Non capisco|non_capisco_home|Non capisco|append
Posso andare in bagno|bagno_permesso|Posso andare in bagno?|speak
"""),
            ("materie", "Materie", """
Italiano|italiano|Abbiamo italiano|speak
Matematica|matematica|Abbiamo matematica|speak
Scienze|scienze|Abbiamo scienze|speak
Storia|storia|Abbiamo storia|speak
Geografia|geografia|Abbiamo geografia|speak
Arte|arte|Abbiamo arte|speak
Lettura e scrittura|lettura_scrittura_nav|Apri lettura e scrittura|navigate|lettura-scrittura
"""),
            ("lettura-scrittura", "Lettura e scrittura", """
Leggere|leggere|Leggo ad alta voce|speak
Scrivere|scrivere|Scrivo sul quaderno|speak
Sillabe|sillabe|Divido in sillabe|speak
Parola|parola|Scrivo una parola|speak
Frase|frase|Scrivo una frase|speak
Disegnare|disegnare|Disegno la storia|speak
Dettato|dettato|Faccio il dettato|speak
"""),
            ("materiali", "Materiali", """
Quaderno|quaderno|Prendo il quaderno|speak
Matita|matita|Ho perso la matita|speak
Gomma|gomma|Mi serve la gomma|speak
Astuccio|astuccio|Apro l'astuccio|speak
Libro|libro|Apro il libro|speak
Forbici|forbici|Uso le forbici|speak
Colla|colla|Mi serve la colla|speak
"""),
            ("attivita", "Attività", """
Ascoltare|ascoltare_scuola|Ascolto la maestra|speak
Rispondere|rispondere|Rispondo alla domanda|speak
Disegnare|disegnare_att|Disegno|speak
Leggere|leggere_att|Leggo|speak
Contare|contare_scuola|Conto|speak
Lavorare in gruppo|gruppo_scuola|Lavoro in gruppo|speak
Ricreazione|ricreazione|Vado in ricreazione|speak
"""),
            ("compagni", "Compagni", """
Amico|amico_scuola|Gioco con il mio amico|speak
Amica|amica_scuola|Parlo con la mia amica|speak
Gruppo|gruppo_compagni|Sono nel gruppo|speak
Turno|turno_scuola|Aspetto il mio turno|speak
Condividere|condividere_scuola|Condivido i colori|speak
Aiutare|aiutare_compagno|Aiuto un compagno|speak
Litigare|litigare_scuola|Ho litigato con un compagno|speak
"""),
            ("regole", "Regole", """
Alzare la mano|alzare_mano|Alzo la mano|speak
Aspettare|aspettare_scuola|Aspetto il turno|speak
Ascoltare|ascoltare_regole|Ascolto in silenzio|speak
Rispettare|rispettare_regole|Rispetto le regole|speak
Chiedere permesso|chiedere_permesso|Chiedo permesso|speak
Riordinare|riordinare_scuola|Riordino i materiali|speak
Silenzio|silenzio_scuola|Faccio silenzio|speak
"""),
            ("frasi", "Frasi e richieste", """
Non capisco|non_capisco_scuola|Non capisco|speak
Puoi ripetere?|ripetere_scuola|Puoi ripetere, per favore?|speak
Mi aiuti?|mi_aiuti_scuola|Mi aiuti?|speak
Ho finito|ho_finito|Ho finito il compito|speak
Devo andare in bagno|bagno_scuola|Devo andare in bagno|speak
Posso bere?|posso_bere|Posso bere?|speak
Non ho capito il compito|compito_non_capito|Non ho capito il compito|speak
Grazie|grazie_scuola|Grazie, maestra|speak
"""),
        ],
    ),
    system(
        "scuola-secondaria",
        "Scuola secondaria",
        "Materie, organizzazione, verifiche e relazioni della scuola secondaria, con un metodo di studio dedicato.",
        "secondaria",
        [
            ("home", "Scuola secondaria", """
Materie|materie_secondaria|Apri le materie|navigate|materie
Organizzazione|organizzazione_scuola|Apri l'organizzazione|navigate|organizzazione
Verifiche|verifiche_home|Apri le verifiche|navigate|verifiche
Relazioni|relazioni_scuola|Apri le relazioni|navigate|relazioni
Laboratori|laboratori_home|Apri i laboratori|navigate|laboratori
Frasi|frasi_secondaria|Apri le frasi|navigate|frasi
Devo studiare|devo_studiare|Devo studiare|append
Ho una domanda|ho_una_domanda|Ho una domanda|speak
"""),
            ("materie", "Materie", """
Italiano|italiano_sec|Abbiamo italiano|speak
Matematica|matematica_sec|Abbiamo matematica|speak
Inglese|inglese|Abbiamo inglese|speak
Scienze|scienze_sec|Abbiamo scienze|speak
Storia|storia_sec|Abbiamo storia|speak
Filosofia|filosofia|Abbiamo filosofia|speak
Informatica|informatica|Abbiamo informatica|speak
"""),
            ("organizzazione", "Organizzazione", """
Orario|orario_scuola|Controllo l'orario|speak
Zaino|zaino|Preparo lo zaino|speak
Compiti|compiti|Ho dei compiti|speak
Scadenze|scadenze_scuola|Segno le scadenze|speak
Armadietto|armadietto|Apro l'armadietto|speak
Registro|registro|Controllo il registro|speak
Assenze|assenze|Devo giustificare un'assenza|speak
"""),
            ("verifiche", "Verifiche", """
Interrogazione|interrogazione|Ho un'interrogazione|speak
Compito|compito_verifica|Ho un compito in classe|speak
Voto|voto|Ho preso un voto|speak
Ripasso|ripasso|Devo ripassare|speak
Ansia|ansia_verifica|Sono in ansia per la verifica|speak
Recupero|recupero|Devo recuperare|speak
Metodo di studio|metodo_studio_nav|Apri il metodo di studio|navigate|metodo-studio
"""),
            ("metodo-studio", "Metodo di studio", """
Riassumere|riassumere|Riassumo il capitolo|speak
Sottolineare|sottolineare|Sottolineo le parti importanti|speak
Schematizzare|schematizzare|Faccio uno schema|speak
Ripetere|ripetere_studio|Ripeto ad alta voce|speak
Appunti|appunti|Rileggo gli appunti|speak
Mappa|mappa|Costruisco una mappa|speak
Tempo|tempo_studio|Mi prendo il tempo necessario|speak
"""),
            ("relazioni", "Relazioni a scuola", """
Compagno|compagno_sec|Parlo con un compagno|speak
Gruppo|gruppo_sec|Lavoro in gruppo|speak
Conflitto|conflitto_scuola|C'è un conflitto|speak
Rispetto|rispetto_scuola|Chiedo rispetto|speak
Collaborare|collaborare|Collaboro con gli altri|speak
Chiedere aiuto|chiedere_aiuto_sec|Chiedo aiuto|speak
Insegnante|insegnante|Parlo con l'insegnante|speak
"""),
            ("laboratori", "Laboratori", """
Esperimento|esperimento|Facciamo un esperimento|speak
Strumenti|strumenti_lab|Prendo gli strumenti|speak
Sicurezza|sicurezza_lab|Rispetto la sicurezza|speak
Gruppo|gruppo_lab|Lavoro con il gruppo|speak
Relazione|relazione_lab|Scrivo la relazione|speak
Osservare|osservare|Osservo il risultato|speak
Conclusione|conclusione|Scrivo la conclusione|speak
"""),
            ("frasi", "Frasi e richieste", """
Ho una verifica|ho_verifica|Ho una verifica|speak
Non ho capito|non_capito_sec|Non ho capito|speak
Posso recuperare?|posso_recuperare|Posso recuperare?|speak
Mi serve una spiegazione|serve_spiegazione|Mi serve una spiegazione|speak
Ho finito|ho_finito_sec|Ho finito|speak
Sono in ritardo|sono_in_ritardo|Sono in ritardo|speak
Posso uscire?|posso_uscire|Posso uscire?|speak
Grazie|grazie_sec|Grazie per l'aiuto|speak
"""),
        ],
    ),
    system(
        "studio-universita",
        "Studio e università",
        "Studio, ricerca, esami e servizi universitari, con un metodo per pianificare sessioni e obiettivi.",
        "adulto",
        [
            ("home", "Studio e università", """
Studio|studio_home|Apri lo studio|navigate|studio
Ricerca|ricerca_home|Apri la ricerca|navigate|ricerca
Esami|esami_home|Apri gli esami|navigate|esami
Organizzazione|organizzazione_univ|Apri l'organizzazione|navigate|organizzazione
Servizi|servizi_univ|Apri i servizi|navigate|servizi
Frasi|frasi_univ|Apri le frasi|navigate|frasi
Devo consegnare|devo_consegnare|Devo consegnare|append
Ho bisogno di un chiarimento|chiarimento|Ho bisogno di un chiarimento|speak
"""),
            ("studio", "Studio", """
Leggere|leggere_univ|Leggo il testo|speak
Riassumere|riassumere_univ|Riassumo il capitolo|speak
Appunti|appunti_univ|Prendo appunti|speak
Mappa|mappa_univ|Costruisco una mappa|speak
Ripetere|ripetere_univ|Ripeto ad alta voce|speak
Concentrarsi|concentrarsi|Provo a concentrarmi|speak
Metodo|metodo_nav|Apri il metodo|navigate|metodo
"""),
            ("metodo", "Metodo di studio", """
Pianificare|pianificare|Pianifico la sessione|speak
Obiettivo|obiettivo|Fisso un obiettivo|speak
Sessione|sessione|Inizio una sessione|speak
Timer|timer|Imposto il timer|speak
Rileggere|rileggere|Rileggo e correggo|speak
Autocorrezione|autocorrezione|Verifico cosa ho capito|speak
Memoria|memoria|Ripasso a distanza|speak
"""),
            ("ricerca", "Ricerca", """
Biblioteca|biblioteca_ricerca|Vado in biblioteca|speak
Articolo|articolo|Cerco un articolo|speak
Database|database|Uso il database|speak
Parole chiave|parole_chiave|Scelgo le parole chiave|speak
Citare|citare|Devo citare la fonte|speak
Fonte|fonte|Verifico la fonte|speak
Tesi|tesi|Lavoro alla tesi|speak
"""),
            ("esami", "Esami", """
Sessione|sessione_esame|È la sessione d'esame|speak
Appello|appello|Mi iscrivo all'appello|speak
Iscrizione|iscrizione|Faccio l'iscrizione|speak
Orale|orale|Ho un esame orale|speak
Scritto|scritto|Ho un esame scritto|speak
Voto|voto_univ|Ho ricevuto un voto|speak
Laurea|laurea|Discuto la laurea|speak
"""),
            ("organizzazione", "Organizzazione", """
Calendario|calendario_univ|Segno sul calendario|speak
Scadenze|scadenze_univ|Controllo le scadenze|speak
Priorità|priorita|Metto in ordine di priorità|speak
Gruppo|gruppo_univ|Studio in gruppo|speak
Relatore|relatore|Parlo con il relatore|speak
Segreteria|segreteria|Contatto la segreteria|speak
Borsa|borsa_studio|Controllo la borsa di studio|speak
"""),
            ("servizi", "Servizi", """
Biblioteca|biblioteca_servizi|Uso la biblioteca|speak
Mensa|mensa|Vado in mensa|speak
Sportello|sportello|Vado allo sportello|speak
Tutor|tutor|Parlo con il tutor|speak
Wifi|wifi_univ|Mi connetto al wifi|speak
Stampa|stampa|Devo stampare|speak
Orientamento|orientamento|Chiedo orientamento|speak
"""),
            ("frasi", "Frasi e richieste", """
Devo studiare|devo_studiare_univ|Devo studiare|speak
Non capisco questo testo|testo_non_capito|Non capisco questo testo|speak
Posso avere un chiarimento?|chiarimento_frase|Posso avere un chiarimento?|speak
Ho un esame|ho_esame|Ho un esame|speak
Mi serve più tempo|serve_tempo|Mi serve più tempo|speak
Ho finito la tesi|tesi_finita|Ho finito la tesi|speak
Sono stanco|stanco_univ|Sono stanco|speak
Grazie|grazie_univ|Grazie per l'aiuto|speak
"""),
        ],
    ),
    system(
        "lavoro",
        "Lavoro",
        "Mansioni, colleghi, strumenti, pause, problemi e sicurezza sul lavoro, con frasi per chiedere aiuto.",
        "adulto",
        [
            ("home", "Lavoro", """
Mansioni|mansioni_home|Apri le mansioni|navigate|mansioni
Colleghi|colleghi_home|Apri i colleghi|navigate|colleghi
Strumenti|strumenti_home|Apri gli strumenti|navigate|strumenti
Pausa|pausa_home|Apri la pausa|navigate|pausa
Problemi|problemi_home|Apri i problemi|navigate|problemi
Frasi|frasi_lavoro|Apri le frasi|navigate|frasi
Devo fare|devo_fare|Devo fare|append
Ho bisogno di aiuto|aiuto_lavoro_home|Ho bisogno di aiuto|speak
"""),
            ("mansioni", "Mansioni", """
Iniziare|iniziare|Inizio il turno|speak
Finire|finire|Finisco il lavoro|speak
Controllare|controllare|Controllo il risultato|speak
Preparare|preparare|Preparo il materiale|speak
Consegnare|consegnare|Consegno il lavoro|speak
Riordinare|riordinare_lavoro|Riordino la postazione|speak
Sicurezza|sicurezza_lavoro_nav|Apri la sicurezza|navigate|sicurezza-lavoro
"""),
            ("sicurezza-lavoro", "Sicurezza sul lavoro", """
Casco|casco|Indosso il casco|speak
Guanti|guanti|Indosso i guanti|speak
Uscita|uscita_lavoro|Dov'è l'uscita?|speak
Rischio|rischio|Segnalo un rischio|speak
Segnalare|segnalare|Segnalo il problema|speak
Formazione|formazione|Ho bisogno di formazione|speak
Infortunio|infortunio|C'è stato un infortunio|speak
"""),
            ("colleghi", "Colleghi", """
Chiedere aiuto|chiedere_aiuto_lavoro|Chiedo aiuto|speak
Collaborare|collaborare_lavoro|Collaboro con i colleghi|speak
Riunione|riunione|Partecipo alla riunione|speak
Turno|turno_lavoro|Cambio turno|speak
Capo|capo|Parlo con il responsabile|speak
Rispetto|rispetto_lavoro|Chiedo rispetto|speak
Pausa|pausa_colleghi|Facciamo una pausa|speak
"""),
            ("strumenti", "Strumenti", """
Computer|computer|Uso il computer|speak
Telefono|telefono_lavoro|Rispondo al telefono|speak
Stampante|stampante|Uso la stampante|speak
Chiavi|chiavi_lavoro|Prendo le chiavi|speak
Badge|badge|Passo il badge|speak
Carrello|carrello|Sposto il carrello|speak
Attrezzi|attrezzi_lavoro|Mi servono gli attrezzi|speak
"""),
            ("pausa", "Pausa", """
Caffè|caffe_pausa|Prendo un caffè|speak
Mangiare|mangiare_pausa|Mangio qualcosa|speak
Riposare|riposare_pausa|Riposo un momento|speak
Bagno|bagno_lavoro|Vado in bagno|speak
Orario|orario_pausa|A che ora è la pausa?|speak
Tornare|tornare|Torno al lavoro|speak
Permesso|permesso_lavoro|Chiedo un permesso|speak
"""),
            ("problemi", "Problemi", """
Ritardo|ritardo_lavoro|Sono in ritardo|speak
Errore|errore|Ho fatto un errore|speak
Guasto|guasto|C'è un guasto|speak
Troppa attività|troppa_attivita|Ho troppa attività|speak
Non capisco|non_capisco_lavoro|Non capisco il compito|speak
Bisogno di aiuto|bisogno_aiuto_lavoro|Ho bisogno di aiuto|speak
Segnalare|segnalare_problema|Segnalo il problema|speak
"""),
            ("frasi", "Frasi e richieste", """
Ho bisogno di una pausa|serve_pausa|Ho bisogno di una pausa|speak
Posso chiedere aiuto?|posso_aiuto|Posso chiedere aiuto?|speak
Non ho capito|non_capito_lavoro|Non ho capito|speak
Ho finito il turno|turno_finito|Ho finito il turno|speak
C'è un problema|c_e_problema|C'è un problema|speak
Devo andare in bagno|bagno_frase_lavoro|Devo andare in bagno|speak
Grazie per la collaborazione|grazie_collaborazione|Grazie per la collaborazione|speak
A domani|a_domani|A domani|speak
"""),
        ],
    ),
    system(
        "gioco",
        "Gioco",
        "Giochi, turni, regole, richieste e emozioni durante il gioco con gli altri.",
        "infanzia",
        [
            ("home", "Gioco", """
Giochi|giochi_home|Apri i giochi|navigate|giochi
Turni|turni_home|Apri i turni|navigate|turni
Richieste|richieste_gioco|Apri le richieste|navigate|richieste
Emozioni|emozioni_gioco|Apri le emozioni|navigate|emozioni
Compagni|compagni_gioco|Apri i compagni|navigate|compagni
Frasi|frasi_gioco|Apri le frasi|navigate|frasi
Voglio giocare|voglio_giocare|Voglio giocare|append
Mi diverto|mi_diverto|Mi diverto|speak
"""),
            ("giochi", "Giochi", """
Puzzle|puzzle|Facciamo un puzzle|speak
Carte|carte|Giochiamo a carte|speak
Costruzioni|costruzioni|Costruisco con i blocchi|speak
Palla|palla|Giochiamo con la palla|speak
Gioco da tavolo|gioco_tavolo|Prendo un gioco da tavolo|speak
Disegno|disegno_gioco|Disegniamo insieme|speak
Nascondino|nascondino|Giochiamo a nascondino|speak
"""),
            ("turni", "Turni", """
Tocca a me|tocca_a_me|Tocca a me|speak
Tocca a te|tocca_a_te|Tocca a te|speak
Aspetta|aspetta_gioco|Aspetta il tuo turno|speak
Ancora|ancora_gioco|Ancora una volta|speak
Basta|basta_gioco|Basta, mi fermo|speak
Cambio|cambio_gioco|Cambio gioco|speak
Regole|regole_nav|Apri le regole|navigate|regole-gioco
"""),
            ("regole-gioco", "Regole del gioco", """
Turno|turno_regola|Rispetto il turno|speak
Aspettare|aspettare_regola|Aspetto con pazienza|speak
Vincere|vincere|Ho vinto|speak
Perdere|perdere|Ho perso|speak
Barare|barare|Non si bara|speak
Rispettare|rispettare_gioco|Rispetto le regole|speak
Fine partita|fine_partita|La partita è finita|speak
"""),
            ("richieste", "Richieste di gioco", """
Giocare con me|giocare_con_me|Vuoi giocare con me?|speak
Posso?|posso_gioco|Posso giocare?|speak
Prestami|prestami|Mi presti il gioco?|speak
Scambiamo|scambiamo|Scambiamo i pezzi?|speak
Ancora una volta|ancora_una_volta|Ancora una partita|speak
Smettiamo|smettiamo|Smettiamo per ora|speak
Scegli tu|scegli_tu|Scegli tu il gioco|speak
"""),
            ("emozioni", "Emozioni nel gioco", """
Divertito|divertito|Sono divertito|speak
Annoiato|annoiato|Sono annoiato|speak
Frustrato|frustrato|Sono frustrato|speak
Orgoglioso|orgoglioso|Sono orgoglioso|speak
Deluso|deluso|Sono deluso|speak
Entusiasta|entusiasta|Sono entusiasta|speak
Calmo|calmo_gioco|Sono calmo|speak
"""),
            ("compagni", "Compagni di gioco", """
Amico|amico_gioco|Gioco con il mio amico|speak
Gruppo|gruppo_gioco|Gioco in gruppo|speak
Squadra|squadra_gioco|Sono nella squadra|speak
Condividere|condividere_gioco|Condivido i giochi|speak
Invitare|invitare_gioco|Invito un amico|speak
Aiutare|aiutare_gioco|Aiuto gli altri|speak
Salutare|salutare_gioco|Saluto i compagni|speak
"""),
            ("frasi", "Frasi e richieste", """
Giochiamo?|giochiamo|Giochiamo?|speak
Tocca a me|tocca_a_me_frase|Tocca a me|speak
Non vale|non_vale|Non vale, hai barato|speak
Mi diverto|mi_diverto_frase|Mi diverto|speak
Ho perso|ho_perso|Ho perso, riprovo|speak
Ancora una partita|ancora_partita|Ancora una partita|speak
Posso giocare con voi?|posso_giocare|Posso giocare con voi?|speak
Grazie|grazie_gioco|Grazie per il gioco|speak
"""),
        ],
    ),
    system(
        "sport",
        "Sport",
        "Sport, azioni, attrezzatura, squadra e regole, con una pagina di allenamento e frasi per il campo.",
        "secondaria",
        [
            ("home", "Sport", """
Sport|sport_home|Apri gli sport|navigate|sport
Azioni|azioni_sport|Apri le azioni|navigate|azioni
Attrezzatura|attrezzatura_home|Apri l'attrezzatura|navigate|attrezzatura
Squadra|squadra_home|Apri la squadra|navigate|squadra
Regole|regole_sport|Apri le regole|navigate|regole
Frasi|frasi_sport|Apri le frasi|navigate|frasi
Voglio provare|voglio_provare|Voglio provare|append
Sono stanco|sono_stanco_home|Sono stanco|speak
"""),
            ("sport", "Sport", """
Calcio|calcio|Gioco a calcio|speak
Basket|basket|Gioco a basket|speak
Nuoto|nuoto|Faccio nuoto|speak
Corsa|corsa|Faccio corsa|speak
Palestra|palestra_sport|Vado in palestra|speak
Tennis|tennis|Gioco a tennis|speak
Bicicletta|bicicletta_sport|Vado in bicicletta|speak
"""),
            ("azioni", "Azioni", """
Correre|correre|Corro|speak
Saltare|saltare|Salto|speak
Lanciare|lanciare|Lancio la palla|speak
Prendere|prendere|Prendo la palla|speak
Passare|passare_sport|Passo la palla|speak
Fermarsi|fermarsi|Mi fermo|speak
Allenamento|allenamento_nav|Apri l'allenamento|navigate|allenamento
"""),
            ("allenamento", "Allenamento", """
Riscaldamento|riscaldamento|Faccio il riscaldamento|speak
Stretching|stretching|Faccio stretching|speak
Serie|serie|Faccio una serie|speak
Ripetizioni|ripetizioni|Conto le ripetizioni|speak
Recupero|recupero_sport|Mi riposo tra le serie|speak
Fatica|fatica|Sento fatica|speak
Costanza|costanza|Mi alleno con costanza|speak
"""),
            ("attrezzatura", "Attrezzatura", """
Scarpe|scarpe_sport|Indosso le scarpe|speak
Maglia|maglia_sport|Indosso la maglia|speak
Pallone|pallone|Prendo il pallone|speak
Racchetta|racchetta|Prendo la racchetta|speak
Borraccia|borraccia|Mi serve la borraccia|speak
Ginocchiera|ginocchiera|Metto la ginocchiera|speak
Casco|casco_sport|Indosso il casco|speak
"""),
            ("squadra", "Squadra", """
Compagni|compagni_sport|Gioco con i compagni|speak
Allenatore|allenatore|Ascolto l'allenatore|speak
Ruolo|ruolo|Conosco il mio ruolo|speak
Passaggio|passaggio|Faccio un passaggio|speak
Difendere|difendere|Difendo la porta|speak
Attaccare|attaccare|Attacco|speak
Fair play|fair_play|Rispetto gli avversari|speak
"""),
            ("regole", "Regole", """
Fallo|fallo|C'è un fallo|speak
Fuorigioco|fuorigioco|È fuorigioco|speak
Punteggio|punteggio|Controllo il punteggio|speak
Tempo|tempo_sport|Quanto tempo manca?|speak
Arbitro|arbitro|Parlo con l'arbitro|speak
Rispetto|rispetto_sport|Rispetto le regole|speak
Pausa|pausa_sport|Facciamo una pausa|speak
"""),
            ("frasi", "Frasi e richieste", """
Passami la palla|passami_palla|Passami la palla|speak
Sono stanco|sono_stanco_frase|Sono stanco|speak
Facciamo pausa|facciamo_pausa|Facciamo una pausa|speak
Mi fa male|fa_male_sport|Mi fa male|speak
Bel gioco|bel_gioco|Bel gioco|speak
Tocca a me|tocca_a_me_sport|Tocca a me|speak
Ho bisogno di acqua|serve_acqua_sport|Ho bisogno di acqua|speak
Grazie|grazie_sport|Grazie per la partita|speak
"""),
        ],
    ),
    system(
        "piscina",
        "Piscina",
        "Acqua, azioni in vasca, spogliatoio, sicurezza e istruzioni, con una pagina sulla respirazione.",
        "primaria",
        [
            ("home", "Piscina", """
Acqua|acqua_piscina|Apri l'acqua|navigate|acqua
Azioni|azioni_piscina|Apri le azioni|navigate|azioni
Spogliatoio|spogliatoio_home|Apri lo spogliatoio|navigate|spogliatoio
Sicurezza|sicurezza_piscina|Apri la sicurezza|navigate|sicurezza
Istruttore|istruttore_home|Apri l'istruttore|navigate|istruttore
Frasi|frasi_piscina|Apri le frasi|navigate|frasi
Voglio nuotare|voglio_nuotare|Voglio nuotare|append
Ho bisogno di aiuto|aiuto_piscina|Ho bisogno di aiuto|speak
"""),
            ("acqua", "Acqua", """
Entrare|entrare|Entro in acqua|speak
Uscire|uscire|Esco dall'acqua|speak
Profondità|profondita|L'acqua è profonda|speak
Temperatura|temperatura_piscina|L'acqua è fredda|speak
Bordo|bordo|Mi avvicino al bordo|speak
Corsia|corsia|Sono nella corsia|speak
Doccia|doccia_piscina|Faccio la doccia|speak
"""),
            ("azioni", "Azioni in vasca", """
Nuotare|nuotare|Nuoto|speak
Galleggiare|galleggiare|Galleggio|speak
Tuffarsi|tuffarsi|Mi tuffo|speak
Remare|remare|Remo con le braccia|speak
Scivolare|scivolare|Scivolo nell'acqua|speak
Riposare|riposare_piscina|Mi riposo al bordo|speak
Respirazione|respirazione_nav|Apri la respirazione|navigate|respirazione
"""),
            ("respirazione", "Respirazione", """
Respirare|respirare_piscina|Respiro con calma|speak
Trattenere|trattenere|Trattengo il respiro|speak
Espirare|espirare|Espiro sott'acqua|speak
Capriola|capriola|Faccio una capriola|speak
Galleggiare|galleggiare_resp|Galleggio supino|speak
Sott'acqua|sott_acqua|Vado sott'acqua|speak
Riemergere|riemergere|Riemergo|speak
"""),
            ("spogliatoio", "Spogliatoio", """
Armadietto|armadietto_piscina|Apro l'armadietto|speak
Costume|costume|Indosso il costume|speak
Cuffia|cuffia|Metto la cuffia|speak
Occhiali|occhiali_piscina|Indosso gli occhiali|speak
Asciugamano|asciugamano_piscina|Prendo l'asciugamano|speak
Doccia|doccia_spogliatoio|Faccio la doccia|speak
Chiave|chiave_piscina|Dov'è la chiave?|speak
"""),
            ("sicurezza", "Sicurezza", """
Scivolare|scivolare_sicurezza|Il pavimento è scivoloso|speak
Correre|correre_piscina|Non corro sul bordo|speak
Bagnino|bagnino|Chiamo il bagnino|speak
Regole|regole_piscina|Rispetto le regole|speak
Fondo|fondo|Controllo il fondo|speak
Aiuto|aiuto_sicurezza|Chiedo aiuto|speak
Chiamare|chiamare_piscina|Chiamo aiuto|speak
"""),
            ("istruttore", "Istruttore", """
Ascoltare|ascoltare_istruttore|Ascolto l'istruttore|speak
Dimostrare|dimostrare|Guardo la dimostrazione|speak
Correggere|correggere|Mi correggo|speak
Provare|provare_piscina|Provo di nuovo|speak
Pausa|pausa_piscina|Faccio una pausa|speak
Livello|livello|Sono a un nuovo livello|speak
Complimenti|complimenti|Ricevo i complimenti|speak
"""),
            ("frasi", "Frasi e richieste", """
Ho bisogno di aiuto|aiuto_frase_piscina|Ho bisogno di aiuto|speak
Non so nuotare|non_so_nuotare|Non so nuotare|speak
Posso riposare?|posso_riposare|Posso riposare?|speak
L'acqua è fredda|acqua_fredda|L'acqua è fredda|speak
Mi hai spruzzato|spruzzato|Mi hai spruzzato|speak
Sto bene|sto_bene_piscina|Sto bene|speak
Ancora una vasca|ancora_vasca|Ancora una vasca|speak
Grazie|grazie_piscina|Grazie, istruttore|speak
"""),
        ],
    ),
    system(
        "musica",
        "Musica",
        "Strumenti, ascolto, pratica e canto, con generi musicali e frasi per regolare volume e preferenze.",
        "secondaria",
        [
            ("home", "Musica", """
Strumenti|strumenti_musica|Apri gli strumenti|navigate|strumenti
Ascoltare|ascoltare_home|Apri l'ascolto|navigate|ascoltare
Suonare|suonare_home|Apri la pratica|navigate|suonare
Cantare|cantare_home|Apri il canto|navigate|cantare
Preferenze|preferenze_home|Apri le preferenze|navigate|preferenze
Frasi|frasi_musica|Apri le frasi|navigate|frasi
Mi piace|mi_piace_musica|Mi piace|append
Alza il volume|volume_su|Alza il volume|speak
"""),
            ("strumenti", "Strumenti", """
Pianoforte|pianoforte|Suono il pianoforte|speak
Chitarra|chitarra|Suono la chitarra|speak
Batteria|batteria|Suono la batteria|speak
Flauto|flauto|Suono il flauto|speak
Violino|violino|Suono il violino|speak
Voce|voce_musica|Uso la voce|speak
Tastiera|tastiera|Suono la tastiera|speak
"""),
            ("ascoltare", "Ascoltare", """
Musica|musica|Ascolto la musica|speak
Volume|volume|Regolo il volume|speak
Cuffie|cuffie|Uso le cuffie|speak
Altoparlante|altoparlante|Accendo l'altoparlante|speak
Playlist|playlist|Scelgo una playlist|speak
Silenzio|silenzio_musica|Vorrei silenzio|speak
Generi|generi_nav|Apri i generi|navigate|generi
"""),
            ("generi", "Generi musicali", """
Classica|classica|Ascolto musica classica|speak
Pop|pop|Ascolto musica pop|speak
Rock|rock|Ascolto rock|speak
Jazz|jazz|Ascolto jazz|speak
Rap|rap|Ascolto rap|speak
Folk|folk|Ascolto folk|speak
Elettronica|elettronica|Ascolto elettronica|speak
"""),
            ("suonare", "Suonare", """
Provare|provare_musica|Provo il brano|speak
Leggere|leggere_spartito|Leggo lo spartito|speak
Ritmo|ritmo|Tengo il ritmo|speak
Accordo|accordo|Faccio un accordo|speak
Melodia|melodia|Suono la melodia|speak
Sbagliare|sbagliare|Ho sbagliato|speak
Ripetere|ripetere_musica|Ripeto la parte|speak
"""),
            ("cantare", "Cantare", """
Testo|testo_canzone|Leggo il testo|speak
Tono|tono|Cerco il tono|speak
Coro|coro|Canto nel coro|speak
Respiro|respiro_canto|Controllo il respiro|speak
Microfono|microfono|Uso il microfono|speak
Insieme|insieme_musica|Cantiamo insieme|speak
Applausi|applausi|Ricevo gli applausi|speak
"""),
            ("preferenze", "Preferenze", """
Mi piace|mi_piace_pref|Mi piace|append
Non mi piace|non_mi_piace_musica|Non mi piace|speak
Bella|bella|È bella|speak
Noiosa|noiosa|È noiosa|speak
Forte|forte_musica|È troppo forte|speak
Dolce|dolce_musica|È dolce|speak
Scegliere|scegliere_musica|Voglio scegliere|speak
"""),
            ("frasi", "Frasi e richieste", """
Alza il volume|alza_volume|Alza il volume|speak
Abbassa la musica|abbassa_musica|Abbassa la musica|speak
Mi piace questa canzone|canzone_piace|Mi piace questa canzone|speak
Posso provare?|posso_provare_musica|Posso provare?|speak
Non la conosco|non_conosco|Non la conosco|speak
Suoniamo insieme|suoniamo_insieme|Suoniamo insieme|speak
Faccio una pausa|pausa_musica|Faccio una pausa|speak
Bravo|bravo_musica|Bravo|speak
"""),
        ],
    ),
    system(
        "tempo-libero",
        "Tempo libero",
        "Attività, uscite, riposo e scelte per organizzare il tempo libero da soli o in compagnia.",
        "adulto",
        [
            ("home", "Tempo libero", """
Attività|attivita_libero|Apri le attività|navigate|attivita
Uscire|uscire_home|Apri le uscite|navigate|uscire
Riposo|riposo_libero|Apri il riposo|navigate|riposo
Scelte|scelte_home|Apri le scelte|navigate|scelte
Compagnia|compagnia_libero|Apri la compagnia|navigate|compagnia
Frasi|frasi_libero|Apri le frasi|navigate|frasi
Oggi voglio|oggi_voglio|Oggi voglio|append
Non ho voglia|non_ho_voglia|Non ho voglia|speak
"""),
            ("attivita", "Attività", """
Leggere|leggere_libero|Leggo un libro|speak
Guardare film|guardare_film|Guardo un film|speak
Passeggiare|passeggiare_libero|Faccio una passeggiata|speak
Cucinare|cucinare_libero|Cucino qualcosa|speak
Disegnare|disegnare_libero|Disegno|speak
Giocare|giocare_libero|Gioco|speak
A casa|a_casa_nav|Apri le attività a casa|navigate|casa-tempo
"""),
            ("casa-tempo", "Attività a casa", """
Serie TV|serie_tv|Guardo una serie|speak
Libro|libro_libero|Leggo un libro|speak
Puzzle|puzzle_libero|Faccio un puzzle|speak
Giardino|giardino_libero|Lavoro in giardino|speak
Riposare|riposare_casa|Riposo|speak
Musica|musica_libero|Ascolto musica|speak
Ospiti|ospiti_libero|Invito degli ospiti|speak
"""),
            ("uscire", "Uscire", """
Passeggiata|passeggiata|Faccio una passeggiata|speak
Parco|parco|Vado al parco|speak
Cinema|cinema|Vado al cinema|speak
Bar|bar|Vado al bar|speak
Museo|museo|Visito un museo|speak
Shopping|shopping|Vado a fare shopping|speak
Bicicletta|bicicletta_libero|Esco in bicicletta|speak
"""),
            ("riposo", "Riposo", """
Dormire|dormire|Dormo|speak
Divano|divano_libero|Mi rilasso sul divano|speak
Silenzio|silenzio_libero|Cerco silenzio|speak
Pausa|pausa_libero|Faccio una pausa|speak
Sonno|sonno_libero|Ho sonno|speak
Sveglia|sveglia_libero|Imposto la sveglia|speak
Energia|energia_libero|Recupero energia|speak
"""),
            ("scelte", "Scelte", """
Cosa faccio|cosa_faccio|Cosa faccio?|speak
Non so|non_so|Non so|speak
Decidi tu|decidi_tu|Decidi tu|speak
Va bene|va_bene_libero|Va bene|speak
Preferisco|preferisco|Preferisco|append
Altro|altro_libero|Vorrei altro|speak
Cambio idea|cambio_idea|Cambio idea|speak
"""),
            ("compagnia", "Compagnia", """
Da solo|da_solo_libero|Sto da solo|speak
Con amici|con_amici|Esco con gli amici|speak
Con famiglia|con_famiglia|Esco con la famiglia|speak
Invitare|invitare_libero|Invito qualcuno|speak
Insieme|insieme_libero|Facciamo insieme|speak
Raggiungere|raggiungere|Raggiungo gli altri|speak
Salutare|salutare_libero|Saluto e vado|speak
"""),
            ("frasi", "Frasi e richieste", """
Cosa facciamo?|cosa_facciamo|Cosa facciamo?|speak
Non ho voglia|non_ho_voglia_frase|Non ho voglia|speak
Mi va bene|mi_va_bene|Mi va bene|speak
Andiamo fuori?|andiamo_fuori|Andiamo fuori?|speak
Sono stanco|stanco_libero|Sono stanco|speak
Preferisco restare|preferisco_restare|Preferisco restare|speak
Ci vediamo dopo|ci_vediamo_dopo|Ci vediamo dopo|speak
Buon divertimento|buon_divertimento|Buon divertimento|speak
"""),
        ],
    ),
    system(
        "amicizie",
        "Amicizie",
        "Persone, attività e conversazioni tra amici, con pagine su conflitti, riparazione e rispetto dei confini.",
        "secondaria",
        [
            ("home", "Amicizie", """
Persone|persone_amici|Apri le persone|navigate|persone
Attività|attivita_amici|Apri le attività|navigate|attivita
Conversazione|conversazione_amici|Apri la conversazione|navigate|conversazione
Conflitti|conflitti_home|Apri i conflitti|navigate|conflitti
Rispetto|rispetto_home|Apri il rispetto|navigate|rispetto
Frasi|frasi_amici|Apri le frasi|navigate|frasi
Ti va di|ti_va_di|Ti va di|append
Mi dispiace|mi_dispiace_home|Mi dispiace|speak
"""),
            ("persone", "Persone", """
Amico|amico|Sei mio amico|speak
Amica|amica|Sei mia amica|speak
Gruppo|gruppo_amici|Siamo un gruppo|speak
Conoscente|conoscente|È un conoscente|speak
Migliore amico|migliore_amico|Sei il mio migliore amico|speak
Compagno|compagno_amici|È un compagno|speak
Vicino|vicino_amici|È il mio vicino|speak
"""),
            ("attivita", "Attività insieme", """
Uscire|uscire_amici|Usciamo insieme|speak
Parlare|parlare_amici|Parliamo|speak
Mangiare insieme|mangiare_amici|Mangiamo insieme|speak
Giocare|giocare_amici|Giochiamo|speak
Cinema|cinema_amici|Andiamo al cinema|speak
Passeggiata|passeggiata_amici|Facciamo una passeggiata|speak
Chiamare|chiamare_amici|Ti chiamo|speak
"""),
            ("conversazione", "Conversazione", """
Come stai|come_stai_amici|Come stai?|speak
Cosa fai|cosa_fai|Cosa fai?|speak
Raccontare|raccontare|Ti racconto|speak
Ascoltare|ascoltare_amici|Ti ascolto|speak
Ridere|ridere|Ridiamo insieme|speak
Interessi|interessi|Parliamo dei nostri interessi|speak
Ricordi|ricordi|Ricordi quella volta?|speak
"""),
            ("conflitti", "Conflitti", """
Litigio|litigio_amici|Abbiamo litigato|speak
Incomprensione|incomprensione|C'è stata un'incomprensione|speak
Gelosia|gelosia|Provo gelosia|speak
Offesa|offesa|Mi sono offeso|speak
Allontanarsi|allontanarsi|Mi allontano un po'|speak
Scusarsi|scusarsi|Voglio scusarmi|speak
Riparare|riparare_nav|Apri come riparare|navigate|riparare
"""),
            ("riparare", "Riparare", """
Scusa|scusa|Scusa|speak
Non volevo|non_volevo|Non volevo ferirti|speak
Capisco|capisco|Capisco il tuo punto|speak
Parliamone|parliamone|Parliamone|speak
Tempo|tempo_riparare|Mi serve tempo|speak
Riprovare|riprovare|Possiamo riprovare?|speak
Abbraccio|abbraccio_riparare|Ti abbraccio|speak
"""),
            ("rispetto", "Rispetto", """
Ascoltare|ascoltare_rispetto|Ti ascolto|speak
Spazio|spazio_rispetto|Ho bisogno di spazio|speak
Confini|confini|Rispetto i tuoi confini|speak
No|no_rispetto|So dire no|speak
Fiducia|fiducia|Ho fiducia in te|speak
Privacy|privacy_amici|Rispetto la tua privacy|speak
Gentilezza|gentilezza|Scelgo la gentilezza|speak
"""),
            ("frasi", "Frasi e richieste", """
Mi manchi|mi_manchi_amici|Mi manchi|speak
Ti va di uscire?|ti_va_uscire|Ti va di uscire?|speak
Sono arrabbiato con te|arrabbiato_te|Sono arrabbiato con te|speak
Mi dispiace|mi_dispiace_frase|Mi dispiace|speak
Ti ascolto|ti_ascolto|Ti ascolto|speak
Ho bisogno di spazio|spazio_frase|Ho bisogno di spazio|speak
Sei importante per me|importante_per_me|Sei importante per me|speak
A presto|a_presto_amici|A presto|speak
"""),
        ],
    ),
    system(
        "conversazione-quotidiana",
        "Conversazione quotidiana",
        "Saluti, domande, risposte e formule di cortesia, con una pagina per chiedere di ripetere o chiarire.",
        "adulto",
        [
            ("home", "Conversazione quotidiana", """
Saluti|saluti_home|Apri i saluti|navigate|saluti
Domande|domande_home|Apri le domande|navigate|domande
Risposte|risposte_home|Apri le risposte|navigate|risposte
Small talk|small_talk_home|Apri le conversazioni|navigate|small-talk
Cortesia|cortesia_home|Apri la cortesia|navigate|cortesia
Frasi|frasi_conv|Apri le frasi|navigate|frasi
Come stai|come_stai_conv|Come stai|append
Non capisco|non_capisco_conv|Non capisco|speak
"""),
            ("saluti", "Saluti", """
Ciao|ciao|Ciao|speak
Buongiorno|buongiorno|Buongiorno|speak
Buonasera|buonasera|Buonasera|speak
Arrivederci|arrivederci|Arrivederci|speak
A presto|a_presto|A presto|speak
Come stai|come_stai_saluti|Come stai?|speak
Piacere|piacere|Piacere di conoscerti|speak
"""),
            ("domande", "Domande", """
Cosa|cosa|Cosa?|speak
Dove|dove|Dove?|speak
Quando|quando|Quando?|speak
Chi|chi|Chi?|speak
Perché|perche|Perché?|speak
Come|come|Come?|speak
Quanto|quanto|Quanto?|speak
"""),
            ("risposte", "Risposte", """
Sì|si|Sì|speak
No|no|No|speak
Forse|forse|Forse|speak
Non so|non_so_conv|Non so|speak
Va bene|va_bene_conv|Va bene|speak
Certo|certo|Certo|speak
Non capisco|non_capisco_nav|Apri le richieste di chiarimento|navigate|non-capisco
"""),
            ("non-capisco", "Chiedere di ripetere", """
Non ho capito|non_ho_capito|Non ho capito|speak
Ripeti|ripeti|Puoi ripetere?|speak
Più lentamente|piu_lentamente|Parla più lentamente|speak
Cosa significa|cosa_significa|Cosa significa?|speak
Puoi scrivere|puoi_scrivere|Puoi scriverlo?|speak
Parla chiaro|parla_chiaro|Parla più chiaro|speak
Scusa|scusa_conv|Scusa|speak
"""),
            ("small-talk", "Conversazioni", """
Meteo|meteo_conv|Parliamo del meteo|speak
Weekend|weekend|Cosa fai nel weekend?|speak
Famiglia|famiglia_conv|Come sta la famiglia?|speak
Lavoro|lavoro_conv|Come va il lavoro?|speak
Cibo|cibo_conv|Ti piace cucinare?|speak
Sport|sport_conv|Segui lo sport?|speak
Novità|novita|Ci sono novità?|speak
"""),
            ("cortesia", "Cortesia", """
Per favore|per_favore_conv|Per favore|speak
Grazie|grazie_conv|Grazie|speak
Scusa|scusa_cortesia|Scusa|speak
Permesso|permesso_conv|Permesso|speak
Buon appetito|buon_appetito_conv|Buon appetito|speak
Salve|salve|Salve|speak
Arrivederci|arrivederci_cortesia|Arrivederci|speak
"""),
            ("frasi", "Frasi e richieste", """
Come stai?|come_stai_frase|Come stai?|speak
Sto bene, grazie|sto_bene_grazie|Sto bene, grazie|speak
Non capisco|non_capisco_frase|Non capisco|speak
Puoi ripetere?|puoi_ripetere|Puoi ripetere?|speak
Di cosa parliamo?|di_cosa_parliamo|Di cosa parliamo?|speak
Interessante|interessante|Interessante|speak
Devo andare|devo_andare|Devo andare|speak
A presto|a_presto_frase|A presto|speak
"""),
        ],
    ),
    system(
        "ristorante",
        "Ristorante",
        "Ordinare, scegliere piatti e bevande, segnalare problemi e allergie, chiedere il conto.",
        "adulto",
        [
            ("home", "Ristorante", """
Ordinare|ordinare_home|Apri le ordinazioni|navigate|ordinare
Cibo|cibo_ristorante|Apri il cibo|navigate|cibo
Bevande|bevande_ristorante|Apri le bevande|navigate|bevande
Problemi|problemi_ristorante|Apri i problemi|navigate|problemi
Pagare|pagare_home|Apri il pagamento|navigate|pagare
Frasi|frasi_ristorante|Apri le frasi|navigate|frasi
Vorrei ordinare|vorrei_ordinare|Vorrei ordinare|append
Il conto, per favore|conto_home|Il conto, per favore|speak
"""),
            ("ordinare", "Ordinare", """
Tavolo|tavolo_ristorante|Vorrei un tavolo|speak
Menù|menu|Posso vedere il menù?|speak
Cameriere|cameriere|Chiamo il cameriere|speak
Vorrei|vorrei|Vorrei|append
Ordinare|ordinare_azione|Ordino ora|speak
Aspettare|aspettare_ristorante|Aspetto il piatto|speak
Consiglio|consiglio|Cosa mi consiglia?|speak
"""),
            ("cibo", "Cibo", """
Primo|primo|Prendo un primo|speak
Secondo|secondo|Prendo un secondo|speak
Contorno|contorno|Prendo un contorno|speak
Insalata|insalata_ristorante|Prendo un'insalata|speak
Pizza|pizza|Vorrei una pizza|speak
Dolce|dolce_ristorante|Vorrei un dolce|speak
Allergie|allergie_nav|Apri le allergie|navigate|allergie
"""),
            ("allergie", "Allergie e intolleranze", """
Allergia|allergia|Ho un'allergia|speak
Intolleranza|intolleranza|Ho un'intolleranza|speak
Glutine|glutine|Sono celiaco|speak
Lattosio|lattosio|Non posso prendere lattosio|speak
Frutta secca|frutta_secca|Attenzione alla frutta secca|speak
Pesce|pesce_allergia|Sono allergico al pesce|speak
Ingredienti|ingredienti|Posso sapere gli ingredienti?|speak
"""),
            ("bevande", "Bevande", """
Acqua|acqua_ristorante|Vorrei dell'acqua|speak
Vino|vino|Vorrei un vino|speak
Birra|birra|Vorrei una birra|speak
Succo|succo_ristorante|Vorrei un succo|speak
Caffè|caffe_ristorante|Vorrei un caffè|speak
Ghiaccio|ghiaccio_ristorante|Senza ghiaccio|speak
Bicchiere|bicchiere|Mi serve un bicchiere|speak
"""),
            ("problemi", "Problemi", """
Freddo|freddo_piatto|Il piatto è freddo|speak
Sbagliato|sbagliato|Questo ordine è sbagliato|speak
Troppo sale|troppo_sale|È troppo salato|speak
Lento|lento|Il servizio è lento|speak
Sporco|sporco|Il tavolo è sporco|speak
Manca|manca|Manca il pane|speak
Cambiare|cambiare_piatto|Posso cambiare piatto?|speak
"""),
            ("pagare", "Pagare", """
Conto|conto|Vorrei il conto|speak
Carta|carta_pagamento|Pago con la carta|speak
Contanti|contanti|Pago in contanti|speak
Ricevuta|ricevuta|Vorrei la ricevuta|speak
Mancia|mancia|Lascio la mancia|speak
Dividere|dividere|Dividiamo il conto|speak
Resto|resto|Il resto, per favore|speak
"""),
            ("frasi", "Frasi e richieste", """
Vorrei ordinare|vorrei_ordinare_frase|Vorrei ordinare|speak
Il conto, per favore|conto_frase|Il conto, per favore|speak
Sono allergico a|allergico_a|Sono allergico a|append
Questo non è quello che ho ordinato|ordine_sbagliato|Questo non è quello che ho ordinato|speak
È buonissimo|buonissimo|È buonissimo|speak
Posso avere dell'acqua?|posso_acqua|Posso avere dell'acqua?|speak
Dov'è il bagno?|bagno_ristorante|Dov'è il bagno?|speak
Grazie|grazie_ristorante|Grazie, tutto ottimo|speak
"""),
        ],
    ),
    system(
        "supermercato",
        "Supermercato",
        "Reparti, prodotti, quantità, prezzi e cassa per fare la spesa e chiedere aiuto.",
        "adulto",
        [
            ("home", "Supermercato", """
Reparti|reparti_home|Apri i reparti|navigate|reparti
Prodotti|prodotti_home|Apri i prodotti|navigate|prodotti
Quantità|quantita_home|Apri le quantità|navigate|quantita
Prezzo|prezzo_home|Apri i prezzi|navigate|prezzo
Cassa|cassa_home|Apri la cassa|navigate|cassa
Frasi|frasi_super|Apri le frasi|navigate|frasi
Vorrei comprare|vorrei_comprare|Vorrei comprare|append
Dove trovo|dove_trovo|Dove trovo|speak
"""),
            ("reparti", "Reparti", """
Frutta|frutta_super|Vado al reparto frutta|speak
Verdura|verdura_super|Vado al reparto verdura|speak
Pane|pane_super|Vado al reparto pane|speak
Carne|carne_super|Vado al reparto carne|speak
Pesce|pesce_super|Vado al reparto pesce|speak
Latticini|latticini|Vado ai latticini|speak
Surgelati|surgelati|Vado ai surgelati|speak
"""),
            ("prodotti", "Prodotti", """
Latte|latte_super|Prendo il latte|speak
Uova|uova|Prendo le uova|speak
Pasta|pasta_super|Prendo la pasta|speak
Riso|riso_super|Prendo il riso|speak
Olio|olio|Prendo l'olio|speak
Detersivo|detersivo|Prendo il detersivo|speak
Carta|carta_casa|Prendo la carta casa|speak
"""),
            ("quantita", "Quantità", """
Un chilo|un_chilo_super|Un chilo, per favore|speak
Mezzo chilo|mezzo_chilo|Mezzo chilo|speak
Un litro|un_litro|Un litro|speak
Una confezione|una_confezione|Una confezione|speak
Due|due|Ne prendo due|speak
Di più|di_piu_super|Ne voglio di più|speak
Basta|basta_super|Basta così|speak
"""),
            ("prezzo", "Prezzo", """
Quanto costa|quanto_costa_super|Quanto costa?|speak
Caro|caro|È troppo caro|speak
Economico|economico|È economico|speak
Totale|totale|Qual è il totale?|speak
Scontrino|scontrino|Controllo lo scontrino|speak
Carta|carta_super|Pago con la carta|speak
Offerte|offerte_nav|Apri le offerte|navigate|offerte
"""),
            ("offerte", "Offerte", """
Sconto|sconto|C'è uno sconto?|speak
Offerta|offerta|È in offerta?|speak
Due per uno|due_per_uno|C'è il due per uno?|speak
Scadenza|scadenza_super|Controllo la scadenza|speak
Marca|marca|Preferisco questa marca|speak
Biologico|biologico_super|Scelgo il biologico|speak
Risparmiare|risparmiare|Voglio risparmiare|speak
"""),
            ("cassa", "Cassa", """
Coda|coda|Faccio la coda|speak
Pagare|pagare_super|Devo pagare|speak
Scontrino|scontrino_cassa|Prendo lo scontrino|speak
Borsa|borsa|Mi serve una borsa|speak
Resto|resto_super|Il resto, per favore|speak
Self-service|self_service|Uso la cassa self-service|speak
Aiuto|aiuto_cassa|Chiedo aiuto alla cassa|speak
"""),
            ("frasi", "Frasi e richieste", """
Dove trovo?|dove_trovo_frase|Dove trovo?|append
Quanto costa?|quanto_costa_frase|Quanto costa?|speak
Vorrei un chilo di|vorrei_chilo|Vorrei un chilo di|append
È in offerta?|in_offerta|È in offerta?|speak
Ho finito la spesa|spesa_finita|Ho finito la spesa|speak
Posso pagare con la carta?|pagare_carta|Posso pagare con la carta?|speak
Mi serve una borsa|serve_borsa|Mi serve una borsa|speak
Grazie|grazie_super|Grazie, arrivederci|speak
"""),
        ],
    ),
    system(
        "trasporti",
        "Trasporti",
        "Mezzi, biglietti, indicazioni, attesa e imprevisti per muoversi in città e fuori.",
        "adulto",
        [
            ("home", "Trasporti", """
Mezzi|mezzi_home|Apri i mezzi|navigate|mezzi
Biglietti|biglietti_home|Apri i biglietti|navigate|biglietti
Indicazioni|indicazioni_home|Apri le indicazioni|navigate|indicazioni
Attesa|attesa_home|Apri l'attesa|navigate|attesa
Imprevisti|imprevisti_home|Apri gli imprevisti|navigate|imprevisti
Frasi|frasi_trasporti|Apri le frasi|navigate|frasi
Devo andare|devo_andare|Devo andare|append
Dove devo scendere|dove_scendere|Dove devo scendere?|speak
"""),
            ("mezzi", "Mezzi", """
Autobus|autobus|Prendo l'autobus|speak
Treno|treno|Prendo il treno|speak
Metro|metro|Prendo la metro|speak
Tram|tram|Prendo il tram|speak
Taxi|taxi|Chiamo un taxi|speak
Bicicletta|bicicletta_trasporti|Uso la bicicletta|speak
A piedi|a_piedi|Vado a piedi|speak
"""),
            ("biglietti", "Biglietti", """
Biglietto|biglietto|Vorrei un biglietto|speak
Abbonamento|abbonamento|Ho l'abbonamento|speak
Convalida|convalida|Convalido il biglietto|speak
Macchinetta|macchinetta|Uso la macchinetta|speak
Controllore|controllore|Arriva il controllore|speak
Tariffa|tariffa|Qual è la tariffa?|speak
Acquisto|acquisto|Faccio l'acquisto|speak
"""),
            ("indicazioni", "Indicazioni", """
Dove|dove_trasporti|Dove devo andare?|speak
Fermata|fermata|Dov'è la fermata?|speak
Direzione|direzione|Qual è la direzione?|speak
Cambio|cambio|Devo cambiare linea|speak
Uscita|uscita_trasporti|Qual è l'uscita?|speak
Mappa|mappa_trasporti|Guardo la mappa|speak
Chiedere|chiedere_indicazioni|Chiedo informazioni|speak
"""),
            ("attesa", "Attesa", """
Aspettare|aspettare_trasporti|Aspetto il mezzo|speak
Banchina|banchina|Sono in banchina|speak
Sedile|sedile|Mi siedo|speak
Annuncio|annuncio|Ascolto l'annuncio|speak
Ritardo|ritardo_trasporti|C'è un ritardo|speak
Arrivare|arrivare_trasporti|Sta arrivando|speak
Orari|orari_nav|Apri gli orari|navigate|orari
"""),
            ("orari", "Orari", """
Orario|orario_mezzi|Controllo l'orario|speak
Ritardo|ritardo_orario|Il treno è in ritardo|speak
Binario|binario|Qual è il binario?|speak
Coincidenza|coincidenza|Ho una coincidenza|speak
Frequenza|frequenza|Ogni quanto passa?|speak
Ultimo|ultimo|A che ora è l'ultimo?|speak
Primo|primo_mezzo|A che ora è il primo?|speak
"""),
            ("imprevisti", "Imprevisti", """
Sciopero|sciopero|C'è uno sciopero|speak
Guasto|guasto_trasporti|C'è un guasto|speak
Perso|perso_mezzo|Ho perso il mezzo|speak
Salto|salto_corsa|Ho saltato la fermata|speak
Affollato|affollato|È molto affollato|speak
Taxi|taxi_imprevisto|Prendo un taxi|speak
Aiuto|aiuto_trasporti|Chiedo aiuto|speak
"""),
            ("frasi", "Frasi e richieste", """
Dove devo scendere?|dove_scendere_frase|Dove devo scendere?|speak
Quale linea prendo?|quale_linea|Quale linea prendo?|speak
Quanto costa il biglietto?|costo_biglietto|Quanto costa il biglietto?|speak
È in ritardo?|in_ritardo|È in ritardo?|speak
Ho perso la coincidenza|perso_coincidenza|Ho perso la coincidenza|speak
Posso sedermi?|posso_sedermi|Posso sedermi?|speak
Mi avvisi alla fermata?|avvisi_fermata|Mi avvisi alla fermata?|speak
Grazie|grazie_trasporti|Grazie per l'aiuto|speak
"""),
        ],
    ),
    system(
        "viaggi",
        "Viaggi",
        "Preparare la valigia, muoversi, gestire l'alloggio e risolvere problemi in viaggio.",
        "adulto",
        [
            ("home", "Viaggi", """
Preparare|preparare_home|Apri la preparazione|navigate|preparare
Muoversi|muoversi_home|Apri gli spostamenti|navigate|muoversi
Alloggio|alloggio_home|Apri l'alloggio|navigate|alloggio
Problemi|problemi_viaggio|Apri i problemi|navigate|problemi
Documenti|documenti_viaggio|Apri i documenti|navigate|documenti
Frasi|frasi_viaggio|Apri le frasi|navigate|frasi
Ho bisogno di|ho_bisogno_viaggio|Ho bisogno di|append
Parlo italiano|parlo_italiano|Parlo italiano|speak
"""),
            ("preparare", "Preparare", """
Valigia|valigia|Preparo la valigia|speak
Biglietti|biglietti_viaggio|Controllo i biglietti|speak
Passaporto|passaporto_prep|Prendo il passaporto|speak
Prenotazione|prenotazione_prep|Controllo la prenotazione|speak
Caricabatterie|caricabatterie_prep|Prendo il caricabatterie|speak
Medicine|medicine_viaggio|Prendo le medicine|speak
Valigia dettagli|valigia_nav|Apri il contenuto della valigia|navigate|valigia
"""),
            ("valigia", "Contenuto della valigia", """
Vestiti|vestiti_valigia|Metto i vestiti|speak
Scarpe|scarpe_valigia|Metto le scarpe|speak
Spazzolino|spazzolino_valigia|Metto lo spazzolino|speak
Caricabatterie|caricabatterie_valigia|Metto il caricabatterie|speak
Medicine|medicine_valigia|Metto le medicine|speak
Libri|libri_valigia|Metto un libro|speak
Bagaglio a mano|bagaglio_mano|Preparo il bagaglio a mano|speak
"""),
            ("muoversi", "Muoversi", """
Aeroporto|aeroporto|Vado in aeroporto|speak
Stazione|stazione_viaggio|Vado in stazione|speak
Hotel|hotel|Cerco l'hotel|speak
Taxi|taxi_viaggio|Prendo un taxi|speak
Mappa|mappa_viaggio|Guardo la mappa|speak
Orientarsi|orientarsi|Provo a orientarmi|speak
Chiedere|chiedere_viaggio|Chiedo indicazioni|speak
"""),
            ("alloggio", "Alloggio", """
Reception|reception|Vado alla reception|speak
Camera|camera_hotel|La camera è pronta?|speak
Chiave|chiave_hotel|Prendo la chiave|speak
Colazione|colazione_hotel|A che ora è la colazione?|speak
Check-in|check_in|Faccio il check-in|speak
Check-out|check_out|Faccio il check-out|speak
Wifi|wifi_hotel|Qual è la password del wifi?|speak
"""),
            ("problemi", "Problemi in viaggio", """
Bagaglio perso|bagaglio_perso|Ho perso il bagaglio|speak
Ritardo|ritardo_viaggio|C'è un ritardo|speak
Prenotazione|problema_prenotazione|C'è un problema con la prenotazione|speak
Stanza|problema_stanza|La stanza non va bene|speak
Lingua|problema_lingua|Non capisco la lingua|speak
Emergenza|emergenza_viaggio|È un'emergenza|speak
Aiuto|aiuto_viaggio|Ho bisogno di aiuto|speak
"""),
            ("documenti", "Documenti", """
Passaporto|passaporto|Ecco il passaporto|speak
Carta d'identità|carta_identita_viaggio|Ecco la carta d'identità|speak
Visto|visto|Ho il visto|speak
Assicurazione|assicurazione|Ho l'assicurazione|speak
Prenotazione|prenotazione_doc|Ecco la prenotazione|speak
Indirizzo|indirizzo|Questo è l'indirizzo|speak
Telefono|telefono_viaggio|Questo è il mio telefono|speak
"""),
            ("frasi", "Frasi e richieste", """
Dov'è il bagno?|bagno_viaggio|Dov'è il bagno?|speak
Parlo italiano|parlo_italiano_frase|Parlo italiano|speak
Non capisco|non_capisco_viaggio|Non capisco|speak
Mi può aiutare?|mi_puo_aiutare|Mi può aiutare?|speak
Ho una prenotazione|ho_prenotazione|Ho una prenotazione|speak
Dov'è la fermata?|fermata_viaggio|Dov'è la fermata?|speak
Ho perso il bagaglio|perso_bagaglio|Ho perso il bagaglio|speak
Grazie|grazie_viaggio|Grazie per l'aiuto|speak
"""),
        ],
    ),
    system(
        "visite-mediche",
        "Visite mediche",
        "Sintomi, visita, farmaci, documenti e sala d'attesa per prepararsi e partecipare a una visita.",
        "adulto",
        [
            ("home", "Visite mediche", """
Sintomi|sintomi_visita|Apri i sintomi|navigate|sintomi
Visita|visita_home|Apri la visita|navigate|visita
Farmaci|farmaci_visita|Apri i farmaci|navigate|farmaci
Documenti|documenti_visita|Apri i documenti|navigate|documenti
Attesa|attesa_visita|Apri l'attesa|navigate|attesa
Frasi|frasi_visita|Apri le frasi|navigate|frasi
Mi fa male|mi_fa_male_visita|Mi fa male|append
Da tre giorni|da_tre_giorni_home|Da tre giorni|speak
"""),
            ("sintomi", "Sintomi", """
Dolore|dolore_visita|Ho dolore|speak
Febbre|febbre_visita|Ho la febbre|speak
Tosse|tosse_visita|Ho la tosse|speak
Nausea|nausea_visita|Ho nausea|speak
Stanchezza|stanchezza_visita|Sono stanco|speak
Vertigini|vertigini_visita|Ho vertigini|speak
Dettagli|dettagli_sintomi_nav|Apri i dettagli dei sintomi|navigate|sintomi-dettagli
"""),
            ("sintomi-dettagli", "Dettagli dei sintomi", """
Da quando|da_quando|Da quando?|speak
Dove|dove_sintomo|Dove fa male?|speak
Forte|forte_sintomo|Il sintomo è forte|speak
Lieve|lieve_sintomo|Il sintomo è lieve|speak
Continuo|continuo_sintomo|È continuo|speak
Peggiora|peggiora|Sta peggiorando|speak
Altro sintomo|altro_sintomo|Ho anche un altro sintomo|speak
"""),
            ("visita", "Visita", """
Medico|medico_visita|Parlo con il medico|speak
Infermiere|infermiere|Parlo con l'infermiere|speak
Visita|visita_azione|Faccio la visita|speak
Domande|domande_visita|Ho delle domande|speak
Esame|esame_visita|Devo fare un esame|speak
Diagnosi|diagnosi|Chiedo la diagnosi|speak
Controllo|controllo|Ho un controllo|speak
"""),
            ("farmaci", "Farmaci", """
Ricetta|ricetta_visita|Mi serve una ricetta|speak
Dose|dose_visita|Qual è la dose?|speak
Orario|orario_terapia|A che ora lo prendo?|speak
Effetti|effetti|Quali sono gli effetti?|speak
Allergia|allergia_visita|Ho un'allergia|speak
Farmacia|farmacia_visita|Vado in farmacia|speak
Terapia|terapia|Seguo la terapia|speak
"""),
            ("documenti", "Documenti", """
Tessera sanitaria|tessera_sanitaria|Ecco la tessera sanitaria|speak
Ricetta|ricetta_doc|Ecco la ricetta|speak
Referto|referto|Ho il referto|speak
Appuntamento|appuntamento_doc|Ho un appuntamento|speak
Moduli|moduli|Compilo i moduli|speak
Identità|identita|Mostro un documento|speak
Cartella|cartella|Porto la cartella clinica|speak
"""),
            ("attesa", "Sala d'attesa", """
Sala|sala_attesa|Sono in sala d'attesa|speak
Numero|numero|Ho il numero|speak
Chiamare|chiamare_visita|Mi chiamano|speak
Aspettare|aspettare_visita|Aspetto|speak
Priorità|priorita_visita|Ho priorità|speak
Urgenza|urgenza|È un'urgenza|speak
Uscire|uscire_visita|Devo uscire un momento|speak
"""),
            ("frasi", "Frasi e richieste", """
Mi fa male qui|male_qui|Mi fa male qui|speak
Da tre giorni|da_tre_giorni|Da tre giorni|speak
Sono allergico a|allergico_visita|Sono allergico a|append
Prendo questi farmaci|prendo_farmaci|Prendo questi farmaci|speak
Ho bisogno di un certificato|serve_certificato|Ho bisogno di un certificato|speak
Quando devo tornare?|quando_tornare|Quando devo tornare?|speak
Ho paura|paura_visita|Ho paura|speak
Grazie|grazie_visita|Grazie, dottore|speak
"""),
        ],
    ),
    system(
        "autonomia",
        "Autonomia",
        "Cura di sé, gestione della casa, spostamenti, denaro e documenti per l'autonomia quotidiana.",
        "adulto",
        [
            ("home", "Autonomia", """
Cura personale|cura_home|Apri la cura personale|navigate|cura-persona
Casa|casa_autonomia|Apri la casa|navigate|casa
Spostamenti|spostamenti_home|Apri gli spostamenti|navigate|spostamenti
Soldi|soldi_home|Apri i soldi|navigate|soldi
Documenti|documenti_home|Apri i documenti|navigate|documenti
Frasi|frasi_autonomia|Apri le frasi|navigate|frasi
Devo fare|devo_fare_autonomia|Devo fare|append
Mi serve aiuto|mi_serve_aiuto|Mi serve aiuto|speak
"""),
            ("cura-persona", "Cura personale", """
Lavarsi|lavarsi|Mi lavo|speak
Vestirsi|vestirsi|Mi vesto|speak
Mangiare|mangiare_autonomia|Mangio|speak
Medicine|medicine_autonomia|Prendo le medicine|speak
Sonno|sonno_autonomia|Dormo|speak
Appuntamento|appuntamento_autonomia|Ho un appuntamento|speak
Chiedere aiuto|chiedere_aiuto_autonomia|Chiedo aiuto|speak
"""),
            ("casa", "Casa", """
Pulire|pulire_autonomia|Pulisco|speak
Cucinare|cucinare_autonomia|Cucino|speak
Lavatrice|lavatrice|Uso la lavatrice|speak
Bollette|bollette|Pago le bollette|speak
Chiavi|chiavi_autonomia|Controllo le chiavi|speak
Riparazione|riparazione_autonomia|Faccio riparare|speak
Sicurezza|sicurezza_autonomia|Controllo la sicurezza|speak
"""),
            ("spostamenti", "Spostamenti", """
Autobus|autobus_autonomia|Prendo l'autobus|speak
Treno|treno_autonomia|Prendo il treno|speak
Mappa|mappa_autonomia|Controllo la mappa|speak
Biglietto|biglietto_autonomia|Prendo il biglietto|speak
Orario|orario_autonomia|Controllo l'orario|speak
Taxi|taxi_autonomia|Chiamo un taxi|speak
Camminare|camminare_autonomia|Cammino|speak
"""),
            ("soldi", "Soldi", """
Contanti|contanti_autonomia|Ho i contanti|speak
Carta|carta_autonomia|Uso la carta|speak
Bancomat|bancomat|Prelievo al bancomat|speak
Resto|resto_autonomia|Controllo il resto|speak
Prezzo|prezzo_autonomia|Controllo il prezzo|speak
Budget|budget|Rispetto il budget|speak
Spesa|spesa_nav|Apri la spesa|navigate|spesa
"""),
            ("spesa", "Fare la spesa", """
Lista|lista_spesa|Preparo la lista|speak
Budget|budget_spesa|Controllo il budget|speak
Supermercato|supermercato_spesa|Vado al supermercato|speak
Pagare|pagare_spesa|Pago alla cassa|speak
Scontrino|scontrino_spesa|Controllo lo scontrino|speak
Borsa|borsa_spesa|Prendo la borsa|speak
Offerte|offerte_spesa|Cerco le offerte|speak
"""),
            ("documenti", "Documenti", """
Carta d'identità|carta_identita|Controllo la carta d'identità|speak
Tessera|tessera|Ho la tessera|speak
Codice fiscale|codice_fiscale|Ho il codice fiscale|speak
Firma|firma|Devo firmare|speak
Scadenza|scadenza_doc|Controllo la scadenza|speak
Fotocopia|fotocopia|Mi serve una fotocopia|speak
Archivio|archivio|Archivio i documenti|speak
"""),
            ("frasi", "Frasi e richieste", """
Devo fare la spesa|devo_spesa|Devo fare la spesa|speak
Quanto ho speso?|quanto_speso|Quanto ho speso?|speak
Mi serve aiuto|serve_aiuto_autonomia|Mi serve aiuto|speak
Ho dimenticato le chiavi|dimenticato_chiavi|Ho dimenticato le chiavi|speak
Devo pagare una bolletta|pagare_bolletta|Devo pagare una bolletta|speak
Posso avere una ricevuta?|ricevuta_autonomia|Posso avere una ricevuta?|speak
Ho un appuntamento|appuntamento_frase|Ho un appuntamento|speak
Grazie|grazie_autonomia|Grazie per l'aiuto|speak
"""),
        ],
    ),
    system(
        "agenda-tempo",
        "Agenda e tempo",
        "Giorni, momenti della giornata, pianificazione, promemoria e scadenze per organizzare il tempo.",
        "adulto",
        [
            ("home", "Agenda e tempo", """
Giorni|giorni_home|Apri i giorni|navigate|giorni
Momenti|momenti_home|Apri i momenti|navigate|momenti
Pianificare|pianificare_home|Apri la pianificazione|navigate|pianificare
Promemoria|promemoria_home|Apri i promemoria|navigate|promemoria
Scadenze|scadenze_home|Apri le scadenze|navigate|scadenze
Frasi|frasi_agenda|Apri le frasi|navigate|frasi
Oggi devo|oggi_devo|Oggi devo|append
Che ore sono|che_ore_sono|Che ore sono?|speak
"""),
            ("giorni", "Giorni", """
Lunedì|lunedi|Lunedì|speak
Martedì|martedi|Martedì|speak
Mercoledì|mercoledi|Mercoledì|speak
Giovedì|giovedi|Giovedì|speak
Venerdì|venerdi|Venerdì|speak
Sabato|sabato|Sabato|speak
Settimana|settimana_nav|Apri la settimana|navigate|settimana
"""),
            ("settimana", "La settimana", """
Oggi|oggi|Oggi|speak
Domani|domani|Domani|speak
Ieri|ieri|Ieri|speak
Questa settimana|questa_settimana|Questa settimana|speak
Prossima settimana|prossima_settimana|La prossima settimana|speak
Fine settimana|fine_settimana|Nel fine settimana|speak
Feriale|feriale|Nei giorni feriali|speak
"""),
            ("momenti", "Momenti", """
Mattina|mattina|La mattina|speak
Pomeriggio|pomeriggio|Il pomeriggio|speak
Sera|sera|La sera|speak
Notte|notte|La notte|speak
Adesso|adesso_agenda|Adesso|speak
Dopo|dopo|Dopo|speak
Prima|prima|Prima|speak
"""),
            ("pianificare", "Pianificare", """
Appuntamento|appuntamento_agenda|Ho un appuntamento|speak
Calendario|calendario|Segno sul calendario|speak
Orario|orario_agenda|Controllo l'orario|speak
Spostare|spostare|Sposto l'impegno|speak
Confermare|confermare|Confermo l'appuntamento|speak
Cancellare|cancellare|Cancello l'impegno|speak
Aggiungere|aggiungere|Aggiungo un impegno|speak
"""),
            ("promemoria", "Promemoria", """
Sveglia|sveglia_agenda|Imposto la sveglia|speak
Nota|nota|Scrivo una nota|speak
Telefono|telefono_agenda|Uso il telefono|speak
Avviso|avviso|Imposto un avviso|speak
Ripetere|ripetere_agenda|Ripeto il promemoria|speak
Importante|importante|È importante|speak
Scadenza|scadenza_promemoria|Ricorda la scadenza|speak
"""),
            ("scadenze", "Scadenze", """
Oggi|oggi_scadenza|Scade oggi|speak
Entro|entro|Entro questa settimana|speak
Entro domani|entro_domani|Entro domani|speak
In ritardo|in_ritardo_agenda|Sono in ritardo|speak
Urgente|urgente|È urgente|speak
Finito|finito_agenda|Ho finito|speak
Rimandare|rimandare|Devo rimandare|speak
"""),
            ("frasi", "Frasi e richieste", """
Che ore sono?|che_ore_frase|Che ore sono?|speak
Quando?|quando_agenda|Quando?|speak
Oggi non posso|oggi_non_posso|Oggi non posso|speak
Facciamo domani|facciamo_domani|Facciamo domani|speak
Sono in ritardo|sono_in_ritardo_agenda|Sono in ritardo|speak
Me lo ricordi?|me_lo_ricordi|Me lo ricordi?|speak
È urgente|e_urgente|È urgente|speak
A dopo|a_dopo|A dopo|speak
"""),
        ],
    ),
    system(
        "tecnologia",
        "Tecnologia",
        "Dispositivi, azioni, problemi, internet e privacy per usare la tecnologia e chiedere aiuto.",
        "adulto",
        [
            ("home", "Tecnologia", """
Dispositivi|dispositivi_home|Apri i dispositivi|navigate|dispositivi
Azioni|azioni_tech|Apri le azioni|navigate|azioni
Problemi|problemi_tech|Apri i problemi|navigate|problemi
Internet|internet_home|Apri internet|navigate|internet
Privacy|privacy_home|Apri la privacy|navigate|privacy
Frasi|frasi_tech|Apri le frasi|navigate|frasi
Voglio aprire|voglio_aprire|Voglio aprire|append
Non funziona|non_funziona|Non funziona|speak
"""),
            ("dispositivi", "Dispositivi", """
Telefono|telefono_tech|Uso il telefono|speak
Computer|computer_tech|Uso il computer|speak
Tablet|tablet|Uso il tablet|speak
Stampante|stampante_tech|Uso la stampante|speak
Cuffie|cuffie_tech|Uso le cuffie|speak
Caricabatterie|caricabatterie_tech|Collego il caricabatterie|speak
Impostazioni|impostazioni_nav|Apri le impostazioni|navigate|impostazioni
"""),
            ("impostazioni", "Impostazioni", """
Wifi|wifi_impostazioni|Attivo il wifi|speak
Bluetooth|bluetooth|Attivo il bluetooth|speak
Volume|volume_tech|Regolo il volume|speak
Luminosità|luminosita|Regolo la luminosità|speak
Aggiornamento|aggiornamento_impostazioni|Faccio l'aggiornamento|speak
Account|account|Controllo l'account|speak
Batteria|batteria|Controllo la batteria|speak
"""),
            ("azioni", "Azioni", """
Accendere|accendere|Accendo il dispositivo|speak
Spegnere|spegnere|Spengo il dispositivo|speak
Aprire|aprire|Apro l'app|speak
Chiudere|chiudere|Chiudo l'app|speak
Scaricare|scaricare|Scarico il file|speak
Inviare|inviare|Invio il messaggio|speak
Salvare|salvare|Salvo il lavoro|speak
"""),
            ("problemi", "Problemi", """
Lento|lento_tech|Il dispositivo è lento|speak
Bloccato|bloccato|Si è bloccato|speak
Batteria|batteria_problema|La batteria è scarica|speak
Connessione|connessione|Manca la connessione|speak
Errore|errore_tech|C'è un errore|speak
Aggiornamento|aggiornamento_problema|L'aggiornamento non parte|speak
Aiuto|aiuto_tech|Mi serve aiuto|speak
"""),
            ("internet", "Internet", """
Wifi|wifi_internet|Mi connetto al wifi|speak
Password|password_internet|Inserisco la password|speak
Sito|sito|Apro il sito|speak
Email|email|Controllo l'email|speak
Videochiamata|videochiamata|Faccio una videochiamata|speak
Ricerca|ricerca_internet|Faccio una ricerca|speak
Download|download|Faccio il download|speak
"""),
            ("privacy", "Privacy", """
Password|password_privacy|Proteggo la password|speak
Permesso|permesso_tech|Chiedo il permesso|speak
Dati|dati|Proteggo i miei dati|speak
Condivisione|condivisione|Controllo la condivisione|speak
Profilo|profilo|Controllo il profilo|speak
Spam|spam|Segnalo lo spam|speak
Bloccare|bloccare|Blocco il contatto|speak
"""),
            ("frasi", "Frasi e richieste", """
Non funziona|non_funziona_frase|Non funziona|speak
Mi aiuti?|mi_aiuti_tech|Mi aiuti?|speak
Come si fa?|come_si_fa|Come si fa?|speak
Ho dimenticato la password|dimenticato_password|Ho dimenticato la password|speak
Posso connettermi?|posso_connettermi|Posso connettermi?|speak
Il wifi non va|wifi_non_va|Il wifi non va|speak
Ho salvato il file|salvato_file|Ho salvato il file|speak
Grazie|grazie_tech|Grazie per l'aiuto|speak
"""),
        ],
    ),
    system(
        "comunicazione-sociale-adulta",
        "Comunicazione sociale adulta",
        "Conversazioni, opinioni, assertività, servizi e relazioni per partecipare alla vita sociale adulta.",
        "adulto",
        [
            ("home", "Comunicazione sociale adulta", """
Conversazione|conversazione_sociale|Apri la conversazione|navigate|conversazione
Opinioni|opinioni_home|Apri le opinioni|navigate|opinioni
Assertività|assertivita_home|Apri l'assertività|navigate|assertivita
Servizi|servizi_sociali|Apri i servizi|navigate|servizi
Relazioni|relazioni_sociali|Apri le relazioni|navigate|relazioni
Frasi|frasi_sociali|Apri le frasi|navigate|frasi
Secondo me|secondo_me|Secondo me|append
Vorrei dire la mia|dire_la_mia|Vorrei dire la mia|speak
"""),
            ("conversazione", "Conversazione", """
Argomento|argomento|Scegliamo un argomento|speak
Ascoltare|ascoltare_sociale|Ascolto|speak
Intervenire|intervenire|Intervengo|speak
Chiedere|chiedere_sociale|Chiedo una precisazione|speak
Chiarire|chiarire|Chiarisco il punto|speak
Congedarsi|congedarsi|Mi congedo|speak
Riunioni|riunioni_nav|Apri le riunioni|navigate|riunioni
"""),
            ("riunioni", "Riunioni", """
Ordine del giorno|ordine_giorno|Leggo l'ordine del giorno|speak
Turno|turno_riunione|Aspetto il mio turno|speak
Proposta|proposta|Faccio una proposta|speak
Obiezione|obiezione|Ho un'obiezione|speak
Accordo|accordo|Troviamo un accordo|speak
Verbale|verbale|Leggo il verbale|speak
Conclusione|conclusione_riunione|Arriviamo alla conclusione|speak
"""),
            ("opinioni", "Opinioni", """
Penso che|penso_che|Penso che|append
Secondo me|secondo_me_op|Secondo me|append
Sono d'accordo|sono_daccordo|Sono d'accordo|speak
Non sono d'accordo|non_daccordo|Non sono d'accordo|speak
Forse|forse_sociale|Forse|speak
Dipende|dipende|Dipende|speak
Rispetto|rispetto_opinioni|Rispetto la tua opinione|speak
"""),
            ("assertivita", "Assertività", """
No|no_assertivo|No|speak
Non voglio|non_voglio_sociale|Non voglio|speak
Ho bisogno|ho_bisogno_sociale|Ho bisogno|append
Chiedo|chiedo|Chiedo|append
Mi sento|mi_sento_sociale|Mi sento|append
Confine|confine|Metto un confine|speak
Compromesso|compromesso|Cerco un compromesso|speak
"""),
            ("servizi", "Servizi", """
Medico|medico_servizi|Vado dal medico|speak
Banca|banca|Vado in banca|speak
Comune|comune|Vado al comune|speak
Posta|posta|Vado alla posta|speak
Negozio|negozio|Vado al negozio|speak
Assistenza|assistenza|Chiedo assistenza|speak
Appuntamento|appuntamento_servizi|Prendo un appuntamento|speak
"""),
            ("relazioni", "Relazioni", """
Collega|collega|Parlo con un collega|speak
Amico|amico_sociale|Parlo con un amico|speak
Famiglia|famiglia_sociale|Parlo con la famiglia|speak
Vicino|vicino_sociale|Saluto il vicino|speak
Gruppo|gruppo_sociale|Partecipo al gruppo|speak
Conflitto|conflitto_sociale|Gestisco un conflitto|speak
Rispetto|rispetto_relazioni|Rispetto gli altri|speak
"""),
            ("frasi", "Frasi e richieste", """
Vorrei dire la mia|dire_la_mia_frase|Vorrei dire la mia|speak
Non sono d'accordo|non_daccordo_frase|Non sono d'accordo|speak
Possiamo parlarne?|possiamo_parlarne|Possiamo parlarne?|speak
Ho bisogno di tempo|serve_tempo_sociale|Ho bisogno di tempo|speak
Capisco il tuo punto|capisco_punto|Capisco il tuo punto|speak
Mi sento ascoltato|mi_sento_ascoltato|Mi sento ascoltato|speak
Chiudiamo qui|chiudiamo_qui|Chiudiamo qui|speak
Grazie|grazie_sociale|Grazie per il confronto|speak
"""),
        ],
    ),
    system(
        "comunicazione-libera-avanzata",
        "Comunicazione libera avanzata",
        "Nuclei, verbi, descrizioni, argomentazione e strategie di riparazione per costruire messaggi liberi.",
        "adulto",
        [
            ("home", "Comunicazione libera avanzata", """
Nuclei|nuclei_home|Apri i nuclei|navigate|nuclei
Verbi|verbi_home|Apri i verbi|navigate|verbi
Descrivere|descrivere_home|Apri le descrizioni|navigate|descrivere
Argomentare|argomentare_home|Apri l'argomentazione|navigate|argomentare
Strategie|strategie_home|Apri le strategie|navigate|strategie
Frasi|frasi_libera|Apri le frasi|navigate|frasi
Voglio dire|voglio_dire|Voglio dire|append
Puoi spiegarmi meglio|spiegami|Puoi spiegarmi meglio?|speak
"""),
            ("nuclei", "Nuclei", """
Io|io|Io|speak
Tu|tu|Tu|speak
Lui|lui|Lui|speak
Lei|lei|Lei|speak
Noi|noi|Noi|speak
Voi|voi|Voi|speak
Loro|loro|Loro|speak
"""),
            ("verbi", "Verbi", """
Essere|essere|Essere|append
Avere|avere|Avere|append
Volere|volere|Volere|append
Potere|potere|Potere|append
Dovere|dovere|Dovere|append
Fare|fare|Fare|append
Dire|dire|Dire|append
"""),
            ("descrivere", "Descrivere", """
Grande|grande|È grande|speak
Piccolo|piccolo|È piccolo|speak
Caldo|caldo_desc|È caldo|speak
Freddo|freddo_desc|È freddo|speak
Bello|bello|È bello|speak
Brutto|brutto|È brutto|speak
Importante|importante_desc|È importante|speak
"""),
            ("argomentare", "Argomentare", """
Perché|perche_arg|Perché|speak
Quindi|quindi|Quindi|speak
Però|pero|Però|speak
Secondo me|secondo_me_arg|Secondo me|append
Primo|primo_arg|Primo|speak
Inoltre|inoltre|Inoltre|speak
Connettivi|connettivi_nav|Apri i connettivi|navigate|connettivi
"""),
            ("connettivi", "Connettivi", """
Ma|ma|Ma|speak
Però|pero_conn|Però|speak
Quindi|quindi_conn|Quindi|speak
Perché|perche_conn|Perché|speak
Anche|anche|Anche|speak
Invece|invece|Invece|speak
Infine|infine|Infine|speak
"""),
            ("strategie", "Strategie", """
Non capisco|non_capisco_libera|Non capisco|speak
Ripeti|ripeti_libera|Ripeti, per favore|speak
Più lentamente|piu_lentamente_libera|Più lentamente|speak
Scrivi|scrivi|Puoi scriverlo?|speak
Esempio|esempio|Fammi un esempio|speak
Pausa|pausa_libera|Ho bisogno di una pausa|speak
Aiuto|aiuto_libera|Mi serve aiuto|speak
"""),
            ("frasi", "Frasi e richieste", """
Voglio dire che|voglio_dire_che|Voglio dire che|append
Secondo me|secondo_me_frase|Secondo me|append
Non sono d'accordo perché|non_daccordo_perche|Non sono d'accordo perché|append
Puoi spiegarmi meglio?|spiegami_frase|Puoi spiegarmi meglio?|speak
Facciamo un esempio|facciamo_esempio|Facciamo un esempio|speak
In conclusione|in_conclusione|In conclusione|speak
Ho bisogno di una pausa|pausa_frase_libera|Ho bisogno di una pausa|speak
Grazie per l'ascolto|grazie_ascolto|Grazie per l'ascolto|speak
"""),
        ],
    ),
]


def build_system(spec: dict) -> dict:
    """Trasforma la specifica compatta in un sistema completo e numerato."""
    pages_out = []
    for page_id, title, raw in spec["pages"]:
        cells = []
        for line in raw.splitlines():
            line = line.strip()
            if not line:
                continue
            parts = [p.strip() for p in line.split("|")]
            if len(parts) not in (4, 5):
                raise ValueError(f"{spec['slug']}/{page_id}: riga non valida: {line!r}")
            label, symbol, spoken, action_type = parts[:4]
            if not label or not symbol or not spoken:
                raise ValueError(f"{spec['slug']}/{page_id}: campo vuoto in {line!r}")
            action = {"type": action_type}
            if action_type == "navigate":
                if len(parts) != 5 or not parts[4]:
                    raise ValueError(f"{spec['slug']}/{page_id}: navigazione senza target: {line!r}")
                action["targetPageId"] = parts[4]
            elif len(parts) == 5:
                raise ValueError(f"{spec['slug']}/{page_id}: target su azione {action_type}: {line!r}")
            cells.append(
                {
                    "id": "",
                    "label": label,
                    "symbolLabel": symbol,
                    "spokenText": spoken,
                    "action": action,
                }
            )

        # Ogni pagina non-home offre sempre una via d'uscita esplicita.
        if page_id != "home":
            cells.append(
                {
                    "id": "",
                    "label": "Indietro",
                    "symbolLabel": "indietro",
                    "spokenText": "Torna indietro",
                    "action": {"type": "back"},
                }
            )
            cells.append(
                {
                    "id": "",
                    "label": "Home",
                    "symbolLabel": "home",
                    "spokenText": "Vai alla home",
                    "action": {"type": "home"},
                }
            )

        pages_out.append({"id": page_id, "title": title, "cells": cells})

    # Gli id delle celle sono unici nell'intero sistema, non solo nella pagina.
    counter = 0
    for page in pages_out:
        for cell in page["cells"]:
            counter += 1
            cell["id"] = f"c{counter}"

    return {
        "id": f"boards.{spec['slug']}",
        "title": spec["title"],
        "description": spec["description"],
        "ageTheme": spec["ageTheme"],
        "rootPageId": "home",
        "pages": pages_out,
    }


def _reachable_depths(pages: list[dict]) -> dict[str, int]:
    by_id = {p["id"]: p for p in pages}
    depths = {"home": 0}
    queue = ["home"]
    while queue:
        current = queue.pop(0)
        for cell in by_id[current]["cells"]:
            action = cell["action"]
            if action["type"] != "navigate":
                continue
            target = action["targetPageId"]
            if target not in depths:
                depths[target] = depths[current] + 1
                queue.append(target)
    return depths


def validate(systems: list[dict]) -> tuple[list[str], dict]:
    errors: list[str] = []
    action_counts: Counter[str] = Counter()
    total_pages = 0
    total_cells = 0
    speak_cells = 0
    pages_per_system: dict[str, int] = {}

    if len(systems) != MIN_SYSTEMS:
        errors.append(f"attesi {MIN_SYSTEMS} sistemi, trovati {len(systems)}")

    ids = Counter(s["id"] for s in systems)
    for sid, count in ids.items():
        if count > 1:
            errors.append(f"id duplicato: {sid}")

    for s in systems:
        sid = s.get("id", "<senza id>")
        if not SYSTEM_ID_RE.match(sid):
            errors.append(f"{sid}: id non conforme a boards.[a-z0-9_-]+")
        if s.get("ageTheme") not in AGE_THEMES:
            errors.append(f"{sid}: ageTheme non valido: {s.get('ageTheme')!r}")
        if not s.get("title"):
            errors.append(f"{sid}: titolo mancante")
        if not s.get("description"):
            errors.append(f"{sid}: descrizione mancante")

        pages = s.get("pages", [])
        total_pages += len(pages)
        pages_per_system[sid] = len(pages)
        if len(pages) < MIN_PAGES:
            errors.append(f"{sid}: solo {len(pages)} pagine")

        page_ids = [p.get("id") for p in pages]
        if len(set(page_ids)) != len(page_ids):
            errors.append(f"{sid}: id pagina duplicati")
        page_set = set(page_ids)
        if s.get("rootPageId") not in page_set:
            errors.append(f"{sid}: rootPageId inesistente")

        cell_ids: set[str] = set()
        for page in pages:
            pid = page.get("id", "<senza id>")
            if not pid:
                errors.append(f"{sid}: pagina senza id")
            if not page.get("title"):
                errors.append(f"{sid}/{pid}: titolo pagina mancante")
            cells = page.get("cells", [])
            if len(cells) < MIN_CELLS_PER_PAGE:
                errors.append(f"{sid}/{pid}: {len(cells)} celle (minimo {MIN_CELLS_PER_PAGE})")
            labels: set[str] = set()
            for cell in cells:
                cid = cell.get("id", "")
                if not cid:
                    errors.append(f"{sid}/{pid}: cella senza id")
                elif cid in cell_ids:
                    errors.append(f"{sid}/{pid}: id cella duplicato {cid}")
                cell_ids.add(cid)
                label = (cell.get("label") or "").strip()
                if not label:
                    errors.append(f"{sid}/{pid}/{cid}: label vuota")
                if label in labels:
                    errors.append(f"{sid}/{pid}: label duplicata {label!r}")
                labels.add(label)
                if not (cell.get("symbolLabel") or "").strip():
                    errors.append(f"{sid}/{pid}/{cid}: symbolLabel vuoto")
                if not (cell.get("spokenText") or "").strip():
                    errors.append(f"{sid}/{pid}/{cid}: spokenText vuoto")

                action = cell.get("action") or {}
                atype = action.get("type")
                if atype not in ACTIONS:
                    errors.append(f"{sid}/{pid}/{cid}: azione non valida {atype!r}")
                    continue
                action_counts[atype] += 1
                if atype == "navigate":
                    target = action.get("targetPageId")
                    if target not in page_set:
                        errors.append(f"{sid}/{pid}/{cid}: target inesistente {target!r}")
                elif atype == "speak":
                    speak_cells += 1

        # Uscite obbligatorie su ogni pagina non-home.
        for page in pages:
            if page.get("id") == "home":
                continue
            types = [c.get("action", {}).get("type") for c in page.get("cells", [])]
            if "back" not in types:
                errors.append(f"{sid}/{page.get('id')}: manca una cella back")
            if "home" not in types:
                errors.append(f"{sid}/{page.get('id')}: manca una cella home")

        # Home: almeno 6 collegamenti e 2 azioni non di navigazione.
        home = next((p for p in pages if p.get("id") == "home"), None)
        if home is None:
            errors.append(f"{sid}: pagina home mancante")
        else:
            types = [c.get("action", {}).get("type") for c in home.get("cells", [])]
            if types.count("navigate") < 6:
                errors.append(f"{sid}: home con {types.count('navigate')} collegamenti (minimo 6)")
            if sum(1 for t in types if t != "navigate") < 2:
                errors.append(f"{sid}: home con meno di 2 azioni")

        # Raggiungibilita' e terzo livello.
        depths = _reachable_depths(pages)
        unreachable = [pid for pid in page_ids if pid not in depths]
        for pid in unreachable:
            errors.append(f"{sid}: pagina irraggiungibile {pid}")
        if depths and max(depths.values()) < 2:
            errors.append(f"{sid}: manca un terzo livello navigabile")

        # Almeno una pagina di frasi/richieste con speak o append.
        if not any(
            sum(1 for c in p.get("cells", []) if c.get("action", {}).get("type") in {"speak", "append"}) >= 3
            for p in pages
        ):
            errors.append(f"{sid}: nessuna pagina di frasi con almeno 3 azioni speak/append")

        total_cells += sum(len(p.get("cells", [])) for p in pages)

    if total_cells < MIN_TOTAL_CELLS:
        errors.append(f"celle totali {total_cells} < {MIN_TOTAL_CELLS}")
    if speak_cells < MIN_SPEAK_CELLS:
        errors.append(f"celle speak {speak_cells} < {MIN_SPEAK_CELLS}")

    stats = {
        "systems": len(systems),
        "pages": total_pages,
        "cells": total_cells,
        "speak_cells": speak_cells,
        "action_counts": action_counts,
        "pages_per_system": pages_per_system,
    }
    return errors, stats


def write_all(systems: list[dict]) -> None:
    """Riscrive l'intera cartella: il set prodotto e' esattamente quello atteso."""
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    for old in sorted(OUT_DIR.glob("*.json")):
        old.unlink()
    for s in sorted(systems, key=lambda item: item["id"]):
        slug = s["id"].split(".", 1)[1]
        data = json.dumps(s, ensure_ascii=False, indent=2) + "\n"
        # Il caricatore Rust cerca "resources/boards/<id>.json": il nome file
        # coincide con l'id completo, non con il solo slug.
        (OUT_DIR / f"boards.{slug}.json").write_text(data, encoding="utf-8")


def load_written() -> list[dict]:
    return [json.loads(path.read_text(encoding="utf-8")) for path in sorted(OUT_DIR.glob("*.json"))]


def main() -> int:
    try:
        systems = [build_system(spec) for spec in SYSTEMS]
    except ValueError as exc:
        print(f"ERRORE: {exc}", file=sys.stderr)
        return 1

    errors, _ = validate(systems)
    if errors:
        for err in errors:
            print(f"ERRORE: {err}", file=sys.stderr)
        return 1

    write_all(systems)

    # Rilegge i file appena scritti: la validazione riguarda il disco, non solo la memoria.
    loaded = load_written()
    errors, stats = validate(loaded)
    if errors:
        for err in errors:
            print(f"ERRORE: {err}", file=sys.stderr)
        return 1

    counts = stats["action_counts"]
    short = sorted(sid for sid, n in stats["pages_per_system"].items() if n < 5)
    print(f"File scritti: {len(loaded)} in {OUT_DIR.relative_to(ROOT)}")
    print(f"Pagine totali: {stats['pages']}")
    print(f"Celle totali: {stats['cells']}")
    print(
        "Celle per azione: "
        + ", ".join(f"{name}={counts.get(name, 0)}" for name in ("speak", "append", "navigate", "back", "home"))
    )
    print(f"Celle con azione speak: {stats['speak_cells']}")
    print("Temi con meno di 5 pagine: " + (", ".join(short) if short else "nessuno"))
    print("Validazione: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
