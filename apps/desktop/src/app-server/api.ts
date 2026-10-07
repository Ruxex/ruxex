import { invoke } from "@tauri-apps/api/core";

import type { ClientRequest, ClientResponse } from "../protocol";

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
