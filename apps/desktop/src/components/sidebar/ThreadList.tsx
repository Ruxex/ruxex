import { Link } from "react-router-dom";

import { useThreadList } from "../../app-server/hooks";
import { projectThreadListParams } from "../../app-server/hooks/useProjects";

type ThreadListProps = {
  projectId?: string;
  selectedThreadId?: string;
};

export function ThreadList({ projectId, selectedThreadId }: ThreadListProps) {
  const threads = useThreadList(
    projectId ? projectThreadListParams(projectId) : { limit: 50 },
  );

  if (threads.isLoading) {
    return <span className="loading loading-spinner loading-sm" />;
  }
  if (threads.isError) {
    return <p className="text-xs text-error">Unable to load chats.</p>;
  }
  if (!threads.data?.data.length) {
    return <p className="text-xs text-base-content/60">No chats yet.</p>;
  }

  return (
    <ul className="menu w-full gap-1 p-0">
      {threads.data.data.map((thread) => (
        <li key={thread.id}>
          <Link
            className={selectedThreadId === thread.id ? "active" : undefined}
            to={`/chat/${thread.id}`}
          >
            <span className="min-w-0">
              <span className="block truncate">
                {thread.name || thread.preview || "Untitled chat"}
              </span>
              <span className="block truncate text-xs opacity-60">
                {thread.model || "Default model"}
              </span>
            </span>
          </Link>
        </li>
      ))}
    </ul>
  );
}
