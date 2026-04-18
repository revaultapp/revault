<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";

  interface Props extends HTMLButtonAttributes {
    variant?: "primary" | "ghost";
    size?: "sm" | "md";
    danger?: boolean;
    alignSelf?: string;
    children: Snippet;
  }

  let {
    variant = "primary",
    size = "md",
    danger = false,
    alignSelf,
    children,
    style: restStyle,
    class: extraClass,
    ...rest
  }: Props = $props();

  let buttonClass = $derived(
    ["btn-" + variant, size === "sm" ? "btn-sm" : "", danger ? "btn-danger" : "", extraClass ?? ""].filter(Boolean).join(" ")
  );

  let style = $derived(
    [alignSelf ? `align-self: ${alignSelf}` : "", restStyle ?? ""].filter(Boolean).join("; ")
  );
</script>

<button
  class={buttonClass}
  {style}
  {...rest}
>
  {@render children()}
</button>

<style>
  .btn-primary {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 10px 28px;
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: #fff;
    font-size: 14px;
    font-weight: 600;
    border: none;
    cursor: pointer;
    transition: opacity 0.15s, transform 0.1s;
    white-space: nowrap;
  }

  .btn-primary:hover {
    opacity: 0.9;
    transform: translateY(-1px);
  }

  .btn-primary:active {
    transform: translateY(0) scale(0.98);
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
    transform: none;
  }

  .btn-primary:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .btn-danger {
    background: var(--danger);
  }

  .btn-danger:hover {
    background: #dc2626;
  }

  .btn-ghost {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 14px;
    border-radius: var(--radius-sm);
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    border: 1px solid var(--border);
    background: none;
    cursor: pointer;
    transition: background 0.15s, transform 0.1s;
    white-space: nowrap;
  }

  .btn-ghost:hover {
    background: var(--navy-bg);
  }

  .btn-ghost:active {
    transform: scale(0.98);
  }

  .btn-ghost:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-ghost:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .btn-sm {
    padding: 6px 16px;
    font-size: 13px;
  }
</style>
