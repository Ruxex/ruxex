import type { ConversationViewItem } from "./conversation-model";

type ConversationItemProps = {
  item: ConversationViewItem;
};

export function ConversationItem({ item }: ConversationItemProps) {
  const isUser = item.type === "userMessage";
  return (
    <article className={`chat ${isUser ? "chat-end" : "chat-start"}`}>
      <div className="chat-header mb-1 text-xs text-base-content/60">
        {isUser ? "You" : "Ruxex"}
      </div>
      <div className={`chat-bubble ${isUser ? "chat-bubble-primary" : ""}`}>
        <p className="whitespace-pre-wrap">{item.text || "…"}</p>
      </div>
    </article>
  );
}
