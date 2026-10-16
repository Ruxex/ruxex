import type { ServerRequest } from "../../protocol";
import type { PendingServerRequest } from "../../app-server/event-reducer";

export type ApprovalDecision = "accept" | "acceptForSession" | "decline" | "cancel";

export function buildApprovalResponse(
  request: ServerRequest,
  decision: ApprovalDecision,
): { decision: ApprovalDecision } | null {
  if (
    request.method === "item/commandExecution/requestApproval" ||
    request.method === "item/fileChange/requestApproval"
  ) {
    return { decision };
  }
  return null;
}

export function buildUserInputResponse(
  request: ServerRequest,
  answers: Record<string, { answers: string[] }>,
): { answers: Record<string, { answers: string[] }> } | null {
  return request.method === "item/tool/requestUserInput" ? { answers } : null;
}

export function isRequestActionable(value: unknown): value is ServerRequest {
  if (!value || typeof value !== "object") {
    return false;
  }
  const request = value as Partial<ServerRequest>;
  return (
    typeof request.method === "string" &&
    (typeof request.id === "string" || typeof request.id === "number") &&
    "params" in request
  );
}

export function canResolvePendingRequest(
  request: PendingServerRequest,
  connected: boolean,
): boolean {
  return connected && isRequestActionable(request.request);
}
