import { invoke } from "@tauri-apps/api/core";

import type { ClientRequest, ClientResponse, RequestId } from "../protocol";

export type AppServerStatus = {
  socketPath: string;
  appServerVersion: string;
};

type ResponseFor<Request extends ClientRequest> = Extract<
  ClientResponse,
  { method: Request["method"] }
>;

type RequestForMethod<Method extends ClientRequest["method"]> = Extract<
  ClientRequest,
  { method: Method }
>;

type ResponseForMethod<Method extends ClientRequest["method"]> = Extract<
  ClientResponse,
  { method: Method }
>;

let requestId = 0;

function nextRequestId(): number {
  requestId += 1;
  return requestId;
}

export function ensureAppServer(): Promise<AppServerStatus> {
  return invoke<AppServerStatus>("ensure_app_server");
}

export function requestAppServer<Request extends ClientRequest>(
  request: Request,
): Promise<ResponseFor<Request>> {
  return invoke<ClientResponse>("app_server_request", { request }) as Promise<
    ResponseFor<Request>
  >;
}

export function requestAppServerMethod<Method extends ClientRequest["method"]>(
  method: Method,
  params: RequestForMethod<Method>["params"],
): Promise<ResponseForMethod<Method>> {
  const request = {
    method,
    id: nextRequestId(),
    params,
  } as RequestForMethod<Method>;
  return requestAppServer(request) as Promise<ResponseForMethod<Method>>;
}

export function resolveServerRequest(
  requestId: RequestId,
  result: unknown,
): Promise<void> {
  return invoke("app_server_resolve_server_request", { requestId, result });
}

export type AppServerRequestError = {
  code: number;
  data?: unknown;
  message: string;
};

export function rejectServerRequest(
  requestId: RequestId,
  error: AppServerRequestError,
): Promise<void> {
  return invoke("app_server_reject_server_request", { requestId, error });
}

export function rejectServerRequestWithMessage(
  requestId: RequestId,
  message: string,
): Promise<void> {
  return rejectServerRequest(requestId, { code: -32000, message });
}
