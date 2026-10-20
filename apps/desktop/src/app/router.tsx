import { createHashRouter } from "react-router-dom";

import { AppLayout } from "../components/layout/AppLayout";
import { AccountPage } from "../pages/AccountPage";
import { ArchivePage } from "../pages/ArchivePage";
import { ApprovalCenter } from "../components/approvals/ApprovalCenter";
import { HomePage } from "../pages/HomePage";
import { LoginPage } from "../pages/LoginPage";
import { NewChatPage } from "../pages/NewChatPage";
import { ProjectHomePage } from "../pages/ProjectHomePage";
import { SettingsPage } from "../pages/SettingsPage";
import { ThreadPage } from "../pages/ThreadPage";
import { ThreadsPage } from "../pages/ThreadsPage";

export const router = createHashRouter([
  {
    path: "/",
    element: <AppLayout />,
    children: [
      { index: true, element: <HomePage /> },
      { path: "login", element: <LoginPage /> },
      { path: "account", element: <AccountPage /> },
      { path: "approvals", element: <ApprovalCenter /> },
      { path: "settings", element: <SettingsPage /> },
      { path: "archive", element: <ArchivePage /> },
      { path: "threads", element: <ThreadsPage /> },
      { path: "chat/new", element: <NewChatPage /> },
      { path: "chat/:threadId", element: <ThreadPage /> },
      { path: "projects/:projectId", element: <ProjectHomePage /> },
      { path: "projects/:projectId/chat/new", element: <NewChatPage /> },
    ],
  },
]);
