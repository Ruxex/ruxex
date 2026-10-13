import { Link } from "react-router-dom";

import { useArchivedThreadList, useUnarchiveThread } from "../app-server/hooks";

export function ArchivePage() {
  const archived = useArchivedThreadList();
  const unarchive = useUnarchiveThread();

  return (
    <div className="space-y-5">
      <div>
        <p className="text-sm font-medium uppercase tracking-wide text-primary">History</p>
        <h1 className="text-3xl font-semibold">Archive</h1>
      </div>
      {archived.isLoading && <span className="loading loading-spinner loading-lg" />}
      {archived.isError && <p className="alert alert-error">Unable to load archived chats.</p>}
      {!archived.isLoading && !archived.data?.data.length && (
        <p className="text-base-content/60">No archived chats.</p>
      )}
      <div className="grid gap-3">
        {archived.data?.data.map((thread) => (
          <article className="card border border-base-300 bg-base-100 shadow-sm" key={thread.id}>
            <div className="card-body gap-3 p-5">
              <h2 className="card-title text-lg">{thread.name || thread.preview || "Untitled chat"}</h2>
              <p className="text-sm text-base-content/70">{thread.preview || "No preview available."}</p>
              <div className="card-actions justify-end">
                <button className="btn btn-primary btn-sm" type="button" disabled={unarchive.isPending} onClick={() => void unarchive.mutateAsync({ threadId: thread.id })}>
                  Restore
                </button>
                <Link className="btn btn-ghost btn-sm" to={`/chat/${thread.id}`}>Open</Link>
              </div>
            </div>
          </article>
        ))}
      </div>
    </div>
  );
}
