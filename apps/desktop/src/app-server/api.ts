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

export function resolveServerRequest(requestId: RequestId, result: unknown): Promise<void> {
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
