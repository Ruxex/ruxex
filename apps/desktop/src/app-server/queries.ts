import { useQuery } from "@tanstack/react-query";

import type { v2 } from "../protocol";
import { ensureAppServer, requestAppServerMethod } from "./api";
import { appServerQueryKeys } from "./query-keys";
import { isDesktopRuntime } from "./runtime";

export function useAppServerStatus() {
  return useQuery({
    queryKey: appServerQueryKeys.status,
    queryFn: () => (isDesktopRuntime ? ensureAppServer() : Promise.resolve(null)),
    enabled: isDesktopRuntime,
    retry: false,
    staleTime: Number.POSITIVE_INFINITY,
  });
}

export function useModelList() {
  const status = useAppServerStatus();
  return useQuery<v2.ModelListResponse | null>({
    queryKey: appServerQueryKeys.models,
    queryFn: async () => {
      if (!status.data) {
        return null;
      }
      const response = await requestAppServerMethod("model/list", {});
      return response.response;
    },
    enabled: Boolean(status.data),
    staleTime: 5 * 60_000,
  });
}
