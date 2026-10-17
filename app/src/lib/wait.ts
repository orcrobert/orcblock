/** Calls `probe` every `intervalMs` until it resolves, up to `attempts` times; `null` when it never does. */
export async function waitFor<T>(
  probe: () => Promise<T>,
  { attempts, intervalMs }: { attempts: number; intervalMs: number },
  sleep: (ms: number) => Promise<void> = (ms) => new Promise((done) => setTimeout(done, ms)),
): Promise<T | null> {
  for (let attempt = 0; attempt < attempts; attempt++) {
    try {
      return await probe();
    } catch {
      if (attempt + 1 < attempts) await sleep(intervalMs);
    }
  }
  return null;
}
