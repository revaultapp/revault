<script lang="ts">
  interface Props {
    size?: number;
    strokeWidth?: number;
    class?: string;
  }

  let { size = 18, strokeWidth = 1.8, class: className = "" }: Props = $props();
</script>

<!--
  Vault — the brand's signature glyph (sidebar "Saved" badge). Safe body
  with feet + handle wheel (duo). The spokes are DIAGONAL on purpose:
  orthogonal spokes read as a crosshair/target at small sizes; diagonals
  read as a valve wheel. The badge has no hover/active state, so
  Sidebar.svelte sets a permanent low-alpha --icon-duo on .saved-badge.
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
  <rect x="3.5" y="3" width="17" height="16.5" rx="3" />
  <circle class="icon-duo" cx="12" cy="11.2" r="3.5" />
  <path d="m14.47 8.73 1.56-1.56" />
  <path d="m7.97 15.23 1.56-1.56" />
  <path d="m14.47 13.67 1.56 1.56" />
  <path d="m7.97 7.17 1.56 1.56" />
  <path d="M7 22v-2.5" />
  <path d="M17 22v-2.5" />
</svg>

<style>
  .icon-duo {
    fill: var(--icon-duo, transparent);
    transition: fill var(--duration-normal) var(--ease-out);
  }
</style>
