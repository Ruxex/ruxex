import { useQuery } from "@tanstack/react-query";

import type { v2 } from "../protocol";
import { requestAppServerMethod } from "./api";
import { appServerQueryKeys } from "./query-keys";
import { useAppServerStatus } from "./runtime-queries";

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

export function useModelProvider() {
  const status = useAppServerStatus();

  return useQuery<v2.ModelProviderReadResponse | null>({
    queryKey: appServerQueryKeys.provider,
    queryFn: async () => {
      const response = await requestAppServerMethod("modelProvider/read", {});
      return response.response;
    },
    enabled: Boolean(status.data),
    staleTime: 5 * 60_000,
  });
}

export function useModelProviderCapabilities() {
  const status = useAppServerStatus();

  return useQuery<v2.ModelProviderCapabilitiesReadResponse | null>({
    queryKey: appServerQueryKeys.providerCapabilities,
    queryFn: async () => {
      const response = await requestAppServerMethod(
        "modelProvider/capabilities/read",
        {},
      );
      return response.response;
    },
    enabled: Boolean(status.data),
    staleTime: 5 * 60_000,
  });
}

export function usePermissionProfiles() {
  const status = useAppServerStatus();

  return useQuery<v2.PermissionProfileListResponse | null>({
    queryKey: appServerQueryKeys.permissionProfiles,
    queryFn: async () => {
      const response = await requestAppServerMethod("permissionProfile/list", {});
      return response.response;
    },
    enabled: Boolean(status.data),
    staleTime: 5 * 60_000,
  });
}
