import { Link } from "react-router-dom";

import { useThreadList } from "../app-server/hooks";

export function ThreadsPage() {
  const threads = useThreadList({ limit: 50 });

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between gap-4">
        <div>
          <p className="text-sm font-medium uppercase tracking-wide text-primary">
            History
          </p>
          <h1 className="text-3xl font-semibold">Threads</h1>
        </div>
        <Link className="btn btn-primary" to="/chat/new">
          New chat
        </Link>
      </div>
      {threads.isLoading && (
        <span className="loading loading-spinner loading-lg" />
      )}
      {threads.isError && (
        <p className="alert alert-error">{String(threads.error)}</p>
      )}
      {threads.data?.data.length === 0 && (
        <div className="card border border-dashed border-base-300 bg-base-100">
          <div className="card-body items-center text-center">
            <p>No threads yet.</p>
            <Link className="btn btn-sm btn-primary" to="/chat/new">
              Start a chat
            </Link>
          </div>
        </div>
      )}
      <div className="grid gap-3">
        {threads.data?.data.map((thread) => (
          <article
            className="card border border-base-300 bg-base-100 shadow-sm"
            key={thread.id}
          >
            <div className="card-body gap-2 p-5">
              <h2 className="card-title text-lg">
                {thread.name ?? (thread.preview || "Untitled thread")}
              </h2>
              <p className="line-clamp-2 text-sm text-base-content/70">
                {thread.preview || "No preview available."}
              </p>
              <div className="text-xs text-base-content/50">
                {thread.model ?? "Unknown model"} ·{" "}
                {new Date(thread.updatedAt * 1000).toLocaleString()}
              </div>
            </div>
          </article>
        ))}
      </div>
    </div>
  );
}
