import { Link, useParams } from "react-router-dom";

import { useProject, useProjectThreads } from "../app-server/hooks";

export function ProjectHomePage() {
  const { projectId = "" } = useParams<{ projectId: string }>();
  const project = useProject(projectId);
  const threads = useProjectThreads(projectId);

  if (project.isLoading) {
    return <span className="loading loading-spinner loading-lg" />;
  }
  if (project.isError || !project.data) {
    return <p className="alert alert-error">Unable to load this project.</p>;
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
          {threads.isLoading && <span className="loading loading-spinner" />}
          {threads.isError && (
            <p className="alert alert-error">Unable to load project chats.</p>
          )}
          {!threads.isLoading && !threads.data?.data.length && (
            <p className="text-base-content/60">No chats in this project yet.</p>
          )}
          <div className="grid gap-3">
            {threads.data?.data.map((thread) => (
              <Link
                className="rounded-box border border-base-300 p-4 transition hover:border-primary"
                key={thread.id}
                to={`/chat/${thread.id}`}
              >
                <h3 className="font-medium">
                  {thread.name || thread.preview || "Untitled chat"}
                </h3>
                <p className="mt-1 line-clamp-2 text-sm text-base-content/70">
                  {thread.preview || "No preview available."}
                </p>
              </Link>
            ))}
          </div>
        </div>
      </section>
    </div>
  );
}
