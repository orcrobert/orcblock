import { describe, expect, it } from "vitest";
import { waitFor } from "./wait";

const noSleep = () => Promise.resolve();

function failingTimes(failures: number) {
  let calls = 0;
  const probe = () => {
    calls += 1;
    return calls > failures ? Promise.resolve("up") : Promise.reject(new Error("down"));
  };
  return { probe, calls: () => calls };
}

describe("waitFor", () => {
  it("returns at once when the probe succeeds", async () => {
    const { probe, calls } = failingTimes(0);
    expect(await waitFor(probe, { attempts: 5, intervalMs: 10 }, noSleep)).toBe("up");
    expect(calls()).toBe(1);
  });

  it("retries until the probe succeeds", async () => {
    const { probe, calls } = failingTimes(3);
    expect(await waitFor(probe, { attempts: 5, intervalMs: 10 }, noSleep)).toBe("up");
    expect(calls()).toBe(4);
  });

  it("gives up after the last attempt without sleeping again", async () => {
    const { probe, calls } = failingTimes(99);
    let sleeps = 0;
    const sleep = () => {
      sleeps += 1;
      return Promise.resolve();
    };
    expect(await waitFor(probe, { attempts: 3, intervalMs: 10 }, sleep)).toBeNull();
    expect(calls()).toBe(3);
    expect(sleeps).toBe(2);
  });
});
