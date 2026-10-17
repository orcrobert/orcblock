/** Svelte action: hides the node until a quarter of it is on screen. */
export function reveal(node: HTMLElement) {
  if (typeof IntersectionObserver === "undefined") return;
  node.dataset.reveal = "hidden";
  const observer = new IntersectionObserver(
    (entries) => {
      if (entries.some((entry) => entry.isIntersecting)) {
        node.dataset.reveal = "shown";
        observer.disconnect();
      }
    },
    { threshold: 0.25 },
  );
  observer.observe(node);
  return { destroy: () => observer.disconnect() };
}
