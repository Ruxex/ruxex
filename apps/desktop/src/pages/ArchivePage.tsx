import { Link } from "react-router-dom";

import { useArchivedThreadList, useUnarchiveThread } from "../app-server/hooks";
import { EmptyState, ErrorState, LoadingState } from "../components/common/AsyncState";
import { ThreadCard } from "../components/common/ThreadCard";

export function ArchivePage() {
  const archived = useArchivedThreadList();
  const unarchive = useUnarchiveThread();

  return (
    <div className="space-y-5">
      <div>
        <p className="text-sm font-medium uppercase tracking-wide text-primary">History</p>
        <h1 className="text-3xl font-semibold">Archive</h1>
      </div>
      {archived.isLoading && <LoadingState />}
      {archived.isError && <ErrorState>Unable to load archived chats.</ErrorState>}
      {!archived.isLoading && !archived.data?.data.length && (
        <EmptyState>No archived chats.</EmptyState>
      )}
      <div className="grid gap-3">
        {archived.data?.data.map((thread) => (
          <ThreadCard
            key={thread.id}
            thread={thread}
            actions={
              <>
                <button className="btn btn-primary btn-sm" type="button" disabled={unarchive.isPending} onClick={() => void unarchive.mutateAsync({ threadId: thread.id })}>
                  Restore
                </button>
                <Link className="btn btn-ghost btn-sm" to={`/chat/${thread.id}`}>Open</Link>
              </>
            }
          />
        ))}
      </div>
    </div>
  );
}
