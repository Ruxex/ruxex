import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import type { v2 } from "../../protocol";
import { ensureAppServer, requestAppServerMethod } from "../api";
import { useAppServerStatus } from "../queries";
import { appServerQueryKeys } from "../query-keys";

export function useAccount(params: v2.GetAccountParams = {}) {
  const status = useAppServerStatus();
  const refreshToken = params.refreshToken ?? false;

  return useQuery({
    queryKey: appServerQueryKeys.accountRead(refreshToken),
    queryFn: async () => {
      const response = await requestAppServerMethod("account/read", params);
      return response.response;
    },
    enabled: Boolean(status.data),
  });
}

export function useLogin() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (params: v2.LoginAccountParams) => {
      await ensureAppServer();
      const response = await requestAppServerMethod(
        "account/login/start",
        params,
      );
      return response.response;
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: appServerQueryKeys.account,
      });
    },
  });
}

export function useCancelLogin() {
  return useMutation({
    mutationFn: async (params: v2.CancelLoginAccountParams) => {
      const response = await requestAppServerMethod(
        "account/login/cancel",
        params,
      );
      return response.response;
    },
  });
}

export function useLogout() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async () => {
      const response = await requestAppServerMethod(
        "account/logout",
        undefined,
      );
      return response.response;
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: appServerQueryKeys.account,
      });
    },
  });
}
