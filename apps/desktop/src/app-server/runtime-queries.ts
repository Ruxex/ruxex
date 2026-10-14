import { useQuery } from "@tanstack/react-query";

import { ensureAppServer } from "./api";
import { appServerQueryKeys } from "./query-keys";
import { isDesktopRuntime } from "./runtime";

export function useAppServerStatus() {
  return useQuery({
    queryKey: appServerQueryKeys.status,
    queryFn: () =>
      isDesktopRuntime ? ensureAppServer() : Promise.resolve(null),
    enabled: isDesktopRuntime,
    retry: false,
    staleTime: Number.POSITIVE_INFINITY,
  });
}
