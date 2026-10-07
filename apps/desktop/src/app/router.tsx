import { createHashRouter } from "react-router-dom";

import { AppLayout } from "../components/layout/AppLayout";

export const router = createHashRouter([
  {
    path: "/",
    element: <AppLayout />,
  },
]);
