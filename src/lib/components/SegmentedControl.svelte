<script lang="ts">
  interface Segment {
    id: string;
    label: string;
    icon?: any;
  }

  let { segments, selected = $bindable() }: {
    segments: readonly Segment[];
    selected: string;
  } = $props();

  // Refs for each button so we can measure widths/offsets
  let buttonEls: HTMLButtonElement[] = $state([]);
  let containerEl: HTMLDivElement | undefined = $state();

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

  // Re-measure whenever `selected` changes or on mount
  $effect(() => {
    // Touch `selected` to subscribe to changes
    void selected;
    // Use a microtask so the DOM has updated
    queueMicrotask(measure);
  });

</script>

<div class="segmented-control" bind:this={containerEl}>
  <!-- Sliding pill indicator -->
  <div
    class="pill"
    class:pill--visible={measured}
    style="left: {pillLeft}px; width: {pillWidth}px;"
  ></div>

  {#each segments as segment, i (segment.id)}
    <button
      class="segment"
      class:segment--active={selected === segment.id}
      bind:this={buttonEls[i]}
      onclick={() => { selected = segment.id; }}
      type="button"
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
    transition: left 0.3s cubic-bezier(0.4, 0, 0.2, 1),
                width 0.3s cubic-bezier(0.4, 0, 0.2, 1);
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
    color: var(--text-muted);
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
