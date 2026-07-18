<script lang="ts">
  interface Props {
    size?: number;
    strokeWidth?: number;
    class?: string;
  }

  let { size = 18, strokeWidth = 1.8, class: className = "" }: Props = $props();
</script>

<!--
  Document with folded corner. The dog-ear triangle is the duo fill
  (stroke-less — its two visible edges are already drawn by the primary
  fold path, and the hypotenuse is the document's own cut corner), plus
  two text lines for the "document" read.
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
  <path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" />
  <path class="icon-duo" stroke="none" d="M14 2v4a2 2 0 0 0 2 2h4Z" />
  <path d="M14 2v4a2 2 0 0 0 2 2h4" />
  <path d="M8 13h8" />
  <path d="M8 17h5" />
</svg>

<style>
  .icon-duo {
    fill: var(--icon-duo, transparent);
    transition: fill var(--duration-normal) var(--ease-out);
  }
</style>
