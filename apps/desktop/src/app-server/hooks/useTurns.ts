import { useMutation, useQueryClient } from "@tanstack/react-query";

import type { v2 } from "../../protocol";
import { requestAppServerMethod } from "../api";
import { appServerQueryKeys } from "../query-keys";

function invalidateThreadQueries(queryClient: ReturnType<typeof useQueryClient>, threadId: string) {
  void queryClient.invalidateQueries({ queryKey: appServerQueryKeys.thread(threadId) });
  void queryClient.invalidateQueries({ queryKey: appServerQueryKeys.threads });
}

export function useSendMessage() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (params: v2.TurnStartParams) => {
      const response = await requestAppServerMethod("turn/start", params);
      return response.response;
    },
    onSuccess: (_response, params) => invalidateThreadQueries(queryClient, params.threadId),
  });
}

export function useInterruptTurn() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (params: v2.TurnInterruptParams) => {
      const response = await requestAppServerMethod("turn/interrupt", params);
      return response.response;
    },
    onSuccess: (_response, params) => invalidateThreadQueries(queryClient, params.threadId),
  });
}

export function useSteerTurn() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (params: v2.TurnSteerParams) => {
      const response = await requestAppServerMethod("turn/steer", params);
      return response.response;
    },
    onSuccess: (_response, params) => invalidateThreadQueries(queryClient, params.threadId),
  });
}

export function useQueueMessage() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (params: Omit<v2.ThreadQueueAddParams, "clientUserMessageId">) => {
      const response = await requestAppServerMethod("thread/queue/add", {
        ...params,
        clientUserMessageId: `desktop-${Date.now()}-${Math.random().toString(36).slice(2)}`,
      });
      return response.response;
    },
    onSuccess: (_response, params) => invalidateThreadQueries(queryClient, params.threadId),
  });
}
