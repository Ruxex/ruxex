import { useAppServerEventStore } from "../event-store";

export function useAppServerEvents() {
  return useAppServerEventStore();
}

export function useServerRequests() {
  return useAppServerEventStore().serverRequests;
}
