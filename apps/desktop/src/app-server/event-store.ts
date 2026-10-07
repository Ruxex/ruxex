import { useSyncExternalStore } from "react";

import { listenToAppServerEvents, type AppServerEvent } from "./events";
import { isDesktopRuntime } from "./runtime";

type EventSnapshot = {
  events: AppServerEvent[];
  connected: boolean;
};

const listeners = new Set<() => void>();
let snapshot: EventSnapshot = { events: [], connected: false };
let unlisten: (() => void) | undefined;

function notify() {
  for (const listener of listeners) {
    listener();
  }
}

export async function startAppServerEventStore(): Promise<void> {
  if (!isDesktopRuntime || unlisten) {
    return;
  }
  unlisten = await listenToAppServerEvents((event) => {
    snapshot = {
      connected: event.type !== "disconnected",
      events: [...snapshot.events, event].slice(-100),
    };
    notify();
  });
  snapshot = { ...snapshot, connected: true };
  notify();
}

export function stopAppServerEventStore(): void {
  unlisten?.();
  unlisten = undefined;
  snapshot = { ...snapshot, connected: false };
  notify();
}

export function useAppServerEventStore(): EventSnapshot {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => snapshot,
    () => ({ events: [], connected: false }),
  );
}
