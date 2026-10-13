import { useMutation, useQueryClient } from "@tanstack/react-query";

import type { v2 } from "../../protocol";
import { requestAppServerMethod } from "../api";
import { appServerQueryKeys } from "../query-keys";

function invalidateThreadList(queryClient: ReturnType<typeof useQueryClient>) {
  void queryClient.invalidateQueries({ queryKey: appServerQueryKeys.threads });
}

export function useCreateThread() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (params: v2.ThreadStartParams = {}) => {
      const response = await requestAppServerMethod("thread/start", params);
      return response.response;
    },
    onSuccess: () => invalidateThreadList(queryClient),
  });
}

export function useArchiveThread() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (params: v2.ThreadArchiveParams) => {
      const response = await requestAppServerMethod("thread/archive", params);
      return response.response;
    },
    onSuccess: () => invalidateThreadList(queryClient),
  });
}

export function useUnarchiveThread() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (params: v2.ThreadUnarchiveParams) => {
      const response = await requestAppServerMethod("thread/unarchive", params);
      return response.response;
    },
    onSuccess: () => invalidateThreadList(queryClient),
  });
}

export function useRenameThread() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (params: v2.ThreadSetNameParams) => {
      const response = await requestAppServerMethod("thread/name/set", params);
      return response.response;
    },
    onSuccess: (_response, params) => {
      void queryClient.invalidateQueries({
        queryKey: appServerQueryKeys.thread(params.threadId),
      });
      invalidateThreadList(queryClient);
    },
  });
}

export function useResumeThread() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (params: v2.ThreadResumeParams) => {
      const response = await requestAppServerMethod("thread/resume", params);
      return response.response;
    },
    onSuccess: (_response, params) => {
      void queryClient.invalidateQueries({
        queryKey: appServerQueryKeys.thread(params.threadId),
      });
      invalidateThreadList(queryClient);
    },
  });
}

export function useForkThread() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (params: v2.ThreadForkParams) => {
      const response = await requestAppServerMethod("thread/fork", params);
      return response.response;
    },
    onSuccess: () => invalidateThreadList(queryClient),
  });
}
