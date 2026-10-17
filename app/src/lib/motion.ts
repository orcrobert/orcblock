/** Whether the user asks for reduced motion right now. Read at the moment an
    animation starts, so changing the setting while the app is open applies
    to the next animation. */
export function prefersReducedMotion(): boolean {
  return typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
}
