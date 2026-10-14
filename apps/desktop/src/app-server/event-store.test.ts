import { beforeEach, describe, expect, it, vi } from "vitest";

import type { AppServerEvent } from "./events";

const listenToAppServerEvents = vi.hoisted(() => vi.fn());

vi.mock("./events", () => ({ listenToAppServerEvents }));
vi.mock("./runtime", () => ({ isDesktopRuntime: true }));

describe("app-server event store", () => {
  beforeEach(async () => {
    listenToAppServerEvents.mockReset();
    const store = await import("./event-store");
    store.stopAppServerEventStore();
  });

  it("exposes reducer state and marks a lagged stream for resynchronization", async () => {
    let handler: ((event: AppServerEvent) => void) | undefined;
    listenToAppServerEvents.mockImplementation(async (next: (event: AppServerEvent) => void) => {
      handler = next;
      return () => undefined;
    });

    const store = await import("./event-store");
    await store.startAppServerEventStore();
    handler?.({ type: "lagged", data: { skipped: 2 } });

    expect(store.getAppServerEventSnapshot()).toMatchObject({
      connected: true,
      resynchronizing: true,
      skippedEvents: 2,
    });
  });

  it("does not leak a listener when StrictMode restarts before listen resolves", async () => {
    let resolveFirst: ((unlisten: () => void) => void) | undefined;
    let resolveSecond: ((unlisten: () => void) => void) | undefined;
    const firstUnlisten = vi.fn();
    const secondUnlisten = vi.fn();
    listenToAppServerEvents
      .mockImplementationOnce(
        () => new Promise((resolve) => {
          resolveFirst = resolve;
        }),
      )
      .mockImplementationOnce(
        () => new Promise((resolve) => {
          resolveSecond = resolve;
        }),
      );

    const store = await import("./event-store");
    const firstStart = store.startAppServerEventStore();
    store.stopAppServerEventStore();
    const secondStart = store.startAppServerEventStore();

    resolveFirst?.(firstUnlisten);
    await firstStart;
    expect(firstUnlisten).toHaveBeenCalledTimes(1);

    resolveSecond?.(secondUnlisten);
    await secondStart;
    store.stopAppServerEventStore();
    expect(secondUnlisten).toHaveBeenCalledTimes(1);
  });
});
