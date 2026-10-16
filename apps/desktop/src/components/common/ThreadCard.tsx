import type { ReactNode } from "react";
import { Link } from "react-router-dom";

type ThreadSummary = {
  id: string;
  name: string | null;
  preview: string;
  model?: string | null;
  updatedAt: number;
};

type ThreadCardProps = {
  thread: ThreadSummary;
  actions?: ReactNode;
  compact?: boolean;
};

export function ThreadCard({ thread, actions, compact = false }: ThreadCardProps) {
  if (compact) {
    return (
      <Link
        className="rounded-box block border border-base-300 p-4 transition hover:border-primary"
        to={`/chat/${thread.id}`}
      >
        <span className="font-medium">
          {thread.name || thread.preview || "Untitled thread"}
        </span>
        <span className="mt-1 block line-clamp-2 text-sm text-base-content/70">
          {thread.preview || "No preview available."}
        </span>
      </Link>
    );
  }

  return (
    <article className="card border border-base-300 bg-base-100 shadow-sm transition hover:border-primary">
      <div className="card-body gap-2 p-5">
        <Link className="card-title text-lg hover:text-primary" to={`/chat/${thread.id}`}>
          {thread.name || thread.preview || "Untitled thread"}
        </Link>
        <p className="line-clamp-2 text-sm text-base-content/70">
          {thread.preview || "No preview available."}
        </p>
        <div className="text-xs text-base-content/50">
          {thread.model || "Unknown model"} · {new Date(thread.updatedAt * 1000).toLocaleString()}
        </div>
        {actions && <div className="card-actions justify-end">{actions}</div>}
      </div>
    </article>
  );
}
