import { Tween, prefersReducedMotion } from "svelte/motion";
import { cubicOut } from "svelte/easing";

/**
 * A headline number (savings MB, %) that eases toward its target.
 * ponytail: Tween animates via requestAnimationFrame, not CSS, so the
 * blanket `prefers-reduced-motion` rule in app.css never reaches it — snap
 * straight to the target here instead when the user has that preference set.
 */
export function animatedNumber(value: number, duration = 600) {
  const tween = new Tween(value, { duration, easing: cubicOut });
  return {
    get current() {
      return tween.current;
    },
    set(next: number) {
      return tween.set(next, {
        duration: prefersReducedMotion.current ? 0 : duration,
      });
    },
  };
}
