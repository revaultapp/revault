<script lang="ts">
  let { checked = $bindable(false) } = $props();
</script>

<button class="toggle" class:on={checked} onclick={() => checked = !checked} aria-pressed={checked} aria-label="Toggle">
  <span class="knob"></span>
</button>

<style>
  .toggle {
    width: 44px;
    height: 24px;
    border-radius: 12px;
    background: var(--toggle-off);
    position: relative;
    transition: background 0.2s;
    flex-shrink: 0;
  }

  .toggle.on {
    background: var(--accent);
  }

  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 20px;
    height: 20px;
    border-radius: 10px;
    background: #fff;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
    transition: transform 0.2s;
  }

  .toggle.on .knob {
    transform: translateX(20px);
  }
</style>
