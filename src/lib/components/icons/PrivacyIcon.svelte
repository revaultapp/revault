<script lang="ts">
  interface Props {
    size?: number;
    strokeWidth?: number;
    class?: string;
  }

  let { size = 18, strokeWidth = 1.8, class: className = "" }: Props = $props();
</script>

<!--
  Shield + keyhole, the privacy signature. Three layers:
  1. Primary shield outline (stroke).
  2. Duo fill: the same shield path PLUS keyhole subpaths under
     fill-rule="evenodd", so when the fill turns on the keyhole is a real
     punched hole showing the chrome background through the accent tint.
  3. Primary keyhole stroke (circle + stem) so the glyph reads as
     "privacy" — not a plain empty shield — in the rest state too
     (the v1 reveal-only keyhole left an egg-shaped outline at rest).
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
    d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"
  />
  <path
    class="icon-duo"
    stroke="none"
    fill-rule="evenodd"
    d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z M10.1 10.5a1.9 1.9 0 1 0 3.8 0 1.9 1.9 0 1 0-3.8 0Z M11.1 12h1.8v3.6h-1.8Z"
  />
  <circle cx="12" cy="10.5" r="1.9" />
  <path d="M12 12.4v3" />
</svg>

<style>
  .icon-duo {
    fill: var(--icon-duo, transparent);
    transition: fill var(--duration-normal) var(--ease-out);
  }
</style>
