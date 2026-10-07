import { useQuery } from "@tanstack/react-query";

import type { ClientRequest, v2 } from "../protocol";
import { ensureAppServer, requestAppServer } from "./api";
import { isDesktopRuntime } from "./runtime";

let requestId = 0;

function nextRequestId(): number {
  requestId += 1;
  return requestId;
}

export function useAppServerStatus() {
  return useQuery({
    queryKey: ["app-server", "status"],
    queryFn: () => (isDesktopRuntime ? ensureAppServer() : Promise.resolve(null)),
    enabled: isDesktopRuntime,
    retry: false,
    staleTime: Number.POSITIVE_INFINITY,
  });
}

export function useModelList() {
  const status = useAppServerStatus();
  return useQuery<v2.ModelListResponse | null>({
    queryKey: ["app-server", "models"],
    queryFn: async () => {
      if (!status.data) {
        return null;
      }
      const request: Extract<ClientRequest, { method: "model/list" }> = {
        method: "model/list",
        id: nextRequestId(),
        params: {},
      };
      const response = await requestAppServer(request);
      return response.response;
    },
    enabled: Boolean(status.data),
    staleTime: 5 * 60_000,
  });
}
