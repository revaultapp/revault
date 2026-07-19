<script lang="ts">
  interface Props { size?: number; strokeWidth?: number; class?: string; }
  let { size = 18, strokeWidth = 1.8, class: className = "" }: Props = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width={strokeWidth} stroke-linecap="round" stroke-linejoin="round" class={className} aria-hidden="true">
  <path d="m3.5 18 3-12 3 12M4.6 13h3.8" />
  <path d="M14 6h6M17 4v2M17 6c0 5-1.5 8.5-4 11M15 10c1.1 2.9 2.9 5.4 5.5 7" />
  <circle class="icon-duo" cx="20" cy="18" r="1.25" stroke="none" />
</svg>

<style>
  .icon-duo { fill: var(--icon-duo, transparent); transition: fill var(--duration-normal) var(--ease-out); }
</style>
