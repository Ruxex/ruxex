import { useQuery } from "@tanstack/react-query";

import type { v2 } from "../../protocol";
import { requestAppServerMethod } from "../api";
import { useAppServerStatus } from "../queries";
import { appServerQueryKeys } from "../query-keys";

export type ThreadListOptions = {
  enabled?: boolean;
};

export function useThreadList(
  params: v2.ThreadListParams = {},
  options: ThreadListOptions = {},
) {
  const status = useAppServerStatus();

  return useQuery({
    queryKey: appServerQueryKeys.threadList(params),
    queryFn: async () => {
      const response = await requestAppServerMethod("thread/list", params);
      return response.response;
    },
    enabled: Boolean(status.data) && options.enabled !== false,
  });
}

export function useThreadSearch(searchTerm: string) {
  const status = useAppServerStatus();
  const params: v2.ThreadSearchParams = {
    searchTerm,
    limit: 50,
    sortKey: "recency_at",
    sortDirection: "desc",
  };

  return useQuery({
    queryKey: appServerQueryKeys.threadSearch(params),
    queryFn: async () => {
      const response = await requestAppServerMethod("thread/search", params);
      return response.response;
    },
    enabled: Boolean(status.data && searchTerm.trim()),
  });
}

export function archivedThreadListParams(): v2.ThreadListParams {
  return {
    archived: true,
    limit: 50,
    sortKey: "updated_at",
    sortDirection: "desc",
  };
}

export function useArchivedThreadList() {
  return useThreadList(archivedThreadListParams());
}

export function useThread(threadId: string, includeTurns = false) {
  const status = useAppServerStatus();

  return useQuery({
    queryKey: [...appServerQueryKeys.thread(threadId), { includeTurns }],
    queryFn: async () => {
      const response = await requestAppServerMethod("thread/read", {
        threadId,
        includeTurns,
      });
      return response.response;
    },
    enabled: Boolean(status.data && threadId),
  });
}

export function useThreadTurns(
  threadId: string,
  params: Omit<v2.ThreadTurnsListParams, "threadId"> = {},
) {
  const status = useAppServerStatus();

  return useQuery({
    queryKey: appServerQueryKeys.threadTurns(threadId, params),
    queryFn: async () => {
      const response = await requestAppServerMethod("thread/turns/list", {
        ...params,
        threadId,
      });
      return response.response;
    },
    enabled: Boolean(status.data && threadId),
  });
}

export function useThreadItems(
  threadId: string,
  params: Omit<v2.ThreadItemsListParams, "threadId"> = {},
) {
  const status = useAppServerStatus();

  return useQuery({
    queryKey: appServerQueryKeys.threadItems(threadId, params),
    queryFn: async () => {
      const response = await requestAppServerMethod("thread/items/list", {
        ...params,
        threadId,
      });
      return response.response;
    },
    enabled: Boolean(status.data && threadId),
  });
}

export type ThreadConversation = {
  thread: v2.ThreadReadResponse["thread"];
  turns: v2.ThreadTurnsListResponse;
  items: v2.ThreadItemsListResponse;
};

export type ThreadConversationOptions = {
  turns?: Omit<v2.ThreadTurnsListParams, "threadId">;
  items?: Omit<v2.ThreadItemsListParams, "threadId">;
};

export function useThreadConversation(
  threadId: string,
  options: ThreadConversationOptions = {},
) {
  const status = useAppServerStatus();
  const turns = options.turns ?? {};
  const items = options.items ?? {};

  return useQuery<ThreadConversation>({
    queryKey: appServerQueryKeys.threadConversation(threadId, { turns, items }),
    queryFn: async () => {
      const [thread, turnsResponse, itemsResponse] = await Promise.all([
        requestAppServerMethod("thread/read", { threadId }),
        requestAppServerMethod("thread/turns/list", { ...turns, threadId }),
        requestAppServerMethod("thread/items/list", { ...items, threadId }),
      ]);
      return {
        thread: thread.response.thread,
        turns: turnsResponse.response,
        items: itemsResponse.response,
      };
    },
    enabled: Boolean(status.data && threadId),
  });
}
