// WorkspaceLayoutManager: unico punto di verità per la disposizione
// dell'ambiente di lavoro. Preferenze globali, persistite con versione.

import { get, writable } from "svelte/store";

export type WorkspaceRibbonMode = "full" | "compact" | "hidden";
export type WorkspaceSidebarMode = "docked" | "overlay" | "collapsed";
export type WorkspacePanel = "simbolo" | "collezioni" | "impostazioni" | "studio" | "ia" | "documento";
export type AssociationScope = "occurrence" | "document" | "profile" | "global" | "vocabulary";

export interface WorkspaceLayout {
  version: 1;
  ribbonMode: WorkspaceRibbonMode;
  ribbonPinned: boolean;
  /// Nasconde l'intera barra superiore (documento + schede) in una riga
  /// sottile con i comandi rapidi. Indipendente da `ribbonMode`.
  topbarMinimized: boolean;
  sidebarMode: WorkspaceSidebarMode;
  /// Vero solo dopo una scelta esplicita dell'utente (pulsante del pannello o
  /// preset). Le installazioni precedenti ripartono dal pannello ancorato.
  sidebarOverlayChoice: boolean;
  sidebarWidth: number;
  sidebarLastWidth: number;
  activeSidebarPanel: WorkspacePanel;
  focusMode: boolean;
  fullscreen: boolean;
  quickAccess: string[];
  quickLabels: boolean;
  associationScope: AssociationScope;
  zoom: number;
  preset: string;
}

export const SIDEBAR_MIN = 260;
export const SIDEBAR_MAX = 640;
const STORAGE_KEY = "caa.workspace.v1";
const PRESETS_KEY = "caa.workspace.presets.v1";

export const defaultQuickAccess = ["save", "undo", "redo", "search", "listen", "export"];

export const defaultWorkspace: WorkspaceLayout = {
  version: 1,
  ribbonMode: "full",
  ribbonPinned: false,
  topbarMinimized: false,
  sidebarMode: "docked",
  sidebarOverlayChoice: false,
  sidebarWidth: 372,
  sidebarLastWidth: 372,
  activeSidebarPanel: "simbolo",
  focusMode: false,
  fullscreen: false,
  quickAccess: defaultQuickAccess,
  quickLabels: false,
  associationScope: "occurrence",
  zoom: 1,
  preset: "",
};

export function clampSidebarWidth(value: number): number {
  return Math.min(SIDEBAR_MAX, Math.max(SIDEBAR_MIN, Math.round(value)));
}

function loadWorkspace(): WorkspaceLayout {
  if (typeof localStorage === "undefined") return { ...defaultWorkspace };
  try {
    const raw = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "null") as Partial<WorkspaceLayout> | null;
    if (!raw || raw.version !== 1) return { ...defaultWorkspace };
    const legacyPanel = String(raw.activeSidebarPanel ?? "");
    const loaded: WorkspaceLayout = {
      ...defaultWorkspace,
      ...raw,
      version: 1,
      // I pannelli «Alternative» e «Cerca» sono stati unificati in «Simbolo».
      activeSidebarPanel:
        legacyPanel === "alternative" || legacyPanel === "cerca"
          ? "simbolo"
          : (raw.activeSidebarPanel ?? "simbolo"),
      sidebarWidth: clampSidebarWidth(Number(raw.sidebarWidth) || 372),
      sidebarLastWidth: clampSidebarWidth(Number(raw.sidebarLastWidth) || 372),
      quickAccess: Array.isArray(raw.quickAccess) && raw.quickAccess.length
        ? raw.quickAccess
        : defaultQuickAccess,
      zoom: Math.min(1.8, Math.max(0.5, Number(raw.zoom) || 1)),
    };
    // Migrazione: il vecchio default sovrapposto non è una scelta esplicita.
    if (!loaded.sidebarOverlayChoice && loaded.sidebarMode === "overlay") {
      loaded.sidebarMode = "docked";
    }
    return loaded;
  } catch {
    return { ...defaultWorkspace };
  }
}

const internal = writable<WorkspaceLayout>(loadWorkspace());

// La persistenza avviene a ogni modifica: il formato è versionato, quindi una
// versione futura può migrare i campi senza interpretare dati ambigui.
internal.subscribe((layout) => {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(layout));
  } catch {
    /* archiviazione non disponibile: la sessione resta coerente */
  }
});

export const workspace = { subscribe: internal.subscribe };

export function updateWorkspace(patch: Partial<WorkspaceLayout>) {
  internal.update((current) => {
    const next: WorkspaceLayout = { ...current, ...patch, version: 1 };
    next.sidebarWidth = clampSidebarWidth(next.sidebarWidth);
    if (patch.sidebarMode && patch.sidebarMode !== "collapsed") {
      next.sidebarLastWidth = next.sidebarWidth;
    }
    next.zoom = Math.min(1.8, Math.max(0.5, Math.round(next.zoom * 100) / 100));
    return next;
  });
}

export function resetWorkspace() {
  internal.set({ ...defaultWorkspace });
}

export interface CustomPreset {
  name: string;
  ribbonMode: WorkspaceRibbonMode;
  sidebarMode: WorkspaceSidebarMode;
  sidebarWidth: number;
  focusMode: boolean;
  zoom: number;
  quickLabels: boolean;
}

const builtinPresets: Record<string, Partial<WorkspaceLayout>> = {
  Scrittura: {
    ribbonMode: "compact",
    sidebarMode: "collapsed",
    focusMode: false,
    zoom: 1,
  },
  "Ricerca pittogrammi": {
    ribbonMode: "compact",
    sidebarMode: "overlay",
    activeSidebarPanel: "simbolo",
    focusMode: false,
    zoom: 1,
  },
  Impaginazione: {
    ribbonMode: "full",
    sidebarMode: "docked",
    activeSidebarPanel: "studio",
    focusMode: false,
    zoom: 1,
  },
  Presentazione: {
    ribbonMode: "hidden",
    sidebarMode: "collapsed",
    focusMode: true,
    zoom: 1.1,
  },
  Concentrazione: {
    ribbonMode: "hidden",
    sidebarMode: "collapsed",
    focusMode: true,
    zoom: 1,
  },
};

function loadCustomPresets(): Record<string, CustomPreset> {
  if (typeof localStorage === "undefined") return {};
  try {
    const raw = JSON.parse(localStorage.getItem(PRESETS_KEY) ?? "{}");
    return raw && typeof raw === "object" ? raw : {};
  } catch {
    return {};
  }
}

const customStore = writable<Record<string, CustomPreset>>(loadCustomPresets());
customStore.subscribe((value) => {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(PRESETS_KEY, JSON.stringify(value));
  } catch {
    /* ignore */
  }
});

export const customPresets = { subscribe: customStore.subscribe };

export function presetNames(): string[] {
  return [...Object.keys(builtinPresets), ...Object.keys(get(customStore))];
}

export function applyPreset(name: string): boolean {
  if (builtinPresets[name]) {
    updateWorkspace({
      ...builtinPresets[name],
      sidebarOverlayChoice: true,
      preset: name,
    });
    return true;
  }
  const custom = get(customStore)[name];
  if (!custom) return false;
  updateWorkspace({
    ribbonMode: custom.ribbonMode,
    sidebarMode: custom.sidebarMode,
    sidebarWidth: custom.sidebarWidth,
    focusMode: custom.focusMode,
    zoom: custom.zoom,
    quickLabels: custom.quickLabels,
    sidebarOverlayChoice: true,
    preset: name,
  });
  return true;
}

export function saveCurrentPreset(name: string) {
  const clean = name.trim();
  if (!clean) return;
  const current = get(internal);
  customStore.update((presets) => ({
    ...presets,
    [clean]: {
      name: clean,
      ribbonMode: current.ribbonMode,
      sidebarMode: current.sidebarMode,
      sidebarWidth: current.sidebarWidth,
      focusMode: current.focusMode,
      zoom: current.zoom,
      quickLabels: current.quickLabels,
    },
  }));
  updateWorkspace({ preset: clean });
}

export function deleteCustomPreset(name: string) {
  customStore.update((presets) => {
    const next = { ...presets };
    delete next[name];
    return next;
  });
}
