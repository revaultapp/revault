<script lang="ts">
  import type { ComponentType } from 'svelte';
  import { nextSegmentIndex } from './segmentedNav';

  interface Segment {
    id: string;
    label: string;
    icon?: ComponentType;
  }

  let { segments, selected = $bindable(), onselect, label }: {
    segments: readonly Segment[];
    selected: string;
    onselect?: (id: string) => void;
    label: string;
  } = $props();

  // Refs for each button so we can measure widths/offsets
  let buttonEls: HTMLButtonElement[] = $state([]);
  let containerEl: HTMLDivElement | undefined = $state();

  let selectedIndex = $derived(segments.findIndex(s => s.id === selected));

  // Pill position + size, driven by the active button's geometry
  let pillLeft = $state(0);
  let pillWidth = $state(0);
  let measured = $state(false);

  function measure() {
    const idx = segments.findIndex(s => s.id === selected);
    const btn = buttonEls[idx];
    if (!btn || !containerEl) return;

    const containerRect = containerEl.getBoundingClientRect();
    const btnRect = btn.getBoundingClientRect();

    pillLeft = btnRect.left - containerRect.left;
    pillWidth = btnRect.width;
    measured = true;
  }

  function selectSegment(id: string) {
    selected = id;
    onselect?.(id);
  }

  function handleKeydown(e: KeyboardEvent) {
    const current = selectedIndex === -1 ? 0 : selectedIndex;
    const target = nextSegmentIndex(current, e.key, segments.length);
    if (target === null) return;
    e.preventDefault();
    buttonEls[target]?.focus();
    selectSegment(segments[target].id);
  }

  // Re-measure whenever `selected` or `segments` changes (e.g. locale switch) or on mount
  $effect(() => {
    // Touch `selected` and `segments` to subscribe to changes
    void selected;
    void segments;
    // Use a microtask so the DOM has updated
    queueMicrotask(measure);
  });

</script>

<div class="segmented-control" bind:this={containerEl} role="radiogroup" aria-label={label} tabindex="-1" onkeydown={handleKeydown}>
  <!-- Sliding pill indicator -->
  <div
    class="pill"
    class:pill--visible={measured}
    style="transform: translateX({pillLeft}px); width: {pillWidth}px;"
    aria-hidden="true"
  ></div>

  {#each segments as segment, i (segment.id)}
    {@const isSelected = selected === segment.id}
    <button
      class="segment"
      class:segment--active={isSelected}
      bind:this={buttonEls[i]}
      onclick={() => selectSegment(segment.id)}
      type="button"
      role="radio"
      aria-checked={isSelected}
      tabindex={i === (selectedIndex === -1 ? 0 : selectedIndex) ? 0 : -1}
    >
      {#if segment.icon}
        {@const Icon = segment.icon}
        <Icon size={16} strokeWidth={2} />
      {/if}
      <span>{segment.label}</span>
    </button>
  {/each}
</div>

<style>
  .segmented-control {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 2px;
    padding: 4px;
    border-radius: 12px;
    background: var(--navy-bg);
    border: 1px solid var(--border);
  }

  /* The sliding white pill */
  .pill {
    position: absolute;
    top: 4px;
    bottom: 4px;
    border-radius: 9px;
    background: var(--bg-card);
    box-shadow:
      0 1px 3px rgba(0, 0, 0, 0.08),
      0 1px 2px rgba(0, 0, 0, 0.06);
    left: 0;
    transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    will-change: transform;
    opacity: 0;
    pointer-events: none;
    z-index: 0;
  }

  .pill--visible {
    opacity: 1;
  }

  .segment {
    position: relative;
    z-index: 1;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 7px 16px;
    border-radius: 9px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    background: none;
    border: none;
    cursor: pointer;
    white-space: nowrap;
    transition: color 0.2s ease;
    user-select: none;
    -webkit-user-select: none;
  }

  .segment:hover:not(.segment--active) {
    color: var(--text-secondary);
  }

  .segment--active {
    color: var(--text-primary);
    font-weight: 600;
  }

  /* Ensure icons align nicely */
  .segment :global(svg) {
    flex-shrink: 0;
  }
</style>
