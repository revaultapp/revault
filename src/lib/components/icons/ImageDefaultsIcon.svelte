<script lang="ts">
  interface Props {
    size?: number;
    strokeWidth?: number;
    class?: string;
  }

  let { size = 18, strokeWidth = 1.8, class: className = "" }: Props = $props();
</script>

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
  <rect x="3" y="4" width="18" height="16" rx="2" />
  <path class="icon-duo" stroke="none" d="m5 17 4.5-5 3 3 2.5-3 4 5Z" />
  <path d="m5 17 4.5-5 3 3 2.5-3 4 5M8 8h.01M9 19v-2M12 19v-2M15 19v-2" />
</svg>

<style>
  .icon-duo {
    fill: var(--icon-duo, transparent);
    transition: fill var(--duration-normal) var(--ease-out);
  }
</style>
