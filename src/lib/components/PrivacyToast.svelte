<script lang="ts">
  import { fly, fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import { Shield } from 'lucide-svelte';
  import { prefersReducedMotion } from 'svelte/motion';

  interface Props {
    message: string;
    visible: boolean;
  }

  let { message, visible }: Props = $props();

  let dur = $derived(prefersReducedMotion.current ? 0 : 350);
  let durOut = $derived(prefersReducedMotion.current ? 0 : 250);
</script>

{#if visible}
  <div
    class="privacy-chip"
    in:fly={{ y: 10, duration: dur, easing: cubicOut }}
    out:fade={{ duration: durOut }}
    role="status"
    aria-live="polite"
  >
    <Shield size={14} strokeWidth={2} />
    <span>{message}</span>
  </div>
{/if}

<style>
  .privacy-chip {
    position: fixed;
    bottom: 24px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    border-radius: 20px;
    background: var(--bg-card);
    border: 1px solid rgba(16, 216, 122, 0.25);
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.2), 0 0 0 1px rgba(16, 216, 122, 0.1);
    color: var(--accent);
    font-size: 13px;
    font-weight: 600;
    pointer-events: none;
    z-index: 100;
    white-space: nowrap;
  }
</style>
