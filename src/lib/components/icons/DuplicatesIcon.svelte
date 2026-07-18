<script lang="ts">
  interface Props {
    size?: number;
    strokeWidth?: number;
    class?: string;
  }

  let { size = 18, strokeWidth = 1.8, class: className = "" }: Props = $props();
</script>

<!--
  Duplicates — two stacked PHOTOS (the plain two-frame "copy" of v2 read
  as a generic clipboard action). The front photo is the duo layer; its
  mountain is drawn after the rect so the stroke sits on top of the
  accent fill when the item activates — the duplicate photo lights up
  over the original peeking from behind.
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
  <path d="M18 4.5H7a2 2 0 0 0-2 2v11" />
  <rect class="icon-duo" x="8.5" y="8" width="13" height="12" rx="2" />
  <path d="m9.5 17 3.5-3.5a1.6 1.6 0 0 1 2.3 0l4.2 4.2" />
</svg>

<style>
  .icon-duo {
    fill: var(--icon-duo, transparent);
    transition: fill var(--duration-normal) var(--ease-out);
  }
</style>
