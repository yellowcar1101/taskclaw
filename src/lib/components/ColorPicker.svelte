<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let value: string = '';       // current hex color, or '' for default
  export let label: string = 'Color';  // accessible label

  const dispatch = createEventDispatcher<{ change: string }>();

  let open = false;
  let nativeInput: HTMLInputElement;

  // Windows / Office–style swatch palette
  const SWATCHES: string[][] = [
    // Neutrals: black → white
    ['#000000','#1F1F1F','#3D3D3D','#5C5C5C','#7A7A7A','#999999','#B8B8B8','#D6D6D6','#EDEDED','#FFFFFF'],
    // Dark shades
    ['#7F0000','#7F2700','#7F5200','#4C5200','#00527F','#002B7F','#37007F','#7F0067','#7F3D00','#3D3D00'],
    // Core colors
    ['#FF0000','#FF6600','#FFCC00','#99CC00','#00AAFF','#0033FF','#7700FF','#FF00CC','#FF8C00','#99FF00'],
    // Mid tones
    ['#FF6666','#FFB366','#FFE680','#CCFF66','#66CCFF','#6680FF','#BB66FF','#FF66DD','#FFBB66','#EEFF99'],
    // Blues / teals
    ['#003366','#006699','#0099CC','#00CCCC','#00CC99','#009966','#006633','#336600','#663300','#993300'],
    // Accents
    ['#FF3333','#FF6633','#FF9933','#33CC33','#33CCFF','#3366FF','#9933FF','#FF33AA','#66FF66','#FFFF33'],
  ];

  function pick(color: string) {
    dispatch('change', color);
    open = false;
  }

  function openNative() {
    nativeInput.value = value || '#e0e0e0';
    nativeInput.click();
  }

  function onNativeChange() {
    pick(nativeInput.value);
  }

  function reset() {
    dispatch('change', '');
    open = false;
  }
</script>

<!-- Hidden native color input -->
<input
  bind:this={nativeInput}
  type="color"
  style="position:absolute;opacity:0;pointer-events:none;width:0;height:0"
  on:change={onNativeChange}
/>

<div class="cp-wrap" aria-label={label}>
  <!-- Trigger swatch -->
  <button
    class="cp-trigger"
    class:default={!value}
    style={value ? `background:${value}` : ''}
    on:click={() => open = !open}
    title={value || 'Default (click to change)'}
  >
    {#if !value}<span class="cp-auto">A</span>{/if}
  </button>

  {#if value}
    <button class="cp-reset" on:click={reset} title="Reset to default">✕</button>
  {/if}

  {#if open}
    <!-- Click-away backdrop -->
    <div class="cp-backdrop" on:click={() => open = false} role="none"></div>

    <div class="cp-panel">
      <div class="cp-section-label">Theme colours</div>
      <div class="cp-grid">
        {#each SWATCHES as row}
          {#each row as color}
            <button
              class="cp-swatch"
              class:selected={color.toLowerCase() === value?.toLowerCase()}
              style="background:{color}"
              on:click={() => pick(color)}
              title={color}
            ></button>
          {/each}
        {/each}
      </div>

      <div class="cp-divider"></div>

      <div style="display:flex;gap:6px;padding:6px 8px 4px">
        <button class="cp-custom-btn" on:click={openNative}>More colours…</button>
        <button class="cp-custom-btn muted" on:click={reset}>Default</button>
      </div>
    </div>
  {/if}
</div>

<style>
  .cp-wrap {
    position: relative;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .cp-trigger {
    width: 28px;
    height: 22px;
    border: 2px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
    background: var(--input-bg);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: border-color 0.1s;
    flex-shrink: 0;
  }
  .cp-trigger:hover { border-color: var(--accent); }
  .cp-trigger.default { background: repeating-linear-gradient(45deg, var(--border) 0px, var(--border) 2px, transparent 2px, transparent 6px); }

  .cp-auto {
    font-size: 11px;
    font-weight: 700;
    color: var(--text-dim);
    line-height: 1;
  }

  .cp-reset {
    background: none;
    border: none;
    color: var(--text-dim);
    cursor: pointer;
    font-size: 10px;
    padding: 0 2px;
    line-height: 1;
  }
  .cp-reset:hover { color: var(--red); }

  .cp-backdrop {
    position: fixed;
    inset: 0;
    z-index: 998;
  }

  .cp-panel {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    z-index: 999;
    background: var(--surface-elevated);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 8px 24px rgba(0,0,0,0.5);
    padding: 8px 8px 4px;
    min-width: 210px;
  }

  .cp-section-label {
    font-size: 10px;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: 6px;
    padding: 0 1px;
  }

  .cp-grid {
    display: grid;
    grid-template-columns: repeat(10, 18px);
    gap: 2px;
  }

  .cp-swatch {
    width: 18px;
    height: 18px;
    border-radius: 2px;
    border: 1px solid rgba(0,0,0,0.25);
    cursor: pointer;
    padding: 0;
    transition: transform 0.05s, box-shadow 0.05s;
  }
  .cp-swatch:hover { transform: scale(1.2); box-shadow: 0 2px 6px rgba(0,0,0,0.4); z-index: 1; position: relative; }
  .cp-swatch.selected { box-shadow: 0 0 0 2px var(--accent); }

  .cp-divider { height: 1px; background: var(--border); margin: 6px 0 2px; }

  .cp-custom-btn {
    flex: 1;
    background: var(--hover-btn);
    border: 1px solid var(--border);
    color: var(--text);
    padding: 4px 8px;
    border-radius: 3px;
    cursor: pointer;
    font-size: 11px;
    text-align: center;
  }
  .cp-custom-btn:hover { background: var(--hover); }
  .cp-custom-btn.muted { color: var(--text-dim); }
</style>
