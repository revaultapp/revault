<script lang="ts">
  import { Sun, Moon, FolderOpen, RotateCcw } from "lucide-svelte";
  import { theme } from "$lib/stores/theme";
  import { defaultOutputDir } from "$lib/stores/settings";
  import { browseOutputDir } from "$lib/utils";

  async function pickOutputDir() {
    const dir = await browseOutputDir();
    if (dir) defaultOutputDir.set(dir);
  }

  function resetOutputDir() {
    defaultOutputDir.set(null);
  }
</script>

<div class="sections">
  <!-- General -->
  <section class="section">
    <div class="section-header">
      <h2>General</h2>
      <p>Basic app preferences</p>
    </div>
    <hr />
    <div class="row">
      <div class="label">
        <span class="name">Theme</span>
        <span class="desc">Switch between light and dark mode</span>
      </div>
      <div class="segmented">
        <button class="seg" class:active={$theme === 'light'} onclick={() => theme.set('light')}>
          <Sun size={14} strokeWidth={2} />
          <span>Light</span>
        </button>
        <button class="seg" class:active={$theme === 'dark'} onclick={() => theme.set('dark')}>
          <Moon size={14} strokeWidth={2} />
          <span>Dark</span>
        </button>
      </div>
    </div>
    <div class="row">
      <div class="label">
        <span class="name">Default output folder</span>
        <span class="desc">Used when no per-tool folder is set</span>
      </div>
      <div class="output-controls">
        <button class="btn-ghost" onclick={pickOutputDir}>
          <FolderOpen size={14} strokeWidth={2} />
          <span class="output-name">{$defaultOutputDir?.split(/[\\/]/).pop() ?? "Same as input"}</span>
        </button>
        {#if $defaultOutputDir}
          <button class="btn-ghost btn-icon" onclick={resetOutputDir} title="Reset to same as input">
            <RotateCcw size={14} strokeWidth={2} />
          </button>
        {/if}
      </div>
    </div>
  </section>

  <!-- About -->
  <section class="section">
    <div class="section-header">
      <h2>About</h2>
      <p>App info and links</p>
    </div>
    <hr />
    <div class="row small">
      <span class="name">Version</span>
      <span class="version-val">0.1.0</span>
    </div>
  </section>
</div>

<style>
  .sections {
    display: flex;
    flex-direction: column;
    gap: 40px;
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  .section-header h2 {
    font-size: 18px;
    font-weight: 700;
    letter-spacing: -0.025em;
    color: var(--text-primary);
  }

  .section-header p {
    font-size: 13px;
    color: var(--text-muted);
    margin-top: 4px;
  }

  hr {
    border: none;
    height: 1px;
    background: var(--border);
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 48px;
  }

  .row.small {
    height: 36px;
  }

  .label {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .name {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .desc {
    font-size: 12px;
    color: var(--text-muted);
  }

  .segmented {
    display: flex;
    border-radius: var(--radius-sm);
    background: var(--navy-bg);
    overflow: hidden;
  }

  .seg {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    font-size: 12px;
    color: var(--text-muted);
    border-radius: var(--radius-sm);
    transition: background-color 0.15s, border-color 0.15s;
  }

  .seg.active {
    background: var(--bg-card);
    color: var(--accent);
    font-weight: 600;
    box-shadow: 0 0 0 1px var(--border);
  }

  .version-val {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-muted);
  }

  .output-controls {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .btn-ghost {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: var(--radius-sm);
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    border: 1px solid var(--border);
    transition: background-color 0.15s, border-color 0.15s;
  }

  .btn-ghost:hover {
    background: var(--navy-bg);
    border-color: var(--text-muted);
  }

  .btn-ghost.btn-icon {
    padding: 6px;
    color: var(--text-muted);
  }

  .output-name {
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
