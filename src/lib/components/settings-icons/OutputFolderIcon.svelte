<script lang="ts">
  interface Props { size?: number; strokeWidth?: number; class?: string; }
  let { size = 18, strokeWidth = 1.8, class: className = "" }: Props = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width={strokeWidth} stroke-linecap="round" stroke-linejoin="round" class={className} aria-hidden="true">
  <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z" />
  <path class="icon-duo" stroke="none" d="M5 10h14v7H5Z" />
  <path d="M12 12v4M10.5 14.5 12 16l1.5-1.5" />
</svg>

<style>
  .icon-duo { fill: var(--icon-duo, transparent); transition: fill var(--duration-normal) var(--ease-out); }
</style>
