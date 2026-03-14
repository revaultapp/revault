<script lang="ts">
  interface Props {
    targetPct: number;
    label: string;
    sublabel?: string;
  }
  let { targetPct, label, sublabel }: Props = $props();

  const circumference = 2 * Math.PI * 54;
  let displayPct = $state(0);

  $effect(() => {
    const target = targetPct;
    let rafId: number;

    function tick() {
      displayPct += (target - displayPct) * 0.06;
      if (target >= 100 && displayPct > 99.5) {
        displayPct = 100;
        return;
      }
      rafId = requestAnimationFrame(tick);
    }

    rafId = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(rafId);
  });

  let offset = $derived(circumference - (displayPct / 100) * circumference);
</script>

<div class="progress-screen">
  <div class="circle-wrap">
    <svg width="180" height="180" viewBox="0 0 120 120">
      <circle cx="60" cy="60" r="54" fill="none" stroke="var(--navy-bg)" stroke-width="6" />
      <circle
        cx="60" cy="60" r="54" fill="none"
        stroke="var(--accent)" stroke-width="6"
        stroke-linecap="round"
        stroke-dasharray={circumference}
        stroke-dashoffset={offset}
        transform="rotate(-90 60 60)"
        class="arc"
      />
    </svg>
    <span class="pct">{Math.round(displayPct)}<small>%</small></span>
  </div>
  <p class="progress-label">{label}</p>
  {#if sublabel}
    <p class="progress-saved">{sublabel}</p>
  {/if}
</div>

<style>
  .progress-screen {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 100%;
    gap: 20px;
  }

  .circle-wrap {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .circle-wrap svg {
    filter: drop-shadow(0 0 10px rgba(16, 185, 129, 0.25));
    animation: glow-pulse 2.5s ease-in-out infinite;
  }

  .arc {
    filter: drop-shadow(0 0 4px rgba(16, 185, 129, 0.5));
  }

  @keyframes glow-pulse {
    0%, 100% { filter: drop-shadow(0 0 8px rgba(16, 185, 129, 0.2)); }
    50% { filter: drop-shadow(0 0 16px rgba(16, 185, 129, 0.45)); }
  }

  .pct {
    position: absolute;
    font-size: 38px;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: -0.02em;
    font-variant-numeric: tabular-nums;
  }

  .pct small {
    font-size: 18px;
    font-weight: 500;
    color: var(--text-muted);
  }

  .progress-label {
    font-size: 15px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .progress-saved {
    font-size: 13px;
    font-weight: 500;
    color: var(--accent);
  }
</style>
