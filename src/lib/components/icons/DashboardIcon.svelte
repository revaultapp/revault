<script lang="ts">
  interface Props {
    size?: number;
    strokeWidth?: number;
    class?: string;
  }

  let { size = 18, strokeWidth = 1.8, class: className = "" }: Props = $props();
</script>

<!--
  Compass — bold rim + kite needle. The needle is the duo layer but keeps
  its stroke: without it the glyph at rest is just an empty circle (the v1
  mistake — rim ticks mushed at 18px and the reveal-only needle left a
  hollow ring). Active fills the needle with the accent tint.
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
  <circle cx="12" cy="12" r="9" />
  <path class="icon-duo" d="M16.24 7.76 14.12 14.12 7.76 16.24 9.88 9.88Z" />
</svg>

<style>
  .icon-duo {
    fill: var(--icon-duo, transparent);
    transition: fill var(--duration-normal) var(--ease-out);
  }
</style>
