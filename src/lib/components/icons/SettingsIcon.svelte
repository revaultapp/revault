<script lang="ts">
  interface Props {
    size?: number;
    strokeWidth?: number;
    class?: string;
  }

  let { size = 18, strokeWidth = 1.8, class: className = "" }: Props = $props();
</script>

<!--
  Settings as staggered sliders instead of a gear: separate gear teeth
  turned to mush at 18px (the v1 lesson), while three tracks with round
  knobs stay crisp at any size and match what the page actually holds
  (toggles/options). The knobs are the duo layer — they light up green
  like adjusted controls. See icons/index.ts for the shared mechanism.
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
  <path d="M3.5 5h17" />
  <path d="M3.5 12h17" />
  <path d="M3.5 19h17" />
  <circle class="icon-duo" cx="15" cy="5" r="2.2" />
  <circle class="icon-duo" cx="8" cy="12" r="2.2" />
  <circle class="icon-duo" cx="16" cy="19" r="2.2" />
</svg>

<style>
  .icon-duo {
    fill: var(--icon-duo, transparent);
    transition: fill var(--duration-normal) var(--ease-out);
  }
</style>
