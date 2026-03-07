<script lang="ts">
  const actions: { icon: string; title: string; desc: string; color: string }[] = [
    { icon: "minimize-2", title: "Compress", desc: "Reduce file size up to 90%", color: "#10b981" },
    { icon: "repeat", title: "Convert", desc: "Change format: HEIC, PNG, WebP", color: "#10b981" },
    { icon: "search", title: "Analyze Storage", desc: "Find duplicates, blurry photos, waste", color: "#8b6f47" },
    { icon: "shield", title: "Privacy Check", desc: "Find and strip sensitive metadata", color: "#8b6f47" },
  ];
</script>

<div class="home">
  <section class="action-cards">
    {#each actions as action}
      <button class="action-card" style="--card-color: {action.color}">
        <div class="action-icon">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke={action.color} stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            {#if action.icon === "minimize-2"}
              <polyline points="4 14 10 14 10 20"/><polyline points="20 10 14 10 14 4"/><line x1="14" y1="10" x2="21" y2="3"/><line x1="3" y1="21" x2="10" y2="14"/>
            {:else if action.icon === "repeat"}
              <path d="m17 2 4 4-4 4"/><path d="M3 11v-1a4 4 0 0 1 4-4h14"/><path d="m7 22-4-4 4-4"/><path d="M21 13v1a4 4 0 0 1-4 4H3"/>
            {:else if action.icon === "search"}
              <circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/>
            {:else if action.icon === "shield"}
              <path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/>
            {/if}
          </svg>
        </div>
        <span class="action-title">{action.title}</span>
        <span class="action-desc">{action.desc}</span>
        <span class="action-spacer"></span>
        <span class="action-link">Start →</span>
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
