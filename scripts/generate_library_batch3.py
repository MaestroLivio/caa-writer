#!/usr/bin/env python3
"""Terzo lotto della biblioteca: 60 nuovi modelli validati, costruiti con i
componenti dell'editor e i principi della ricerca documentata."""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
from generate_library_batch2 import (  # noqa: E402
    ACCENT_SOFT,
    agenda_checklist,
    base_spec,
    header,
    page,
    style,
    tax,
    el,
    agenda_daily,
    agenda_first_then,
    agenda_weekly,
    break_card,
    caa_notebook,
    caa_story,
    caa_table,
    choice_board,
    counting_sheet,
    emotion_scale,
    game_cards,
    language_sheet,
    math_sheet,
    science_sheet,
    social_story,
    teacch_cards,
    teacch_matching,
    teacch_sequence,
    teacch_sorting,
    token_board,
)

OUT = ROOT / "templates" / "sources" / "templates" / "lotto3"


def save(spec):
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / f"{spec['id']}.json").write_text(json.dumps(spec, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def number_poster(pid, title, desc):
    words = ["uno", "due", "tre", "quattro", "cinque", "sei", "sette", "otto", "nove"]
    els = []
    header(els, "Numeri", title, "Poster da stampare e appendere.", page_no=1, total=1)
    for i, (number, word) in enumerate(zip(range(1, 10), words)):
        x = 14 + (i % 3) * 62
        y = 48 + (i // 3) * 72
        els.append(el(f"n{i}", "text", str(number), x=x, y=y, w=52, h=28, st=style(size=22, color="#ffffff", background="#0f766e", radius=5, align="center", valign="middle", bold=True)))
        els.append(el(f"w{i}", "text", word, x=x, y=y + 32, w=52, h=18, st=style(size=5.0, align="center", valign="middle")))
    return base_spec(pid, title, desc, "Materiali stampabili", "printable", tax("stampabili", "poster", "infanzia", desc, ["numeri", "poster", "conteggio"], "base", False, False, True), interactive=False, orientation="portrait", pages=[page("home", title, els, home=True)])


def main():
    specs = []

    for args in [
        ("gen3_caa_igiene", "Comunicazione: igiene", "Parole per la cura di sé e l'igiene.", ["lavare", "mani", "viso", "denti", "spazzolino", "sapone", "asciugare", "bagno", "doccia", "pettine", "camera da letto", "vestirsi", "acqua", "asciugamano", "specchio", "aiuto"]),
        ("gen3_caa_luoghi", "Comunicazione: luoghi", "Luoghi della comunità e della scuola.", ["casa", "scuola", "parco", "supermercato", "biblioteca", "ospedale", "piscina", "museo", "ristorante", ("stazione", "stazione ferroviaria"), "autobus", "strada", "giardino", "palestra", "negozio", "andare"]),
        ("gen3_caa_persone", "Comunicazione: persone", "Persone della famiglia, della scuola e della comunità.", ["mamma", "papà", "nonna", "nonno", "fratello", "sorella", "maestra", "compagno", "amico", "dottore", "adulto", "bambino", "persona", "famiglia", ("insieme", "unire"), "aiuto"]),
        ("gen3_caa_azioni", "Comunicazione: azioni", "Verbi per descrivere che cosa faccio.", ["andare", "venire", "mangiare", "bere", "dormire", "giocare", "leggere", "scrivere", "disegnare", "correre", "saltare", "parlare", "ascoltare", "guardare", "aspettare", "finire"]),
        ("gen3_caa_descrittori", "Comunicazione: descrittori", "Parole per descrivere quantità, qualità e temperatura.", ["grande", "piccolo", "alto", "basso", "caldo", "freddo", "bello", "brutto", "buono", "pieno", "vuoto", "veloce", "lento", "molto", "poco", "ancora"]),
        ("gen3_caa_scelte", "Comunicazione: scelgo", "Parole per fare richieste e scegliere.", ["voglio", "non voglio", "ancora", "finito", "più", "meno", "questo", "quello", "aiuto", ("pausa", "riposo"), "dopo", "adesso", "per favore", "grazie", "sì", "no"]),
    ]:
        specs.append(caa_table(*args))

    specs.append(caa_notebook("gen3_quaderno_comunita", "Quaderno: nella comunità", "Quaderno per negozi, trasporti e luoghi pubblici.", [
        ("negozio", "Negozio", ["negozio", "supermercato", "soldi", "comprare", "cassa", "cibo", "acqua", "borsa", "grazie"]),
        ("trasporti", ("Trasporti", "treno"), ["autobus", "treno", "macchina", "fermare", "aspettare", "salire", "scendere", "biglietto", ("stazione", "stazione ferroviaria")]),
        ("salute", "Salute", ["medico", "dottore", "ospedale", "dolore", "farmacia", "aiuto", "paura", "calmo", "adulto"]),
        ("tempo_libero", "Tempo libero", ["parco", "biblioteca", "piscina", "museo", "giocare", "leggere", "nuotare", "guardare", ("insieme", "unire")]),
    ]))

    stories = [
        ("gen3_storia_volpe_uva", "La volpe e l'uva", "Una volpe cerca un grappolo d'uva.", [{"title": "La volpe", "hero": "volpe", "words": ["volpe", "vedere", "uva"], "sentence": "La volpe vede un grappolo d'uva."}, {"title": "La volpe salta", "hero": "uva", "words": ["volpe", "saltare", "uva"], "sentence": "La volpe salta per prendere l'uva."}, {"title": "La volpe va via", "hero": "volpe", "words": ["volpe", "andare", "casa"], "sentence": "L'uva è troppo alta e la volpe torna a casa."}]),
        ("gen3_storia_riccio_mela", "Il riccio e la mela", "Un riccio trova una mela nel bosco.", [{"title": "La mela", "hero": "riccio", "words": ["riccio", "trovare", "mela"], "sentence": "Il riccio trova una mela sotto un albero."}, {"title": "Il riccio porta", "hero": "mela", "words": ["riccio", "portare", "casa"], "sentence": "Il riccio porta la mela verso casa."}, {"title": "La merenda", "hero": "riccio", "words": ["riccio", "mangiare", "mela"], "sentence": "Il riccio mangia la mela con calma."}]),
        ("gen3_storia_nuvola_sole", "La nuvola e il sole", "Una nuvola copre il sole e poi lo lascia passare.", [{"title": "Il sole", "hero": "sole", "words": ["sole", "nuvola", "cielo"], "sentence": "Il sole splende nel cielo."}, {"title": "La nuvola", "hero": "nuvola", "words": ["nuvola", "coprire", "sole"], "sentence": "Una nuvola copre il sole per un momento."}, {"title": "Di nuovo il sole", "hero": "sole", "words": ["nuvola", "andare", "sole"], "sentence": "La nuvola va via e il sole torna."}]),
        ("gen3_storia_rana_stagno", "La rana e lo stagno", "Una rana salta nello stagno.", [{"title": "Lo stagno", "hero": "rana", "words": ["rana", "stagno", "acqua"], "sentence": "La rana vive vicino allo stagno."}, {"title": "La rana salta", "hero": "acqua", "words": ["rana", "saltare", "acqua"], "sentence": "La rana salta nell'acqua."}, {"title": "La rana aspetta", "hero": "rana", "words": ["rana", "aspettare", "farfalla"], "sentence": "La rana aspetta un insetto sul sasso."}]),
        ("gen3_storia_robot_gatto", "Il robot e il gatto", "Un robot incontra un gatto curioso.", [{"title": "Il robot", "hero": "robot", "words": ["robot", "vedere", "gatto"], "sentence": "Il robot vede un gatto sul tavolo."}, {"title": "Il gatto guarda", "hero": "gatto", "words": ["gatto", "guardare", "robot"], "sentence": "Il gatto guarda il robot con curiosità."}, {"title": "Insieme", "hero": "robot", "words": ["robot", "gatto", "giocare"], "sentence": "Il robot e il gatto giocano con una palla."}]),
        ("gen3_storia_lumaca_pioggia", "La lumaca e la pioggia", "Una lumaca esce dopo la pioggia.", [{"title": "La pioggia", "hero": "pioggia", "words": ["pioggia", "cadere", "giardino"], "sentence": "La pioggia cade sul giardino."}, {"title": "La lumaca", "hero": "lumaca", "words": ["lumaca", "uscire", "pioggia"], "sentence": "Dopo la pioggia esce una lumaca."}, {"title": "La lumaca cammina", "hero": "lumaca", "words": ["lumaca", "camminare", "foglia"], "sentence": "La lumaca cammina piano su una foglia."}]),
        ("gen3_storia_cavallo_stella", "Il cavallo e la stella", "Un cavallo guarda le stelle prima di dormire.", [{"title": "La notte", "hero": "cavallo", "words": ["cavallo", "notte", "stella"], "sentence": "Il cavallo guarda le stelle nella notte."}, {"title": "La stella", "hero": "stella", "words": ["stella", "brillante", "cielo"], "sentence": "Una stella brilla sopra il campo."}, {"title": "Il riposo", "hero": "cavallo", "words": ["cavallo", "dormire", "notte"], "sentence": "Il cavallo si riposa e dorme."}]),
    ]
    for args in stories:
        specs.append(caa_story(*args))

    socials = [
        ("gen3_sociale_taglio", "Il taglio dei capelli", "Prepararsi a un taglio di capelli.", "Oggi andrò dal parrucchiere.", [["Entrerò nel negozio con un adulto.", "Potrò sedermi e aspettare il mio turno."], ["Il parrucchiere userà forbici e pettine.", "Potrebbe chiedermi come voglio i capelli."], ["Posso sentire rumori o solletico.", "Posso chiedere una pausa o dire che cosa mi aiuta."], ["Il taglio finisce e poi tornerò a casa.", "Posso guardarmi allo specchio e raccontare come mi sento."]], "Mostrare prima gli strumenti e concordare una pausa. Non obbligare al contatto fisico senza preavviso."),
        ("gen3_sociale_mensa", "La mensa affollata", "Capire che cosa succede in mensa.", "Oggi mangerò in mensa.", [["In mensa ci sono molti bambini e rumori.", "Farò la fila e prenderò il vassoio."], ["Posso scegliere dove sedermi.", "Posso chiedere aiuto se non trovo il posto."], ["Il rumore può essere forte.", "Posso usare le cuffie o chiedere una pausa."], ["Dopo il pranzo tornerò in classe.", "Posso parlare con un adulto se qualcosa non va."]], "Concordare un posto tranquillo e un adulto di riferimento. Non forzare a mangiare tutto."),
        ("gen3_sociale_insegnante", "Cambia l'insegnante", "Anticipare il cambio di insegnante.", "Domani in classe ci sarà un'altra insegnante.", [["La mia insegnante sarà assente.", "Un'altra insegnante seguirà la classe."], ["Le regole della classe restano le stesse.", "Posso chiedere il nome e dove trovare i materiali."], ["Posso sentirmi incerto.", "Posso guardare l'agenda e chiedere aiuto."], ["La giornata continua e poi torna la mia insegnante.", "Posso raccontare come è andata."]], "Mostrare una foto o il nome della persona. Mantenere le routine e segnalare i cambiamenti prevedibili."),
        ("gen3_sociale_supermercato", "Al supermercato", "Prepararsi a fare la spesa.", "Oggi andrò al supermercato.", [["Entrerò con un adulto e prenderò un carrello.", "Cercherò gli oggetti della lista."], ["Ci saranno molte persone e rumori.", "Posso restare vicino all'adulto e chiedere una pausa."], ["Alla cassa aspetterò il mio turno.", "Posso aiutare a mettere gli oggetti nel sacchetto."], ["Alla fine tornerò a casa.", "Posso raccontare che cosa abbiamo comprato."]], "Preparare una lista con immagini e concordare un punto di ritrovo. Evitare lunghe attese non prevedibili."),
        ("gen3_sociale_parco_affollato", "Il parco affollato", "Che cosa fare quando il parco è pieno.", "Oggi andrò al parco.", [["Al parco potrebbero esserci molti bambini.", "Dovrò aspettare il mio turno per lo scivolo."], ["Posso scegliere un gioco più libero.", "Posso chiedere una pausa se c'è troppo rumore."], ["Posso invitare un bambino a giocare.", "Posso dire no se non voglio essere toccato."], ["Quando è ora di andare via saluterò.", "Posso raccontare che cosa mi è piaciuto."]], "Concordare un segnale per la pausa e un limite di tempo. Non obbligare a giocare con tutti."),
        ("gen3_sociale_piscina", "La piscina", "Anticipare una lezione in piscina.", "Oggi andrò in piscina.", [["Mi cambierò nello spogliatoio.", "Farò la doccia prima di entrare."], ["L'acqua può essere fredda e il rumore forte.", "Posso entrare piano e chiedere una pausa."], ["L'istruttore spiegherà gli esercizi.", "Posso guardare prima di provare."], ["Dopo la lezione mi asciugherò e mi cambierò.", "Posso raccontare che cosa ho imparato."]], "Visitare prima gli spazi e concordare un adulto di riferimento. Rispettare i tempi di adattamento all'acqua."),
        ("gen3_sociale_dentista", "La visita dal dentista", "Prepararsi a una visita dentistica.", "Oggi andrò dal dentista.", [["In sala d'attesa aspetterò il mio turno.", "Il dentista mi chiederà di aprire la bocca."], ["Userà strumenti piccoli e una luce.", "Posso chiedere di vedere gli strumenti prima."], ["Posso sentire rumori o fastidio.", "Posso alzare la mano per una pausa."], ["La visita dura poco e poi tornerò a casa.", "Posso raccontare come è andata."]], "Concordare un segnale per la pausa e chiedere spiegazioni semplici. Non descrivere la visita come dolorosa se non necessario."),
    ]
    for args in socials:
        specs.append(social_story(*args))

    specs.append(agenda_weekly("gen3_agenda_settimana_secondaria", "Settimana della secondaria", "Cinque giorni con attività e studio.", [("scuola", "scuola"), ("studio", "leggere"), ("pranzo", "pranzo"), ("compiti", "compiti")]))
    specs.append(agenda_daily("gen3_agenda_mattina", "La routine del mattino", "Sequenza dalla sveglia all'uscita.", [("7:00", "sveglia", "svegliare"), ("7:10", "vestirsi", "vestirsi"), ("7:25", "colazione", "far colazione"), ("7:45", "lavarsi", "lavare"), ("8:00", "zaino", "zaino"), ("8:10", "uscita", "andare")]))
    specs.append(agenda_daily("gen3_agenda_sera", "La routine della sera", "Sequenza dalla cena al riposo.", [("18:30", "cena", "cena"), ("19:00", "gioco", "giocare"), ("19:30", "lavarsi", "lavare"), ("20:00", "pigiama", "vestirsi"), ("20:15", "letto", "dormire"), ("20:30", "luci spente", "dormire")]))
    specs.append(agenda_first_then("gen3_agenda_prima_dopo_scuola", "Prima scuola, poi casa", "Transizione dalla scuola a casa.", "scuola", "casa"))
    specs.append(agenda_checklist("gen3_checklist_zaino", "Preparo lo zaino", "Checklist per preparare lo zaino la sera.", ["Metto il quaderno", "Metto i libri", "Metto la merenda", "Metto la borraccia", "Controllo la lista", "Chiudo lo zaino"]))
    specs.append(agenda_daily("gen3_agenda_transizioni", "Cambio di attività", "Cinque passi per una transizione prevedibile.", [("1", "avviso", "campana"), ("2", "timer", "orologio"), ("3", "finire", "finire"), ("4", "riporre", "ordinare"), ("5", "iniziare", "giocare")]))
    specs.append(agenda_checklist("gen3_checklist_vacanza", "Checklist della vacanza", "Cose da preparare prima di partire.", ["Scegliere i vestiti", "Preparare i farmaci", "Mettere il caricabatterie", "Preparare il libro", "Controllare i documenti"]))

    specs.append(teacch_sequence("gen3_teacch_vestirsi", "Sequenza: vestirsi", "Cinque passi per vestirsi da soli.", [("maglietta", "Prendo la maglietta."), ("pantaloni", "Metto i pantaloni."), ("calze", "Metto le calze."), ("scarpe", "Metto le scarpe."), ("giacca", "Prendo la giacca.")]))
    specs.append(teacch_sequence("gen3_teacch_cucinare", "Sequenza: preparare la merenda", "Passi per preparare una merenda semplice.", [("mani", "Mi lavo le mani."), ("pane", "Prendo il pane."), ("formaggio", "Metto il formaggio."), ("piatto", "Metto sul piatto."), ("ordinare", "Riordino.")]))
    specs.append(teacch_matching("gen3_teacch_abbinamento_cibo", "Abbina i cibi", "Coppie di cibi uguali.", [("mela", "mela"), ("banana", "banana"), ("pane", "pane"), ("latte", "latte"), ("formaggio", "formaggio")]))
    specs.append(teacch_sorting("gen3_teacch_classifica_colori", "Classifica i colori", "Rosso, blu e verde.", [("Rosso", ["mela", "pomodoro", "cuore"]), ("Blu", ["acqua", "cielo", "palla"]), ("Verde", ["albero", "foglia", "insalata"])]))
    specs.append(teacch_cards("gen3_teacch_cartellini_igiene", "Cartellini dell'igiene", "Cartellini da ritagliare per la routine.", ["lavare", "mani", "viso", "denti", "spazzolino", "sapone", "asciugare", "bagno", "doccia", "pettine", "vestirsi", "riposo"]))
    specs.append(teacch_sequence("gen3_teacch_compito_casa", "Sequenza: compito a casa", "Sistema di lavoro per i compiti.", [("zaino", "Prendo il quaderno."), ("leggere", "Leggo la consegna."), ("scrivere", "Faccio il primo esercizio."), ("controllare", "Controllo."), ("ordinare", "Metto via.")]))
    specs.append(teacch_sorting("gen3_teacch_classifica_animali", "Classifica gli animali", "Mammiferi, uccelli e pesci.", [("Mammiferi", ["cane", "gatto", "cavallo"]), ("Uccelli", ["rondine", "gufo", "pinguino"]), ("Pesci", ["squalo", "trota", "pesce"])]))

    specs.append(token_board("gen3_token_tre", "Token board: tre gettoni", "Tre gettoni per un obiettivo breve.", 3))
    specs.append(choice_board("gen3_scelta_quattro", "Scelgo tra quattro attività", "Quattro alternative visive.", ["giocare", "leggere", "disegnare", "musica"], 4))
    specs.append(choice_board("gen3_scelta_pausa", "Chiedo una pausa", "Due modi per chiedere una pausa.", ["riposo", "acqua"], 2))
    specs.append(emotion_scale("gen3_autovalutazione", "Come ho lavorato?", "Autovalutazione con cinque livelli.", True))
    specs.append(break_card("gen3_rientro", "Rientro dopo la pausa", "Carta per riprendere l'attività.", True))

    specs.append(language_sheet("gen3_ita_digrammi", "Digrammi e trigrammi", "Completa parole con GN, GL, SC.", ["gnomo", "aglio", "pesce", "scivolo", "bagno", "ragno"], lambda w: w[0] + "___" + w[-1], "Completa la parola con il digramma giusto.", True))
    specs.append(language_sheet("gen3_ita_ordine_frase", "Riordina la frase", "Metti le parole nell'ordine giusto.", ["cane", "mangiare", "mela", "casa", "scuola", "amico"], lambda w: "___", "Scrivi una frase con la parola.", True))
    specs.append(language_sheet("gen3_ita_sinonimi", "Sinonimi", "Trova una parola con lo stesso significato.", ["grande", "felice", "veloce", "bello", "buono", "caldo"], lambda w: "sinonimo di " + w, "Scrivi un sinonimo per ogni parola.", True))

    specs.append(math_sheet("gen3_mate_moltiplicazioni", "Moltiplicazioni del 2 e del 5", "Otto moltiplicazioni semplici.", [(2, "×", 1, 2), (2, "×", 2, 4), (2, "×", 3, 6), (5, "×", 1, 5), (5, "×", 2, 10), (5, "×", 3, 15), (2, "×", 4, 8), (5, "×", 4, 20)], "Completa le moltiplicazioni.", "MATEMATICA"))
    specs.append(counting_sheet("gen3_mate_conteggio20", "Conteggio entro il 20", "Conta e scrivi il numero.", [10, 12, 15, 18, 20, 14, 16, 11]))
    specs.append(math_sheet("gen3_mate_calcolo_misto", "Calcolo misto", "Addizioni e sottrazioni entro il 10.", [(3, "+", 4, 7), (8, "-", 2, 6), (5, "+", 5, 10), (9, "-", 4, 5), (6, "+", 3, 9), (10, "-", 7, 3)], "Completa le operazioni.", "MATEMATICA"))

    specs.append(science_sheet("gen3_scienze_piante", "Le parti della pianta", "Radici, fusto, foglie e fiore.", [("Radici", "#ecfdf5"), ("Fusto e foglie", "#eff6ff"), ("Fiori", "#f0fdf4")], ["radice", "fusto", "foglia", "fiore", "albero", "erba", "margherita", "rosa", "seme"], True))
    specs.append(language_sheet("gen3_inglese_colori", "English: colours", "Abbina il colore alla parola inglese.", ["red", "blue", "green", "yellow", "orange", "purple"], lambda w: "colour: ___", "Scrivi il colore in inglese.", True, False))
    specs.append(language_sheet("gen3_storia_tempo", "Prima e dopo", "Metti gli eventi in ordine.", ["sveglia", "colazione", "scuola", "pranzo", "casa", "letto"], lambda w: "___", "Scrivi il numero d'ordine.", True))
    specs.append(language_sheet("gen3_geografia_ambienti", "Ambienti e paesaggi", "Abbina l'ambiente agli elementi.", ["montagna", "mare", "città", "campagna", "fiume", "deserto"], lambda w: "___", "Scrivi un elemento del paesaggio.", True))
    specs.append(language_sheet("gen3_musica_ritmo", "Ritmo e body percussion", "Riproduci la sequenza.", ["mani", "piedi", "gambe", "dita", "pancia", "testa"], lambda w: "tocca: " + w, "Osserva il ritmo e riproduci la sequenza.", True))

    specs.append(game_cards("gen3_tombola_cibo", "Tombola del cibo", "Cartelle con nove cibi.", ["mela", "banana", "pane", "pasta", "latte", "acqua", "formaggio", "biscotto", "succo"], "tombola"))
    specs.append(game_cards("gen3_domino_animali", "Domino degli animali", "Tessere con animali.", ["cane", "gatto", "uccello", "pesce", "cavallo", "mucca", "pecora", "topo", "farfalla"], "domino"))
    specs.append(game_cards("gen3_sequenze_storia", "Sequenze di storia", "Metti in ordine le vignette.", ["sveglia", "colazione", "scuola", "pranzo", "giocare", "letto"], "sequenze"))
    specs.append(game_cards("gen3_etichette_scuola", "Etichette per la scuola", "Etichette per materiali e spazi.", ["quaderno", "matita", "zaino", "aula", "biblioteca", "palestra", "bagno", "giardino", "mensa"], "etichette", kind="printable", interactive=False))
    specs.append(number_poster("gen3_poster_numeri", "Poster dei numeri", "Numeri da 1 a 9 con parole."))

    for spec in specs:
        if spec["id"].startswith("gen3_etichette"):
            spec["category"] = "Materiali stampabili"
            spec["kind"] = "printable"
            spec["taxonomy"].update({"category_id": "stampabili", "subcategory_id": "etichette"})
        elif spec["id"].startswith("gen3_poster"):
            spec["category"] = "Materiali stampabili"
            spec["kind"] = "printable"
            spec["taxonomy"].update({"category_id": "stampabili", "subcategory_id": "poster"})
        save(spec)
    print(f"{len(specs)} specifiche del terzo lotto scritte in {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
