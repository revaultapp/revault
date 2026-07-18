<script lang="ts">
  interface Props {
    size?: number;
    strokeWidth?: number;
    class?: string;
  }

  let { size = 18, strokeWidth = 1.8, class: className = "" }: Props = $props();
</script>

<!--
  Saved-space glyph (sidebar "Saved" badge): an arrow flowing back into
  the drive — literally "space reclaimed". Replaces the v2 safe-with-wheel,
  which kept reading as a crosshair/camera shutter at 16px no matter the
  spoke layout. Two duo shapes: the solid arrowhead (stroke-less, its
  outline is the primary chevron) and the drive slab; the activity dots
  are drawn after the slab so they punch through the accent fill. The
  badge has no hover/active state — Sidebar.svelte sets a permanent
  low-alpha --icon-duo on .saved-badge.
  See icons/index.ts for the shared duo-fill mechanism.
-->
<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={strokeWidth}
  stroke-linecap="round"
  stroke-linejoin="round"
  class={className}
  aria-hidden="true"
>
  <path class="icon-duo" stroke="none" d="m16 6-4 4-4-4Z" />
  <path d="M12 2v8" />
  <path d="m16 6-4 4-4-4" />
  <rect class="icon-duo" x="2.5" y="14" width="19" height="7.5" rx="2" />
  <path d="M6.5 17.75h.01" />
  <path d="M10.5 17.75h.01" />
</svg>

<style>
  .icon-duo {
    fill: var(--icon-duo, transparent);
    transition: fill var(--duration-normal) var(--ease-out);
  }
</style>
