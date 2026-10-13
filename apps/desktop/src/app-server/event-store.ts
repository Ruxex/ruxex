import { useSyncExternalStore } from "react";

import {
  createInitialEventState,
  reduceAppServerEvent,
  type EventState,
} from "./event-reducer";
import { listenToAppServerEvents } from "./events";
import { isDesktopRuntime } from "./runtime";

export type EventSnapshot = EventState;

const listeners = new Set<() => void>();
let snapshot: EventSnapshot = createInitialEventState();
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
    snapshot = reduceAppServerEvent(snapshot, event);
    notify();
  });
  snapshot = { ...snapshot, connected: true };
  notify();
}

export function stopAppServerEventStore(): void {
  unlisten?.();
  unlisten = undefined;
  snapshot = { ...createInitialEventState(), connected: false };
  notify();
}

export function getAppServerEventSnapshot(): EventSnapshot {
  return snapshot;
}

export function useAppServerEventStore(): EventSnapshot {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => snapshot,
    () => createInitialEventState(),
  );
}
