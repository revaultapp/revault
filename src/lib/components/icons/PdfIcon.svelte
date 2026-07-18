<script lang="ts">
  interface Props {
    size?: number;
    strokeWidth?: number;
    class?: string;
  }

  let { size = 18, strokeWidth = 1.8, class: className = "" }: Props = $props();
</script>

<!--
  Document with an oversized folded corner — the dog-ear grown to half
  the page width so it becomes the glyph's signature rather than a
  timid notch. The fold triangle is the duo fill (stroke-less: its two
  visible edges are drawn by the primary fold path and its hypotenuse
  IS the document's cut corner, kept perfectly aligned by sharing the
  13,2 → 20,9 diagonal), plus a single text line for the "document" read.
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
  <path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9Z" />
  <path class="icon-duo" stroke="none" d="M13 2v5a2 2 0 0 0 2 2h5Z" />
  <path d="M13 2v5a2 2 0 0 0 2 2h5" />
  <path d="M8.5 16h7" />
</svg>

<style>
  .icon-duo {
    fill: var(--icon-duo, transparent);
    transition: fill var(--duration-normal) var(--ease-out);
  }
</style>
