<script lang="ts">
  import { FolderOpen, Sun, Moon, ChevronDown } from "lucide-svelte";
  import ToggleSwitch from "$lib/components/ToggleSwitch.svelte";

  let overwrite = $state(false);
  let autostrip = $state(true);
  let theme = $state<"light" | "dark">("light");
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
        <span class="name">Output folder</span>
        <span class="desc">Where processed files are saved</span>
      </div>
      <button class="pill">
        <FolderOpen size={14} strokeWidth={2} />
        <span>~/Pictures/ReVault</span>
      </button>
    </div>
    <div class="row">
      <div class="label">
        <span class="name">Overwrite originals</span>
        <span class="desc">Replace files instead of creating copies</span>
      </div>
      <ToggleSwitch bind:checked={overwrite} />
    </div>
    <div class="row">
      <div class="label">
        <span class="name">Theme</span>
        <span class="desc">Switch between light and dark mode</span>
      </div>
      <div class="segmented">
        <button class="seg" class:active={theme === 'light'} onclick={() => theme = 'light'}>
          <Sun size={14} strokeWidth={2} />
          <span>Light</span>
        </button>
        <button class="seg" class:active={theme === 'dark'} onclick={() => theme = 'dark'}>
          <Moon size={14} strokeWidth={2} />
          <span>Dark</span>
        </button>
      </div>
    </div>
  </section>

  <!-- Compression -->
  <section class="section">
    <div class="section-header">
      <h2>Compression</h2>
      <p>Default compression behavior</p>
    </div>
    <hr />
    <div class="row">
      <div class="label">
        <span class="name">Default quality</span>
        <span class="desc">Balance between size and quality</span>
      </div>
      <div class="quality-val">
        <span class="quality-num">80</span>
        <span class="quality-unit">%</span>
      </div>
    </div>
    <div class="row">
      <div class="label">
        <span class="name">Default format</span>
        <span class="desc">Output format for conversions</span>
      </div>
      <button class="pill">
        <span>WebP</span>
        <ChevronDown size={12} strokeWidth={2} />
      </button>
    </div>
  </section>

  <!-- Privacy -->
  <section class="section">
    <div class="section-header">
      <h2>Privacy</h2>
      <p>Metadata and sensitive data</p>
    </div>
    <hr />
    <div class="row">
      <div class="label">
        <span class="name">Auto-strip on import</span>
        <span class="desc">Remove metadata when files are added</span>
      </div>
      <ToggleSwitch bind:checked={autostrip} />
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
    <div class="row small">
      <span class="name">Source code</span>
      <button class="pill">GitHub &rarr;</button>
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
    font-weight: 600;
    color: var(--text-primary);
  }

  .section-header p {
    font-size: 13px;
    color: #999;
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
    color: #999;
  }

  .pill {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: 8px;
    background: var(--wood-bg);
    color: var(--wood);
    font-size: 12px;
    font-weight: 500;
    transition: background 0.15s;
  }

  .pill:hover {
    background: var(--wood-border);
  }

  .segmented {
    display: flex;
    border-radius: 8px;
    background: var(--wood-bg);
    overflow: hidden;
  }

  .seg {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    font-size: 12px;
    color: #999;
    border-radius: 8px;
    transition: all 0.15s;
  }

  .seg.active {
    background: #fff;
    color: var(--wood);
    font-weight: 600;
    box-shadow: 0 0 0 1px var(--border);
  }

  .quality-val {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .quality-num {
    font-size: 14px;
    font-weight: 700;
    color: var(--accent);
  }

  .quality-unit {
    font-size: 12px;
    color: #999;
  }

  .version-val {
    font-size: 13px;
    font-weight: 500;
    color: #999;
  }
</style>
