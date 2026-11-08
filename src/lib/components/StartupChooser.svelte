<script lang="ts">
  import { api } from "../api";
  import type { Profile } from "../types";

  interface Props {
    profiles: Profile[];
    onChooseProfile: (profile: Profile) => Promise<void>;
    onChooseCreator: (pin: string | null) => Promise<boolean>;
  }

  let { profiles, onChooseProfile, onChooseCreator }: Props = $props();

  let creatorMode = $state(false);
  let pinRequired = $state(false);
  let pin = $state("");
  let busy = $state(false);
  let error = $state("");

  $effect(() => {
    void (async () => {
      try {
        pinRequired = await api.creatorPinStatus();
      } catch {
        pinRequired = false;
      }
    })();
  });

  async function chooseProfile(profile: Profile) {
    if (busy) return;
    busy = true;
    error = "";
    try {
      await onChooseProfile(profile);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = false;
    }
  }

  async function chooseCreator() {
    if (busy) return;
    if (pinRequired && !pin.trim()) {
      error = "Inserisci il PIN dell'Area Creator.";
      return;
    }
    busy = true;
    error = "";
    try {
      const ok = await onChooseCreator(pin.trim() || null);
      if (!ok) error = "PIN non valido.";
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = false;
    }
  }
</script>

<div class="startup" role="dialog" aria-modal="true" aria-labelledby="startup-title">
  <div class="startup__card">
    <header class="startup__head">
      <div class="startup__mark" aria-hidden="true">CAA</div>
      <div>
        <h1 id="startup-title">CAA Writer</h1>
        <p>Scegli chi usa il programma oppure entra nell'Area Creator.</p>
      </div>
    </header>

    <section aria-labelledby="startup-users">
      <h2 id="startup-users">Utente</h2>
      <div class="startup__profiles">
        {#each profiles as profile (profile.id)}
          <button
            class="profile-card"
            disabled={busy}
            onclick={() => void chooseProfile(profile)}
          >
            <span class="profile-card__avatar" aria-hidden="true">
              {profile.avatar || profile.name.slice(0, 1)}
            </span>
            <span class="profile-card__body">
              <strong>{profile.name}</strong>
              <span>{profile.age_theme || "profilo personale"}</span>
            </span>
            <span class="profile-card__arrow" aria-hidden="true">›</span>
          </button>
        {/each}
      </div>
    </section>

    <section class="startup__creator" aria-labelledby="startup-creator">
      <h2 id="startup-creator">Area Creator</h2>
      <p>
        Insegnanti, logopedisti ed educatori: crea materiali, gestisci profili e usa tutti gli
        strumenti dell'editor.
      </p>
      {#if creatorMode && pinRequired}
        <label class="startup__pin">
          <span>PIN</span>
          <input
            type="password"
            inputmode="numeric"
            bind:value={pin}
            autocomplete="off"
            onkeydown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                void chooseCreator();
              }
            }}
          />
        </label>
      {/if}
      {#if error}<p class="startup__error" role="alert">{error}</p>{/if}
      <div class="startup__actions">
        {#if creatorMode}
          <button class="btn btn--ghost" onclick={() => (creatorMode = false)} disabled={busy}>Indietro</button>
          <button class="btn btn--solid" onclick={() => void chooseCreator()} disabled={busy}>
            {busy ? "Verifico…" : "Entra come Creator"}
          </button>
        {:else}
          <button
            class="btn btn--solid"
            onclick={() => {
              if (pinRequired) creatorMode = true;
              else void chooseCreator();
            }}
            disabled={busy}
          >Modalità Creator</button>
        {/if}
      </div>
    </section>
  </div>
</div>

<style>
  .startup {
    position: fixed;
    inset: 0;
    z-index: 3000;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    background: radial-gradient(circle at 20% 10%, #0f766e33, transparent 55%),
      linear-gradient(150deg, #0f172a, #123c3a 60%, #0f172a);
    color: #e2e8f0;
    overflow: auto;
  }
  .startup__card {
    width: min(760px, 100%);
    background: #ffffff;
    color: #0f172a;
    border-radius: 22px;
    padding: clamp(20px, 4vw, 38px);
    box-shadow: 0 30px 80px #0008;
    display: grid;
    gap: 24px;
  }
  .startup__head { display: flex; gap: 16px; align-items: center; }
  .startup__mark {
    width: 58px;
    height: 58px;
    border-radius: 16px;
    background: #0f766e;
    color: #fff;
    display: grid;
    place-items: center;
    font-weight: 800;
    letter-spacing: 0.04em;
  }
  .startup__head h1 { margin: 0; font-size: 26px; }
  .startup__head p { margin: 4px 0 0; color: #64748b; font-size: 14px; }
  h2 { font-size: 13px; text-transform: uppercase; letter-spacing: 0.09em; color: #0f766e; margin: 0 0 10px; }
  .startup__profiles { display: grid; gap: 10px; }
  .profile-card {
    display: flex;
    align-items: center;
    gap: 14px;
    width: 100%;
    border: 1px solid #dbe3ee;
    background: #f8fafc;
    border-radius: 14px;
    padding: 12px 14px;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .profile-card:hover { border-color: #0f766e; background: #ecfdf5; }
  .profile-card:disabled { opacity: 0.6; cursor: wait; }
  .profile-card__avatar {
    width: 46px;
    height: 46px;
    border-radius: 50%;
    background: #ccfbf1;
    color: #115e59;
    display: grid;
    place-items: center;
    font-weight: 800;
    font-size: 18px;
  }
  .profile-card__body { display: grid; gap: 2px; flex: 1; }
  .profile-card__body strong { font-size: 17px; }
  .profile-card__body span { font-size: 12px; color: #64748b; }
  .profile-card__arrow { color: #0f766e; font-size: 26px; line-height: 1; }
  .startup__creator {
    border-top: 1px solid #e2e8f0;
    padding-top: 18px;
  }
  .startup__creator p { margin: 0 0 12px; color: #475569; font-size: 13px; }
  .startup__pin { display: grid; gap: 5px; max-width: 220px; margin-bottom: 10px; font-size: 12px; color: #475569; }
  .startup__pin input { padding: 9px 11px; border: 1px solid #cbd5e1; border-radius: 9px; font: inherit; }
  .startup__actions { display: flex; gap: 8px; flex-wrap: wrap; }
  .startup__error { margin: 0 0 10px; color: #b91c1c; font-size: 12px; font-weight: 600; }
  .btn {
    border: 1px solid #cbd5e1;
    background: #fff;
    color: #1e293b;
    border-radius: 10px;
    padding: 9px 14px;
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }
  .btn--solid { background: #0f766e; border-color: #0f766e; color: #fff; }
  .btn--solid:hover { background: #115e59; }
  .btn--ghost { background: transparent; }
  .btn:disabled { opacity: 0.6; cursor: wait; }
  @media (max-width: 520px) {
    .startup { padding: 0; align-items: stretch; }
    .startup__card { border-radius: 0; min-height: 100%; }
  }
</style>
