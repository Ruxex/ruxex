import {
  useAppServerEvents,
  useAppServerStatus,
  useModelList,
} from "../app-server/hooks";

export function HomePage() {
  const appServer = useAppServerStatus();
  const models = useModelList();
  const events = useAppServerEvents();

  return (
    <>
      <div className="card border border-base-300 bg-base-100 shadow-sm">
        <div className="card-body gap-4">
          <div>
            <p className="text-sm font-medium uppercase tracking-wide text-primary">
              App server
            </p>
            <h1 className="card-title text-3xl">Ready for local agents</h1>
            <p className="mt-2 text-base-content/70">
              {appServer.isError
                ? String(appServer.error)
                : "The Tauri bridge connects this window to the shared Ruxex app-server daemon."}
            </p>
          </div>
          <div className="card-actions justify-end">
            <button
              className="btn btn-primary"
              type="button"
              onClick={() => void appServer.refetch()}
            >
              {appServer.isFetching ? "Connecting…" : "Check connection"}
            </button>
          </div>
        </div>
      </div>
      <div className="mt-4 grid gap-4 md:grid-cols-2">
        <div className="card border border-base-300 bg-base-100 shadow-sm">
          <div className="card-body">
            <h2 className="card-title">Models</h2>
            {models.isLoading && <span className="loading loading-spinner" />}
            {models.data?.data.length === 0 && <p>No models available.</p>}
            <ul className="space-y-2">
              {models.data?.data.slice(0, 8).map((model) => (
                <li className="flex justify-between" key={model.id}>
                  <span>{model.displayName}</span>
                  <span className="text-sm text-base-content/60">
                    {model.model}
                  </span>
                </li>
              ))}
            </ul>
          </div>
        </div>
        <div className="card border border-base-300 bg-base-100 shadow-sm">
          <div className="card-body">
            <h2 className="card-title">Events</h2>
            <p className="text-sm text-base-content/70">
              {events.connected
                ? `${events.history.length} events received`
                : "Waiting for the app-server"}
            </p>
          </div>
        </div>
      </div>
    </>
  );
}
