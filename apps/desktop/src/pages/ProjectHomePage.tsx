import { Link, useParams } from "react-router-dom";

import { useProject, useProjectThreads } from "../app-server/hooks";
import { EmptyState, ErrorState, LoadingState } from "../components/common/AsyncState";
import { ThreadCard } from "../components/common/ThreadCard";

export function ProjectHomePage() {
  const { projectId = "" } = useParams<{ projectId: string }>();
  const project = useProject(projectId);
  const threads = useProjectThreads(projectId);

  if (project.isLoading) {
    return <LoadingState />;
  }
  if (project.isError || !project.data) {
    return <ErrorState>Unable to load this project.</ErrorState>;
  }

  const details = project.data.project;
  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <p className="text-sm font-medium uppercase tracking-wide text-primary">Project</p>
          <h1 className="text-3xl font-semibold">{details.name}</h1>
          <p className="mt-2 text-sm text-base-content/70">
            {details.roots.length} workspace root{details.roots.length === 1 ? "" : "s"}
          </p>
        </div>
        <Link className="btn btn-primary" to={`/projects/${projectId}/chat/new`}>
          New chat
        </Link>
      </div>
      <section className="card border border-base-300 bg-base-100 shadow-sm">
        <div className="card-body">
          <h2 className="card-title">Recent chats</h2>
          {threads.isLoading && <LoadingState size="sm" />}
          {threads.isError && (
            <ErrorState>Unable to load project chats.</ErrorState>
          )}
          {!threads.isLoading && !threads.data?.data.length && (
            <EmptyState>No chats in this project yet.</EmptyState>
          )}
          <div className="grid gap-3">
            {threads.data?.data.map((thread) => (
              <ThreadCard key={thread.id} thread={thread} compact />
            ))}
          </div>
        </div>
      </section>
    </div>
  );
}
