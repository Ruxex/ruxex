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
let starting: Promise<void> | undefined;
let lifecycleGeneration = 0;

function notify() {
  for (const listener of listeners) {
    listener();
  }
}

export async function startAppServerEventStore(): Promise<void> {
  if (!isDesktopRuntime || unlisten) {
    return;
  }
  if (starting) {
    return starting;
  }

  const generation = ++lifecycleGeneration;
  const start = (async () => {
    const nextUnlisten = await listenToAppServerEvents((event) => {
      snapshot = reduceAppServerEvent(snapshot, event);
      notify();
    });
    if (generation !== lifecycleGeneration) {
      nextUnlisten();
      return;
    }
    unlisten = nextUnlisten;
    snapshot = { ...snapshot, connected: true };
    notify();
  })();
  starting = start;
  start.then(
    () => {
      if (starting === start) {
        starting = undefined;
      }
    },
    () => {
      if (starting === start) {
        starting = undefined;
      }
    },
  );
  return start;
}

export function stopAppServerEventStore(): void {
  lifecycleGeneration += 1;
  starting = undefined;
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
