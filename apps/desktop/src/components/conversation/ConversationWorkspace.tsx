import { useAppServerEvents, useThreadConversation } from "../../app-server/hooks";
import { Composer } from "./Composer";
import { ConversationItem } from "./ConversationItem";
import { ActivityItem } from "./ActivityItem";
import { mergeConversationItems } from "./conversation-model";

export function ConversationWorkspace({ threadId }: { threadId: string }) {
  const conversation = useThreadConversation(threadId);
  const events = useAppServerEvents();
  const liveThread = events.threads[threadId];

  if (conversation.isLoading) {
    return <span className="loading loading-spinner loading-lg" />;
  }
  if (conversation.isError || !conversation.data) {
    return <p className="alert alert-error">Unable to load this conversation.</p>;
  }

  const items = mergeConversationItems(conversation.data.items.data, liveThread?.items ?? []);
  return (
    <div className="flex min-h-[calc(100vh-9rem)] flex-col gap-4">
      <header>
        <p className="text-sm font-medium uppercase tracking-wide text-primary">Conversation</p>
        <h1 className="text-2xl font-semibold">
          {conversation.data.thread.name || conversation.data.thread.preview || "Untitled chat"}
        </h1>
        {liveThread?.status === "resynchronizing" && (
          <p className="mt-2 text-sm text-warning">Reconnecting and resynchronizing this chat…</p>
        )}
      </header>
      <section className="min-h-0 flex-1 space-y-4 overflow-y-auto rounded-box border border-base-300 bg-base-200 p-4">
        {items.length === 0 && (
          <p className="py-12 text-center text-base-content/60">No messages yet.</p>
        )}
        {items.map((item) =>
          item.type === "userMessage" || item.type === "agentMessage" ? (
            <ConversationItem item={item} key={item.id} />
          ) : (
            <ActivityItem item={item} key={item.id} />
          ),
        )}
      </section>
      <Composer threadId={threadId} activeTurnId={liveThread?.activeTurnId ?? null} />
    </div>
  );
}
