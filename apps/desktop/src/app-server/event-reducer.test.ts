import { describe, expect, it } from "vitest";

import type { AppServerEvent } from "./events";
import {
  createInitialEventState,
  reduceAppServerEvent,
} from "./event-reducer";

const notification = (data: unknown): AppServerEvent => ({
  type: "serverNotification",
  data: data as AppServerEvent extends { type: "serverNotification"; data: infer T }
    ? T
    : never,
});

const serverRequest = (data: unknown): AppServerEvent => ({
  type: "serverRequest",
  data: data as AppServerEvent extends { type: "serverRequest"; data: infer T }
    ? T
    : never,
});

describe("reduceAppServerEvent", () => {
  it("accumulates agent message deltas in order and tracks the active turn", () => {
    let state = createInitialEventState();
    state = reduceAppServerEvent(
      state,
      notification({
        method: "turn/started",
        params: {
          threadId: "thread-1",
          turn: { id: "turn-1", status: "inProgress" },
        },
      }),
    );
    state = reduceAppServerEvent(
      state,
      notification({
        method: "item/agentMessage/delta",
        params: {
          threadId: "thread-1",
          turnId: "turn-1",
          itemId: "item-1",
          delta: "Hello",
        },
      }),
    );
    state = reduceAppServerEvent(
      state,
      notification({
        method: "item/agentMessage/delta",
        params: {
          threadId: "thread-1",
          turnId: "turn-1",
          itemId: "item-1",
          delta: " world",
        },
      }),
    );

    expect(state.threads["thread-1"]).toMatchObject({
      activeTurnId: "turn-1",
      status: "running",
      items: [
        {
          id: "item-1",
          type: "agentMessage",
          turnId: "turn-1",
          text: "Hello world",
        },
      ],
    });
  });

  it("marks a turn completed while preserving its streamed items", () => {
    let state = createInitialEventState();
    state = reduceAppServerEvent(
      state,
      notification({
        method: "item/agentMessage/delta",
        params: {
          threadId: "thread-1",
          turnId: "turn-1",
          itemId: "item-1",
          delta: "done",
        },
      }),
    );
    state = reduceAppServerEvent(
      state,
      notification({
        method: "turn/completed",
        params: {
          threadId: "thread-1",
          turn: {
            id: "turn-1",
            status: "completed",
            items: [],
          },
        },
      }),
    );

    expect(state.threads["thread-1"]).toMatchObject({
      activeTurnId: null,
      status: "completed",
      completedTurn: { id: "turn-1", status: "completed" },
      items: [{ id: "item-1", text: "done" }],
    });
  });

  it("keeps approval requests keyed by request id until resolved", () => {
    let state = createInitialEventState();
    const request = serverRequest({
      method: "item/commandExecution/requestApproval",
      id: 42,
      params: {
        kind: "command",
        threadId: "thread-1",
        turnId: "turn-1",
        itemId: "item-1",
        startedAtMs: 1,
        environmentId: null,
        command: "pnpm test",
      },
    });
    state = reduceAppServerEvent(state, request);

    expect(state.pendingServerRequests["42"]).toMatchObject({
      request: (request as Extract<AppServerEvent, { type: "serverRequest" }>).data,
      requestId: 42,
      kind: "approval",
    });

    state = reduceAppServerEvent(
      state,
      notification({
        method: "serverRequest/resolved",
        params: { threadId: "thread-1", requestId: 42 },
      }),
    );

    expect(state.pendingServerRequests).toEqual({});
  });

  it("deduplicates an identical notification without losing ordered history", () => {
    const event = notification({
      method: "item/agentMessage/delta",
      params: {
        threadId: "thread-1",
        turnId: "turn-1",
        itemId: "item-1",
        delta: "once",
      },
    });
    let state = reduceAppServerEvent(createInitialEventState(), event);
    state = reduceAppServerEvent(state, event);

    expect(state.history).toHaveLength(1);
    expect(state.history[0]).toMatchObject({
      id: expect.any(String),
      event,
    });
    expect(state.threads["thread-1"]?.items[0]?.text).toBe("once");
  });

  it("marks the state as resynchronizing after lagged or disconnected events", () => {
    let state = reduceAppServerEvent(
      createInitialEventState(),
      { type: "lagged", data: { skipped: 3 } },
    );
    expect(state).toMatchObject({
      connected: true,
      resynchronizing: true,
      skippedEvents: 3,
    });

    state = reduceAppServerEvent(state, {
      type: "disconnected",
      data: { message: "socket closed" },
    });
    expect(state).toMatchObject({
      connected: false,
      resynchronizing: true,
      lastError: "socket closed",
    });
  });
});
