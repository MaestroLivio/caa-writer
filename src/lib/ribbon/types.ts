// Registro dichiarativo del Ribbon.
//
// La stessa definizione alimenta il Ribbon, la barra di accesso rapido e i
// tooltip; i comandi eseguono le stesse azioni del resto dell'applicazione.
// Un comando non implementato non viene mostrato come pulsante attivo.

export type RibbonControlKind = "button" | "select" | "color";

export interface RibbonOption {
  value: string;
  label: string;
}

export interface RibbonCommand {
  id: string;
  label: string;
  icon?: string;
  shortcut?: string;
  tooltip?: string;
  kind?: RibbonControlKind;
  enabled?: boolean;
  active?: boolean;
  value?: string;
  options?: RibbonOption[];
  width?: string;
  /** Comandi primari: icona + etichetta. Secondari: icona compatta. */
  emphasis?: "primary" | "secondary";
  execute?: () => void;
  onChange?: (value: string) => void;
}

export interface RibbonGroup {
  id: string;
  label: string;
  commands: RibbonCommand[];
  /** Quanti comandi restano visibili prima del menu di overflow. */
  visible?: number;
  /**
   * `tiles` mostra i comandi come riquadri visivi grandi: icona in una tessera
   * CAA ed etichetta sotto. È il formato della galleria «Inserisci», pensato
   * per essere riconoscibile a colpo d'occhio, non un elenco di comandi.
   */
  layout?: "list" | "tiles";
}

export interface RibbonTab {
  id: string;
  label: string;
  groups: RibbonGroup[];
  contextual?: boolean;
  /** Se falso la scheda è visibile ma i comandi sono disabilitati. */
  enabled?: boolean;
}

export type RibbonMode = "full" | "compact" | "hidden";
