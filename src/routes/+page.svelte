<script lang="ts">
  import { Zap, Shuffle, ScanSearch, ShieldCheck } from "lucide-svelte";
  import type { ComponentType } from "svelte";

  const actions: { icon: ComponentType; title: string; desc: string; color: string; link: string }[] = [
    { icon: Zap, title: "Compress", desc: "Reduce file size up to 90%", color: "#10b981", link: "Start →" },
    { icon: Shuffle, title: "Convert", desc: "Change format: HEIC, PNG, WebP", color: "#10b981", link: "Start →" },
    { icon: ScanSearch, title: "Analyze Storage", desc: "Find duplicates, blurry photos, waste", color: "#10b981", link: "Scan →" },
    { icon: ShieldCheck, title: "Privacy Check", desc: "Find and strip sensitive metadata", color: "#10b981", link: "Scan →" },
  ];
</script>

<div class="home">
  <section class="action-cards">
    {#each actions as action}
      <button class="action-card" style="--card-color: {action.color}">
        <div class="action-icon">
          <action.icon size={18} strokeWidth={1.8} color={action.color} />
        </div>
        <span class="action-title">{action.title}</span>
        <span class="action-desc">{action.desc}</span>
        <span class="action-spacer"></span>
        <span class="action-link">{action.link}</span>
      </button>
    {/each}
  </section>

  <p class="hint">Drop files anywhere to get started</p>
</div>

<style>
  .home {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 28px;
    min-height: 100%;
  }

  .action-cards {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 16px;
    width: 100%;
  }

  .action-card {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 20px;
    height: 180px;
    border-radius: var(--radius-lg);
    background: var(--bg-card);
    border: 1px solid color-mix(in srgb, var(--card-color) 13%, transparent);
    box-shadow: 0 2px 12px rgba(0, 0, 0, 0.04);
    text-align: left;
    transition: border-color 0.15s, box-shadow 0.15s;
  }

  .action-card:hover {
    border-color: color-mix(in srgb, var(--card-color) 30%, transparent);
    box-shadow: 0 4px 20px color-mix(in srgb, var(--card-color) 10%, transparent);
  }

  .action-icon {
    width: 36px;
    height: 36px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--card-color) 10%, transparent);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .action-title {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .action-desc {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.4;
  }

  .action-spacer {
    flex: 1;
  }

  .action-link {
    font-size: 12px;
    font-weight: 600;
    color: var(--card-color);
  }

  .hint {
    font-size: 13px;
    color: var(--text-muted);
    text-align: center;
  }
</style>
