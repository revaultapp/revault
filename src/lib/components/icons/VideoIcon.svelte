<script lang="ts">
  interface Props {
    size?: number;
    strokeWidth?: number;
    class?: string;
  }

  let { size = 18, strokeWidth = 1.8, class: className = "" }: Props = $props();
</script>

<!--
  Clapperboard — open slate. The tilted top plate is the duo layer
  (stroked, always visible); its two diagonal marks sit on top so the
  accent fill slides in underneath them on activation.
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
  <path
    class="icon-duo"
    d="M20.2 6 3 11l-.9-2.4c-.3-1.1.3-2.2 1.3-2.5l13.5-4c1.1-.3 2.2.3 2.5 1.3Z"
  />
  <path d="m6.2 5.3 3.1 3.9" />
  <path d="m12.4 3.4 3.1 4" />
  <path d="M3 11h18v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z" />
</svg>

<style>
  .icon-duo {
    fill: var(--icon-duo, transparent);
    transition: fill var(--duration-normal) var(--ease-out);
  }
</style>
