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
});
