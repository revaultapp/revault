<script lang="ts">
  interface Props { size?: number; strokeWidth?: number; class?: string; }
  let { size = 18, strokeWidth = 1.8, class: className = "" }: Props = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width={strokeWidth} stroke-linecap="round" stroke-linejoin="round" class={className} aria-hidden="true">
  <rect x="3" y="4" width="18" height="12" rx="2" />
  <path class="icon-duo" stroke="none" d="M5 6h14v8H5Z" />
  <path d="M9 20h6M12 16v4" />
</svg>

<style>
  .icon-duo { fill: var(--icon-duo, transparent); transition: fill var(--duration-normal) var(--ease-out); }
</style>
