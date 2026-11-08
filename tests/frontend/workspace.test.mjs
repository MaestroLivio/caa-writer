// Test del WorkspaceLayoutManager (modulo TypeScript compilato da
// `npm run test:frontend`). Verifica persistenza versionata, limiti, preset.

import assert from "node:assert/strict";
import test from "node:test";
import { get } from "svelte/store";

class MemoryStorage {
  #data = new Map();
  get length() {
    return this.#data.size;
  }
  getItem(key) {
    return this.#data.has(key) ? this.#data.get(key) : null;
  }
  setItem(key, value) {
    this.#data.set(key, String(value));
  }
  removeItem(key) {
    this.#data.delete(key);
  }
  clear() {
    this.#data.clear();
  }
  key(index) {
    return [...this.#data.keys()][index] ?? null;
  }
}

globalThis.localStorage = new MemoryStorage();
const {
  applyPreset,
  clampSidebarWidth,
  customPresets,
  defaultWorkspace,
  resetWorkspace,
  saveCurrentPreset,
  updateWorkspace,
  workspace,
} = await import("../../.test-build/workspace/layout.js");

test("il workspace parte dai valori predefiniti ed è versionato", () => {
  resetWorkspace();
  const layout = get(workspace);
  assert.equal(layout.version, 1);
  assert.equal(layout.ribbonMode, defaultWorkspace.ribbonMode);
  assert.equal(layout.sidebarMode, "docked");
  const stored = JSON.parse(localStorage.getItem("caa.workspace.v1"));
  assert.equal(stored.version, 1);
});

test("la larghezza del pannello rispetta i limiti", () => {
  assert.equal(clampSidebarWidth(100), 260);
  assert.equal(clampSidebarWidth(1000), 640);
  assert.equal(clampSidebarWidth(432), 432);
});

test("le modifiche sono persistite e la larghezza precedente è ricordata", () => {
  updateWorkspace({ sidebarWidth: 500, sidebarMode: "docked" });
  assert.equal(get(workspace).sidebarWidth, 500);
  updateWorkspace({ sidebarMode: "collapsed" });
  assert.equal(get(workspace).sidebarMode, "collapsed");
  assert.equal(get(workspace).sidebarLastWidth, 500);
  const stored = JSON.parse(localStorage.getItem("caa.workspace.v1"));
  assert.equal(stored.sidebarLastWidth, 500);
});

test("i preset incorporati applicano modalità coerenti", () => {
  assert.equal(applyPreset("Scrittura"), true);
  assert.equal(get(workspace).ribbonMode, "compact");
  assert.equal(get(workspace).sidebarMode, "collapsed");
  assert.equal(applyPreset("Ricerca pittogrammi"), true);
  assert.equal(get(workspace).sidebarMode, "overlay");
  assert.equal(get(workspace).activeSidebarPanel, "simbolo");
  assert.equal(applyPreset("Concentrazione"), true);
  assert.equal(get(workspace).focusMode, true);
});

test("un preset personalizzato viene salvato e riapplicato", () => {
  updateWorkspace({ sidebarWidth: 480, ribbonMode: "compact", sidebarMode: "collapsed" });
  saveCurrentPreset("Mio preset");
  assert.ok(get(customPresets)["Mio preset"]);
  updateWorkspace({ sidebarWidth: 300, ribbonMode: "full", sidebarMode: "docked" });
  assert.equal(applyPreset("Mio preset"), true);
  assert.equal(get(workspace).sidebarWidth, 480);
  assert.equal(get(workspace).ribbonMode, "compact");
  assert.equal(get(workspace).preset, "Mio preset");
});

test("un preset inesistente non altera il workspace", () => {
  const before = get(workspace);
  assert.equal(applyPreset("Non esiste"), false);
  assert.deepEqual(get(workspace), before);
});
