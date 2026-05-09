<script lang="ts">
  import { scale } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { CheckCircle } from 'lucide-svelte';
  import { prefersReducedMotion } from 'svelte/motion';

  interface Props {
    targetPct: number;
    label: string;
    sublabel?: string;
  }
  let { targetPct, label, sublabel }: Props = $props();

  const circumference = 2 * Math.PI * 54;
  let displayPct = $state(0);
  let rafId = $state(0);

  $effect(() => {
    if (rafId) cancelAnimationFrame(rafId);

    const target = targetPct;

    if (target === 0) {
      displayPct = 0;
      return;
    }

    let currentRafId = 0;

    function tick() {
      displayPct += (target - displayPct) * 0.06;
      if (target >= 100 && displayPct > 99.5) {
        displayPct = 100;
        return;
      }
      currentRafId = requestAnimationFrame(tick);
    }

    tick();
    rafId = currentRafId;

    return () => cancelAnimationFrame(currentRafId);
  });

  let offset = $derived(circumference - (displayPct / 100) * circumference);
  let isComplete = $derived(displayPct >= 99.9);
</script>

<div class="progress-screen">
  <div class="circle-wrap" class:complete={isComplete}>
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
    <span class="pct">
      {#if isComplete}
        <span
          class="check-icon"
          in:scale={{ duration: prefersReducedMotion.current ? 0 : 300, start: 0.6, easing: cubicOut }}
        >
          <CheckCircle size={42} strokeWidth={1.5} color="var(--accent)" />
        </span>
      {:else}
        {Math.round(displayPct)}<small>%</small>
      {/if}
    </span>
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
    filter: drop-shadow(0 0 10px rgba(16, 216, 122, 0.25));
    animation: glow-pulse 2.5s ease-in-out infinite;
  }

  .arc {
    filter: drop-shadow(0 0 4px rgba(16, 216, 122, 0.5));
  }

  @keyframes glow-pulse {
    0%, 100% { filter: drop-shadow(0 0 8px rgba(16, 216, 122, 0.2)); }
    50% { filter: drop-shadow(0 0 16px rgba(16, 216, 122, 0.45)); }
  }

  .circle-wrap.complete svg {
    animation: completion-burst 500ms var(--ease-out) forwards;
  }

  @keyframes completion-burst {
    0%   { transform: scale(1);    filter: drop-shadow(0 0 8px  rgba(16, 216, 122, 0.2)); }
    40%  { transform: scale(1.06); filter: drop-shadow(0 0 20px rgba(16, 216, 122, 0.6)); }
    70%  { transform: scale(1.03); filter: drop-shadow(0 0 14px rgba(16, 216, 122, 0.4)); }
    100% { transform: scale(1);    filter: drop-shadow(0 0 10px rgba(16, 216, 122, 0.25)); }
  }

  .circle-wrap.complete::after {
    content: '';
    position: absolute;
    width: 180px;
    height: 180px;
    border-radius: 50%;
    border: 2px solid var(--accent);
    animation: ring-fade-out 600ms var(--ease-out) forwards;
    pointer-events: none;
  }

  @keyframes ring-fade-out {
    0%   { transform: scale(1);    opacity: 0.7; }
    100% { transform: scale(1.35); opacity: 0;   }
  }

  .check-icon {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  @media (prefers-reduced-motion: reduce) {
    .circle-wrap.complete svg { animation: none; }
    .circle-wrap.complete::after { animation: none; opacity: 0; }
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
