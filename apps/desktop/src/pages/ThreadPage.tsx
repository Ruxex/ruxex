import { useParams } from "react-router-dom";

import { ConversationWorkspace } from "../components/conversation/ConversationWorkspace";

export function ThreadPage() {
  const { threadId = "" } = useParams<{ threadId: string }>();
  return <ConversationWorkspace threadId={threadId} />;
}
