<script lang="ts">
  import { Sun, Moon } from "lucide-svelte";
  import { theme } from "$lib/stores/theme";
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
    border-radius: 8px;
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
    border-radius: 8px;
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
</style>
