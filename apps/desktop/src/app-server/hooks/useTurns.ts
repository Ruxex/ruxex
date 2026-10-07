import { useMutation } from "@tanstack/react-query";

import type { v2 } from "../../protocol";
import { requestAppServerMethod } from "../api";

export function useSendMessage() {
  return useMutation({
    mutationFn: async (params: v2.TurnStartParams) => {
      const response = await requestAppServerMethod("turn/start", params);
      return response.response;
    },
  });
}

export function useInterruptTurn() {
  return useMutation({
    mutationFn: async (params: v2.TurnInterruptParams) => {
      const response = await requestAppServerMethod("turn/interrupt", params);
      return response.response;
    },
  });
}
