#!/usr/bin/env python3
"""Quarto lotto: completa il piano di 180 modelli nuovi (124 + 56)."""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
from generate_library_batch2 import (  # noqa: E402
    agenda_weekly,
    caa_story,
    counting_sheet,
    game_cards,
    language_sheet,
    math_sheet,
    social_story,
    teacch_cards,
    teacch_matching,
    teacch_sequence,
    teacch_sorting,
)

OUT = ROOT / "templates" / "sources" / "templates" / "lotto4"


def save(spec):
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / f"{spec['id']}.json").write_text(json.dumps(spec, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def main():
    specs = []

    # 1 storia CAA, 1 storia sociale, 1 agenda
    specs.append(caa_story("gen4_storia_cane_smarrito", "Il cane smarrito", "Un cane si allontana e poi ritrova il suo bambino.", [
        {"title": "Il cane esce", "hero": "cane", "words": ["cane", "uscire", "giardino"], "sentence": "Il cane esce dal giardino."},
        {"title": "Il cane cammina", "hero": "strada", "words": ["cane", "camminare", "strada"], "sentence": "Il cane cammina lungo la strada."},
        {"title": "Il bambino cerca", "hero": "bambino", "words": ["bambino", "cercare", "cane"], "sentence": "Il bambino cerca il cane con la mamma."},
        {"title": "Il ritorno", "hero": "casa", "words": ["cane", "andare", "casa"], "sentence": "Il cane torna a casa e il bambino lo abbraccia."},
    ]))
    specs.append(social_story("gen4_sociale_chiedere_pausa", "Chiedere una pausa", "Come chiedere una pausa durante un'attività.", "A volte ho bisogno di fermarmi.", [
        ["Durante un'attività posso sentirmi stanco o sovraccarico.", "Il mio corpo può darmi segnali: respiro veloce, tensione, voglia di muovermi."],
        ["Posso chiedere una pausa a un adulto.", "Posso usare una carta, un gesto o dire «pausa»."],
        ["La pausa può durare qualche minuto.", "Posso bere, respirare o stare in un posto tranquillo."],
        ["Quando sto meglio posso tornare all'attività.", "Posso chiedere aiuto per riprendere dal punto giusto."],
    ], "La pausa non è un premio né una punizione: è uno strumento di autoregolazione. Concordare durata e luogo con l'adulto, senza obbligare a nascondere il disagio."))
    specs.append(agenda_weekly("gen4_agenda_extrascolastiche", "Settimana con attività extrascolastiche", "Cinque giorni con scuola e attività pomeridiane.", [("scuola", "scuola"), ("pranzo", "pranzo"), ("musica", "musica"), ("sport", "correre")]))

    # 6 TEACCH
    specs.append(teacch_matching("gen4_teacch_forme", "Abbina le forme", "Coppie di forme geometriche uguali.", [("cerchio", "cerchio"), ("quadrato", "quadrato"), ("triangolo", "triangolo"), ("rettangolo", "rettangolo"), ("rombo", "rombo")]))
    specs.append(teacch_sorting("gen4_teacch_vestiti", "Classifica i vestiti", "Vestiti per la scuola, la casa e lo sport.", [("Scuola", ["maglietta", "pantaloni", "scarpe"]), ("Casa", ["pigiama", "calze", "maglietta"]), ("Sport", ["scarpe", "maglietta", "pantaloni"])]))
    specs.append(teacch_sequence("gen4_teacch_apparecchiare", "Sequenza: apparecchiare", "Passi per apparecchiare la tavola.", [("piatto", "Metto il piatto."), ("bicchiere", "Metto il bicchiere."), ("forchetta", "Metto la forchetta."), ("tovaglia", "Metto il tovagliolo."), ("ordinare", "Controllo e riordino.")]))
    specs.append(teacch_sequence("gen4_teacch_igiene_denti", "Sequenza: lavare i denti", "Passi per l'igiene orale.", [("spazzolino", "Prendo lo spazzolino."), ("dentifricio", "Metto il dentifricio."), ("denti", "Spazzolo i denti."), ("acqua", "Risciacquo."), ("asciugare", "Asciugo e riordino.")]))
    specs.append(teacch_cards("gen4_teacch_emozioni", "Cartellini delle emozioni", "Cartellini da ritagliare per parlare delle emozioni.", ["felice", "triste", "arrabbiato", "preoccupato", "calmo", "stanco", "sorpreso", "spaventato", "bene", "abbraccio", "aiuto", "riposo"]))
    specs.append(teacch_sequence("gen4_teacch_sistema_lavoro", "Sistema di lavoro", "Che cosa fare, quanto lavoro, come finire, cosa dopo.", [("leggere", "Leggo la consegna."), ("fare", "Faccio il primo passo."), ("controllare", "Controllo il lavoro."), ("ordinare", "Metto a posto."), ("giocare", "Scelgo la pausa.")]))

    # 11 Italiano
    specs.append(language_sheet("gen4_ita_vocali_iniziali", "Vocali iniziali", "Riconosci la vocale che apre la parola.", ["ape", "elefante", "isola", "oca", "uva", "albero"], lambda w: "___" + w[1:], "Scrivi la vocale iniziale.", True))
    specs.append(language_sheet("gen4_ita_consonanti", "Consonanti iniziali", "Riconosci la consonante iniziale.", ["cane", "sole", "mela", "luna", "pesce", "topo"], lambda w: "___" + w[1:], "Scrivi la consonante iniziale.", True))
    specs.append(language_sheet("gen4_ita_sillabe_inverse", "Sillabe inverse", "Completa parole con sillabe inverse.", ["albero", "orso", "arco", "erba", "isola", "uva"], lambda w: w[:2] + "___", "Scrivi la sillaba che manca.", True))
    specs.append(language_sheet("gen4_ita_trisillabe", "Parole trisillabe", "Dividi le parole in sillabe.", ["banana", "matita", "telefono", "farfalla", "compito", "musica"], lambda w: w[:3] + "___", "Scrivi la sillaba che manca.", True))
    specs.append(language_sheet("gen4_ita_digrammi", "Digrammi GN e GL", "Completa parole con GN e GL.", ["gnomo", "aglio", "bagno", "ragno", "maglietta", "aglio"], lambda w: w[0] + "___" + w[-1], "Completa con il digramma giusto.", True))
    specs.append(language_sheet("gen4_ita_cu_qu", "CU e QU", "Scegli la grafia corretta.", ["cuore", "quaderno", "quattro", "cucina", "quercia", "scuola"], lambda w: "___", "Scrivi la parola corretta.", True, False))
    specs.append(language_sheet("gen4_ita_completamento", "Completa le parole", "Aggiungi la lettera o la sillaba mancante.", ["casa", "sole", "mela", "luna", "pane", "topo"], lambda w: w[:-1] + "___", "Completa la parola.", True))
    specs.append(language_sheet("gen4_ita_plurale", "Singolare e plurale", "Trasforma le parole.", ["cane", "gatto", "mela", "libro", "amico", "casa"], lambda w: w + " → ___", "Scrivi il plurale.", True))
    specs.append(language_sheet("gen4_ita_grammatica", "Nomi e verbi", "Riconosci nomi e verbi.", ["cane", "correre", "mela", "mangiare", "scuola", "leggere"], lambda w: "nome o verbo: ___", "Scrivi se è un nome o un verbo.", True))
    specs.append(language_sheet("gen4_ita_produzione", "Produzione scritta", "Scrivi una frase con la parola data.", ["cane", "scuola", "mela", "casa", "amico", "giocare"], lambda w: "frase con: " + w, "Scrivi una frase per ogni parola.", True))
    specs.append(language_sheet("gen4_ita_lettura_caa", "Lettura in CAA", "Leggi le parole e ricopiale.", ["mamma", "papà", "casa", "scuola", "amico", "giocare"], lambda w: "___", "Leggi e ricopia la parola.", True))

    # 12 Matematica
    specs.append(counting_sheet("gen4_mate_conteggio5", "Conteggio entro il 5", "Conta e scrivi il numero.", [1, 2, 3, 4, 5, 3, 2, 4]))
    specs.append(counting_sheet("gen4_mate_conteggio10", "Conteggio entro il 10", "Conta e scrivi il numero.", [4, 6, 8, 9, 3, 7, 5, 10]))
    specs.append(math_sheet("gen4_mate_add10", "Addizioni entro il 10", "Otto addizioni entro dieci.", [(2, "+", 1, 3), (3, "+", 3, 6), (4, "+", 2, 6), (5, "+", 4, 9), (1, "+", 7, 8), (6, "+", 2, 8), (2, "+", 6, 8), (4, "+", 5, 9)], "Completa le addizioni.", "MATEMATICA"))
    specs.append(math_sheet("gen4_mate_sot10", "Sottrazioni entro il 10", "Otto sottrazioni entro dieci.", [(5, "-", 2, 3), (7, "-", 3, 4), (9, "-", 4, 5), (8, "-", 1, 7), (6, "-", 2, 4), (10, "-", 5, 5), (4, "-", 3, 1), (7, "-", 6, 1)], "Completa le sottrazioni.", "MATEMATICA"))
    specs.append(math_sheet("gen4_mate_add20", "Addizioni entro il 20", "Otto addizioni entro venti.", [(8, "+", 4, 12), (9, "+", 6, 15), (7, "+", 7, 14), (10, "+", 5, 15), (6, "+", 9, 15), (12, "+", 4, 16), (11, "+", 3, 14), (13, "+", 5, 18)], "Completa le addizioni.", "MATEMATICA"))
    specs.append(math_sheet("gen4_mate_sot20", "Sottrazioni entro il 20", "Otto sottrazioni entro venti.", [(15, "-", 5, 10), (18, "-", 6, 12), (14, "-", 4, 10), (20, "-", 8, 12), (17, "-", 9, 8), (16, "-", 7, 9), (13, "-", 3, 10), (19, "-", 10, 9)], "Completa le sottrazioni.", "MATEMATICA"))
    specs.append(math_sheet("gen4_mate_molti2", "Moltiplicazioni del 2", "Otto moltiplicazioni del 2.", [(2, "×", 1, 2), (2, "×", 2, 4), (2, "×", 3, 6), (2, "×", 4, 8), (2, "×", 5, 10), (2, "×", 6, 12), (2, "×", 7, 14), (2, "×", 8, 16)], "Completa le moltiplicazioni.", "MATEMATICA"))
    specs.append(math_sheet("gen4_mate_molti5", "Moltiplicazioni del 5", "Otto moltiplicazioni del 5.", [(5, "×", 1, 5), (5, "×", 2, 10), (5, "×", 3, 15), (5, "×", 4, 20), (5, "×", 5, 25), (5, "×", 6, 30), (5, "×", 7, 35), (5, "×", 8, 40)], "Completa le moltiplicazioni.", "MATEMATICA"))
    specs.append(math_sheet("gen4_mate_div2", "Divisioni per 2", "Otto divisioni per due.", [(2, "÷", 2, 1), (4, "÷", 2, 2), (6, "÷", 2, 3), (8, "÷", 2, 4), (10, "÷", 2, 5), (12, "÷", 2, 6), (14, "÷", 2, 7), (16, "÷", 2, 8)], "Completa le divisioni.", "MATEMATICA"))
    specs.append(language_sheet("gen4_mate_forme", "Forme geometriche", "Riconosci e nomina le forme.", ["cerchio", "quadrato", "triangolo", "rettangolo", "rombo", "ovale"], lambda w: "nome: ___", "Scrivi il nome della forma.", True, True, "Matematica"))
    specs.append(language_sheet("gen4_mate_frazioni", "Frazioni semplici", "Metà, terzo, quarto e intero.", ["intero", "metà", "terzo", "quarto", "quinto", "decimo"], lambda w: "frazione: ___", "Scrivi la frazione.", True, False, "Matematica"))
    specs.append(language_sheet("gen4_mate_misure", "Misure e grandezze", "Lungo, corto, pesante, leggero.", ["lungo", "corto", "pesante", "leggero", "alto", "basso"], lambda w: "misura: ___", "Scrivi la parola che descrive.", True, True, "Matematica"))

    # 13 altre discipline
    specs.append(language_sheet("gen4_eng_colori", "English: colours", "Colori in inglese.", ["red", "blue", "green", "yellow", "orange", "purple"], lambda w: "colour: ___", "Scrivi il colore in inglese.", True, False, "Inglese"))
    specs.append(language_sheet("gen4_eng_animali", "English: animals", "Animali in inglese.", ["dog", "cat", "bird", "fish", "horse", "cow"], lambda w: "animal: ___", "Scrivi l'animale in inglese.", True, False, "Inglese"))
    specs.append(language_sheet("gen4_eng_numeri", "English: numbers", "Numeri da uno a sei.", ["one", "two", "three", "four", "five", "six"], lambda w: "number: ___", "Scrivi il numero in inglese.", True, False, "Inglese"))
    specs.append(language_sheet("gen4_eng_scuola", "English: school objects", "Oggetti di scuola in inglese.", ["book", "pencil", "bag", "desk", "chair", "ruler"], lambda w: "object: ___", "Scrivi l'oggetto in inglese.", True, False, "Inglese"))
    specs.append(language_sheet("gen4_scienze_corpo", "Il corpo umano", "Parti del corpo e funzioni.", ["testa", "mano", "piede", "occhio", "orecchio", "bocca"], lambda w: "parte: ___", "Scrivi il nome della parte del corpo.", True, True, "Scienze"))
    specs.append(language_sheet("gen4_scienze_materiali", "Materiali e proprietà", "Legno, vetro, plastica, metallo.", ["legno", "vetro", "plastica", "metallo", "carta", "stoffa"], lambda w: "materiale: ___", "Scrivi il materiale.", True, False, "Scienze"))
    specs.append(language_sheet("gen4_scienze_ciclo", "Il ciclo della pianta", "Seme, germoglio, pianta, fiore.", ["seme", "pianta", "pianta", "fiore", "fiore", "radice"], lambda w: "fase: ___", "Scrivi la fase del ciclo.", True, False, "Scienze"))
    specs.append(language_sheet("gen4_storia_prima_dopo", "Prima e dopo", "Metti gli eventi in ordine.", ["sveglia", "colazione", "scuola", "pranzo", "casa", "letto"], lambda w: "___", "Scrivi il numero d'ordine.", True, True, "Storia"))
    specs.append(language_sheet("gen4_storia_fonti", "Le fonti storiche", "Riconosci le fonti.", ["foto", "lettera", "moneta", "libro", "oggetto", "racconto"], lambda w: "fonte: ___", "Scrivi che tipo di fonte è.", True, False, "Storia"))
    specs.append(language_sheet("gen4_geo_ambienti", "Ambienti e paesaggi", "Montagna, mare, città, campagna.", ["montagna", "mare", "città", "campagna", "fiume", "deserto"], lambda w: "ambiente: ___", "Scrivi il nome dell'ambiente.", True, True, "Geografia"))
    specs.append(language_sheet("gen4_geo_orientamento", "Orientamento", "Nord, sud, est, ovest.", ["nord", "sud", "est", "ovest", "mappa", "bussola"], lambda w: "___", "Scrivi la direzione.", True, True, "Geografia"))
    specs.append(language_sheet("gen4_musica_strumenti", "Strumenti musicali", "Riconosci gli strumenti.", ["pianoforte", "chitarra", "violino", "flauto", "tamburo", "tromba"], lambda w: "strumento: ___", "Scrivi il nome dello strumento.", True, True, "Musica"))
    specs.append(language_sheet("gen4_civica_regole", "Regole condivise", "Le regole della classe.", ["ascoltare", "aspettare", "aiutare", "rispettare", "condividere", "ordinare"], lambda w: "regola: ___", "Scrivi la regola.", True, True, "Educazione civica"))
    specs.append(language_sheet("gen4_tecnologia_coding", "Coding unplugged", "Sequenze e istruzioni.", ["avanti", "andare", "destra", "sinistra", "saltare", "fermare"], lambda w: "istruzione: ___", "Scrivi l'istruzione.", True, True, "Tecnologia"))

    # 7 giochi
    specs.append(game_cards("gen4_memory_scuola", "Memory della scuola", "Quattro coppie di oggetti di scuola.", ["quaderno", "quaderno", "matita", "matita", "zaino", "zaino", "libro", "libro"], "memory"))
    specs.append(game_cards("gen4_memory_cibo", "Memory del cibo", "Quattro coppie di cibi.", ["mela", "mela", "banana", "banana", "pane", "pane", "latte", "latte"], "memory"))
    specs.append(game_cards("gen4_tombola_animali", "Tombola degli animali", "Cartelle con nove animali.", ["cane", "gatto", "uccello", "pesce", "cavallo", "mucca", "pecora", "topo", "farfalla"], "tombola"))
    specs.append(game_cards("gen4_domino_scuola", "Domino della scuola", "Tessere con oggetti di scuola.", ["scuola", "maestra", "quaderno", "matita", "zaino", "intervallo", "pranzo", "compagno", "aula"], "domino"))
    specs.append(game_cards("gen4_puzzle_casa", "Puzzle della casa", "Quattro parti da ricomporre.", ["casa", "cucina", "camera da letto", "bagno", "divano", "letto", "finestra", "porta"], "puzzle"))
    specs.append(game_cards("gen4_associazioni", "Associazioni di oggetti", "Abbina oggetto e uso.", ["matita", "quaderno", "forchetta", "piatto", "spazzolino", "denti", "sapone", "mani", "zaino", "scuola"], "associazioni"))
    specs.append(game_cards("gen4_ascolto", "Attività di ascolto", "Ascolta l'indizio e scegli.", ["cane", "gatto", "uccello", "pesce", "cavallo", "mucca", "pecora", "topo", "farfalla"], "ascolto"))

    # 4 stampabili
    specs.append(game_cards("gen4_flashcard_scuola", "Flashcard: scuola", "Nove flashcard della scuola.", ["scuola", "maestra", "quaderno", "matita", "zaino", "intervallo", "pranzo", "compagno", "aula"], "flashcard", kind="printable", interactive=False))
    specs.append(game_cards("gen4_etichette_cibo", "Etichette del cibo", "Etichette per la cucina.", ["mela", "banana", "pane", "pasta", "latte", "acqua", "formaggio", "biscotto", "succo"], "etichette", kind="printable", interactive=False))
    specs.append(game_cards("gen4_poster_regole", "Poster delle regole", "Nove regole della classe.", ["ascoltare", "aspettare", "aiutare", "rispettare", "condividere", "ordinare", "parlare", "giocare", "unire"], "poster", kind="printable", interactive=False))
    specs.append(game_cards("gen4_striscia_visiva", "Striscia visiva della giornata", "Sequenza da appendere.", ["sveglia", "colazione", "scuola", "pranzo", "giocare", "compiti", "cena", "letto", "riposo"], "strisce", kind="printable", interactive=False))

    for spec in specs:
        if spec["id"].startswith("gen4_etichette"):
            spec["category"] = "Materiali stampabili"
            spec["kind"] = "printable"
            spec["taxonomy"].update({"category_id": "stampabili", "subcategory_id": "etichette"})
        elif spec["id"].startswith("gen4_poster"):
            spec["category"] = "Materiali stampabili"
            spec["kind"] = "printable"
            spec["taxonomy"].update({"category_id": "stampabili", "subcategory_id": "poster"})
        elif spec["id"].startswith("gen4_flashcard"):
            spec["category"] = "Materiali stampabili"
            spec["kind"] = "printable"
            spec["taxonomy"].update({"category_id": "stampabili", "subcategory_id": "flashcard"})
        elif spec["id"].startswith("gen4_striscia"):
            spec["category"] = "Materiali stampabili"
            spec["kind"] = "printable"
            spec["taxonomy"].update({"category_id": "stampabili", "subcategory_id": "strisce"})
        save(spec)
    print(f"{len(specs)} specifiche del quarto lotto scritte in {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
