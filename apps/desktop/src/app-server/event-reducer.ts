import type { RequestId, ServerNotification, ServerRequest, v2 } from "../protocol";

import type { AppServerEvent } from "./events";

export type LiveConversationItem = {
  id: string;
  threadId: string;
  turnId: string;
  type: "agentMessage" | "plan" | "reasoning";
  text: string;
};

export type ThreadExecutionState = {
  threadId: string;
  activeTurnId: string | null;
  status:
    | "idle"
    | "running"
    | "completed"
    | "interrupted"
    | "failed"
    | "resynchronizing";
  items: LiveConversationItem[];
  completedTurn: v2.Turn | null;
};

export type PendingServerRequest = {
  requestId: RequestId;
  kind: "approval" | "userInput" | "elicitation" | "dynamicToolCall" | "other";
  request: ServerRequest;
  threadId: string | null;
  turnId: string | null;
  itemId: string | null;
};

export type EventHistoryEntry = {
  id: string;
  event: AppServerEvent;
};

export type EventState = {
  connected: boolean;
  resynchronizing: boolean;
  skippedEvents: number;
  lastError: string | null;
  history: EventHistoryEntry[];
  pendingServerRequests: Record<string, PendingServerRequest>;
  threads: Record<string, ThreadExecutionState>;
  seenEventIds: ReadonlySet<string>;
};

const MAX_HISTORY = 200;

export function createInitialEventState(): EventState {
  return {
    connected: false,
    resynchronizing: false,
    skippedEvents: 0,
    lastError: null,
    history: [],
    pendingServerRequests: {},
    threads: {},
    seenEventIds: new Set(),
  };
}

export function reduceAppServerEvent(
  state: EventState,
  event: AppServerEvent,
): EventState {
  const eventId = eventIdentity(event);
  if (state.seenEventIds.has(eventId)) {
    return state;
  }

  const seenEventIds = new Set(state.seenEventIds);
  seenEventIds.add(eventId);
  const history = [
    ...state.history,
    { id: eventId, event },
  ].slice(-MAX_HISTORY);

  const nextState: EventState = {
    ...state,
    connected: event.type !== "disconnected",
    history,
    seenEventIds,
  };

  if (event.type === "lagged") {
    return {
      ...nextState,
      connected: true,
      resynchronizing: true,
      skippedEvents: state.skippedEvents + event.data.skipped,
    };
  }

  if (event.type === "disconnected") {
    return {
      ...nextState,
      connected: false,
      resynchronizing: true,
      lastError: event.data.message,
      threads: Object.fromEntries(
        Object.entries(state.threads).map(([threadId, thread]) => [threadId, {
          ...thread,
          status: thread.activeTurnId ? "resynchronizing" : thread.status,
        }]),
      ),
    };
  }

  if (event.type === "serverRequest") {
    const request = event.data;
    const requestId = request.id;
    const params = request.params as { threadId?: string; turnId?: string; itemId?: string };
    return {
      ...nextState,
      pendingServerRequests: {
        ...state.pendingServerRequests,
        [requestKey(requestId)]: {
          requestId,
          kind: serverRequestKind(request),
          request,
          threadId: params.threadId ?? null,
          turnId: params.turnId ?? null,
          itemId: params.itemId ?? null,
        },
      },
    };
  }

  return reduceNotification(nextState, event.data);
}

function reduceNotification(state: EventState, notification: ServerNotification): EventState {
  switch (notification.method) {
    case "turn/started": {
      const { threadId, turn } = notification.params;
      return updateThread(state, threadId, (thread) => ({
        ...thread,
        activeTurnId: turn.id,
        status: "running",
        completedTurn: null,
      }));
    }
    case "turn/completed": {
      const { threadId, turn } = notification.params;
      return updateThread(state, threadId, (thread) => ({
        ...thread,
        activeTurnId: null,
        status: turnStatus(turn.status),
        completedTurn: turn,
      }));
    }
    case "item/agentMessage/delta":
      return addDelta(state, notification.params, "agentMessage");
    case "item/plan/delta":
      return addDelta(state, notification.params, "plan");
    case "item/reasoning/textDelta":
      return addDelta(state, notification.params, "reasoning");
    case "serverRequest/resolved": {
      const requestId = requestKey(notification.params.requestId);
      if (!(requestId in state.pendingServerRequests)) {
        return state;
      }
      const pendingServerRequests = { ...state.pendingServerRequests };
      delete pendingServerRequests[requestId];
      return { ...state, pendingServerRequests };
    }
    default:
      return state;
  }
}

function addDelta(
  state: EventState,
  params: { threadId: string; turnId: string; itemId: string; delta: string },
  type: LiveConversationItem["type"],
): EventState {
  return updateThread(state, params.threadId, (thread) => {
    const index = thread.items.findIndex((item) => item.id === params.itemId);
    if (index === -1) {
      return {
        ...thread,
        activeTurnId: thread.activeTurnId ?? params.turnId,
        status: "running",
        items: [
          ...thread.items,
          {
            id: params.itemId,
            threadId: params.threadId,
            turnId: params.turnId,
            type,
            text: params.delta,
          },
        ],
      };
    }

    const item = thread.items[index];
    const items = [...thread.items];
    items[index] = { ...item, text: item.text + params.delta };
    return { ...thread, activeTurnId: thread.activeTurnId ?? params.turnId, status: "running", items };
  });
}

function updateThread(
  state: EventState,
  threadId: string,
  update: (thread: ThreadExecutionState) => ThreadExecutionState,
): EventState {
  const current = state.threads[threadId] ?? {
    threadId,
    activeTurnId: null,
    status: "idle" as const,
    items: [],
    completedTurn: null,
  };
  return {
    ...state,
    threads: {
      ...state.threads,
      [threadId]: update(current),
    },
  };
}

function eventIdentity(event: AppServerEvent): string {
  if (event.type === "serverRequest") {
    return `serverRequest:${requestKey(event.data.id)}`;
  }
  return `${event.type}:${stableJson(event.data)}`;
}

function requestKey(requestId: RequestId): string {
  return String(requestId);
}

function serverRequestKind(request: ServerRequest): PendingServerRequest["kind"] {
  const method = typeof request.method === "string" ? request.method : "";
  if (method.includes("requestApproval")) {
    return "approval";
  }
  if (request.method === "item/tool/requestUserInput") {
    return "userInput";
  }
  if (request.method === "mcpServer/elicitation/request") {
    return "elicitation";
  }
  if (request.method === "item/tool/call") {
    return "dynamicToolCall";
  }
  return "other";
}

function turnStatus(status: v2.Turn["status"]): ThreadExecutionState["status"] {
  return status === "inProgress" ? "running" : status;
}

function stableJson(value: unknown): string {
  if (value === null || typeof value !== "object") {
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    return `[${value.map(stableJson).join(",")}]`;
  }
  const record = value as Record<string, unknown>;
  return `{${Object.keys(record)
    .sort()
    .map((key) => `${JSON.stringify(key)}:${stableJson(record[key])}`)
    .join(",")}}`;
}
