import { writable } from "svelte/store";

const stored = typeof localStorage !== "undefined" ? localStorage.getItem("theme") : null;

export const theme = writable<"light" | "dark">((stored as "light" | "dark") ?? "light");

theme.subscribe((t) => {
  if (typeof document === "undefined") return;
  if (t === "dark") {
    document.documentElement.setAttribute("data-theme", "dark");
  } else {
    document.documentElement.removeAttribute("data-theme");
  }
  localStorage.setItem("theme", t);
});
