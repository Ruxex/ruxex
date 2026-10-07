import { useState, type FormEvent } from "react";
import { useNavigate } from "react-router-dom";

import {
  useCreateThread,
  useModelList,
  useSendMessage,
} from "../app-server/hooks";

export function NewChatPage() {
  const navigate = useNavigate();
  const models = useModelList();
  const createThread = useCreateThread();
  const sendMessage = useSendMessage();
  const [model, setModel] = useState("");
  const [message, setMessage] = useState("");

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    try {
      const thread = await createThread.mutateAsync(model ? { model } : {});
      await sendMessage.mutateAsync({
        threadId: thread.thread.id,
        input: [{ type: "text", text: message, text_elements: [] }],
      });
      navigate("/threads");
    } catch {
      // The mutation state renders the error below the form.
    }
  }

  const isPending = createThread.isPending || sendMessage.isPending;
  const error = createThread.error ?? sendMessage.error;

  return (
    <div className="mx-auto max-w-3xl">
      <div className="card border border-base-300 bg-base-100 shadow-sm">
        <form
          className="card-body gap-5"
          onSubmit={(event) => void handleSubmit(event)}
        >
          <div>
            <p className="text-sm font-medium uppercase tracking-wide text-primary">
              Chat
            </p>
            <h1 className="card-title text-3xl">New chat</h1>
            <p className="mt-2 text-base-content/70">
              Create a local thread and send its first message.
            </p>
          </div>
          <label className="form-control">
            <span className="label-text">Model</span>
            <select
              className="select select-bordered mt-2"
              value={model}
              onChange={(event) => setModel(event.target.value)}
            >
              <option value="">Use the default model</option>
              {models.data?.data.map((item) => (
                <option key={item.id} value={item.model}>
                  {item.displayName}
                </option>
              ))}
            </select>
          </label>
          <label className="form-control">
            <span className="label-text">Message</span>
            <textarea
              className="textarea textarea-bordered mt-2 min-h-40"
              value={message}
              onChange={(event) => setMessage(event.target.value)}
              placeholder="What would you like to work on?"
              required
            />
          </label>
          {error && <p className="text-error">{String(error)}</p>}
          <div className="card-actions justify-end">
            <button
              className="btn btn-primary"
              type="submit"
              disabled={isPending}
            >
              {isPending ? "Starting…" : "Start chat"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
