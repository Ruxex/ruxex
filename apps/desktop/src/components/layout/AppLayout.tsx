import { Outlet, useParams } from "react-router-dom";

import { ProjectSidebar } from "../sidebar/ProjectSidebar";
import { TopBar } from "./TopBar";

export function AppLayout() {
  const { projectId, threadId } = useParams<{
    projectId?: string;
    threadId?: string;
  }>();

  return (
    <main className="min-h-screen bg-base-200 text-base-content">
      <div className="flex min-h-screen">
        <ProjectSidebar
          selectedProjectId={projectId}
          selectedThreadId={threadId}
        />
        <div className="min-w-0 flex-1">
          <TopBar />
          <section className="mx-auto max-w-6xl p-6">
            <Outlet />
          </section>
        </div>
      </div>
    </main>
  );
}
