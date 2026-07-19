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
  <path
    class="icon-duo"
    d="M20.4 15.2A8.8 8.8 0 0 1 8.8 3.6 8.8 8.8 0 1 0 20.4 15.2Z"
  />
  <path d="M20.4 15.2A8.8 8.8 0 0 1 8.8 3.6 8.8 8.8 0 1 0 20.4 15.2Z" />
  <path d="m17.8 4.6.5 1.1 1.1.5-1.1.5-.5 1.1-.5-1.1-1.1-.5 1.1-.5Z" />
</svg>

<style>
  .icon-duo {
    fill: var(--icon-duo, transparent);
    transition: fill var(--duration-normal) var(--ease-out);
  }
</style>
