import { Link } from "react-router-dom";

import { useState } from "react";

import { useThreadList, useThreadSearch } from "../app-server/hooks";

export function ThreadsPage() {
  const [search, setSearch] = useState("");
  const threads = useThreadList({ limit: 50 });
  const searchResults = useThreadSearch(search);
  const rows = search.trim()
    ? searchResults.data?.data.map((result) => result.thread) ?? []
    : threads.data?.data ?? [];

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
      <input
        className="input input-bordered w-full"
        value={search}
        onChange={(event) => setSearch(event.target.value)}
        placeholder="Search chats"
        aria-label="Search chats"
      />
      {(threads.isLoading || searchResults.isLoading) && (
        <span className="loading loading-spinner loading-lg" />
      )}
      {threads.isError && (
        <p className="alert alert-error">{String(threads.error)}</p>
      )}
      {!threads.isLoading && !searchResults.isLoading && rows.length === 0 && (
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
        {rows.map((thread) => (
          <article
            className="card border border-base-300 bg-base-100 shadow-sm transition hover:border-primary"
            key={thread.id}
          >
            <div className="card-body gap-2 p-5">
              <Link className="card-title text-lg hover:text-primary" to={`/chat/${thread.id}`}>
                {thread.name ?? (thread.preview || "Untitled thread")}
              </Link>
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
