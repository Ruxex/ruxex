import { describe, expect, it } from "vitest";

import type { PendingServerRequest } from "../../app-server/event-reducer";
import type { AppServerEvent } from "../../app-server/events";
import {
  buildApprovalResponse,
  buildUserInputResponse,
  isRequestActionable,
  canResolvePendingRequest,
} from "./approval-model";

const commandRequest = {
  type: "serverRequest",
  data: {
    method: "item/commandExecution/requestApproval",
    id: 17,
    params: {
      kind: "command",
      threadId: "thread-1",
      turnId: "turn-1",
      itemId: "item-1",
      startedAtMs: 1,
      environmentId: null,
      command: "pnpm check",
    },
  },
} as AppServerEvent;

const pending = {
  requestId: 17,
  kind: "approval",
  request: commandRequest.data,
  threadId: "thread-1",
  turnId: "turn-1",
  itemId: "item-1",
} as PendingServerRequest;

describe("approval request model", () => {
  it("builds exact command approval results for the original request id", () => {
    expect(buildApprovalResponse(pending.request, "accept")).toEqual({
      decision: "accept",
    });
    expect(pending.request.id).toBe(17);
  });

  it("builds user-input responses without auto-approving unknown requests", () => {
    const request = {
      method: "item/tool/requestUserInput",
      id: "input-1",
      params: {
        threadId: "thread-1",
        turnId: "turn-1",
        itemId: "item-2",
        questions: [],
        isBlocking: true,
      },
    } as never;
    expect(buildUserInputResponse(request, { answer: { answers: ["yes"] } })).toEqual({
      answers: { answer: { answers: ["yes"] } },
    });
    expect(buildApprovalResponse(request, "accept")).toBeNull();
  });

  it("rejects malformed/unknown payloads and keeps disconnected requests unresolved", () => {
    expect(isRequestActionable({ method: "unknown", id: 1 })).toBe(false);
    expect(isRequestActionable(commandRequest.data)).toBe(true);
    expect(canResolvePendingRequest(pending, false)).toBe(false);
    expect(canResolvePendingRequest(pending, true)).toBe(true);
  });
});
