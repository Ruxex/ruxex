import { QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "react-router-dom";
import { useEffect } from "react";

import {
  startAppServerEventStore,
  stopAppServerEventStore,
} from "../app-server/event-store";
import { queryClient } from "./query-client";

type AppProvidersProps = {
  router: Parameters<typeof RouterProvider>[0]["router"];
};

export function AppProviders({ router }: AppProvidersProps) {
  useEffect(() => {
    void startAppServerEventStore();
    return stopAppServerEventStore;
  }, []);

  return (
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  );
}
