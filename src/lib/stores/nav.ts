import { writable } from "svelte/store";

export const activePage = writable("home");
export const activeTool = writable<"compress" | "convert" | "resize">("compress");
