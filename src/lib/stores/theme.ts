import { writable } from "svelte/store";

export type Theme = "light" | "dark" | "system";

const stored = typeof localStorage !== "undefined" ? localStorage.getItem("theme") : null;

const isTheme = (v: string | null): v is Theme =>
  v === "light" || v === "dark" || v === "system";

// New installations follow the operating system. Membership check (not a cast)
// keeps a stale/corrupted stored value on that same safe default.
export const theme = writable<Theme>(isTheme(stored) ? stored : "system");

function prefersDark(): boolean {
  return typeof matchMedia !== "undefined" && matchMedia("(prefers-color-scheme: dark)").matches;
}

function resolve(t: Theme): "light" | "dark" {
  return t === "system" ? (prefersDark() ? "dark" : "light") : t;
}

function applyTheme(t: Theme) {
  if (typeof document === "undefined") return;
  if (resolve(t) === "dark") {
    document.documentElement.setAttribute("data-theme", "dark");
  } else {
    document.documentElement.removeAttribute("data-theme");
  }
}

let unlistenSystem: (() => void) | undefined;

theme.subscribe((t) => {
  if (typeof localStorage !== "undefined") localStorage.setItem("theme", t);
  applyTheme(t);

  unlistenSystem?.();
  unlistenSystem = undefined;

  if (t === "system" && typeof matchMedia !== "undefined") {
    const mql = matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => applyTheme("system");
    mql.addEventListener("change", onChange);
    unlistenSystem = () => mql.removeEventListener("change", onChange);
  }
});
