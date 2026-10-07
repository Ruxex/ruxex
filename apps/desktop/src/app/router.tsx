import { createHashRouter } from "react-router-dom";

import { AppLayout } from "../components/layout/AppLayout";
import { AccountPage } from "../pages/AccountPage";
import { HomePage } from "../pages/HomePage";
import { LoginPage } from "../pages/LoginPage";
import { NewChatPage } from "../pages/NewChatPage";
import { ThreadsPage } from "../pages/ThreadsPage";

export const router = createHashRouter([
  {
    path: "/",
    element: <AppLayout />,
    children: [
      { index: true, element: <HomePage /> },
      { path: "login", element: <LoginPage /> },
      { path: "account", element: <AccountPage /> },
      { path: "threads", element: <ThreadsPage /> },
      { path: "chat/new", element: <NewChatPage /> },
    ],
  },
]);
