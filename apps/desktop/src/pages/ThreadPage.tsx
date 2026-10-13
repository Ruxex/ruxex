import { Link, useParams } from "react-router-dom";

import { useThread } from "../app-server/hooks";

export function ThreadPage() {
  const { threadId = "" } = useParams<{ threadId: string }>();
  const thread = useThread(threadId);

  if (thread.isLoading) {
    return <span className="loading loading-spinner loading-lg" />;
  }
  if (thread.isError || !thread.data) {
    return <p className="alert alert-error">Unable to load this chat.</p>;
  }

  return (
    <div className="space-y-4">
      <div>
        <p className="text-sm font-medium uppercase tracking-wide text-primary">Chat</p>
        <h1 className="text-3xl font-semibold">
          {thread.data.thread.name || thread.data.thread.preview || "Untitled chat"}
        </h1>
      </div>
      <div className="alert alert-info">
        <span>Conversation workspace is loading for this thread.</span>
      </div>
      <Link className="btn btn-primary" to="/chat/new">
        Start another chat
      </Link>
    </div>
  );
}
