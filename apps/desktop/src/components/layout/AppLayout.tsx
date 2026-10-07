import { Link, Outlet } from "react-router-dom";

export function AppLayout() {
  return (
    <main className="min-h-screen bg-base-200 text-base-content">
      <div className="navbar border-b border-base-300 bg-base-100 px-6">
        <div className="flex-1">
          <Link className="text-xl font-semibold" to="/">
            Genex Desktop
          </Link>
        </div>
        <div className="badge badge-outline gap-2">
          <span className="status status-success" aria-hidden="true" />
          Local
        </div>
      </div>
      <section className="mx-auto max-w-5xl p-6">
        <div className="card border border-base-300 bg-base-100 shadow-sm">
          <div className="card-body gap-4">
            <div>
              <p className="text-sm font-medium uppercase tracking-wide text-primary">
                App server
              </p>
              <h1 className="card-title text-3xl">Ready for local agents</h1>
              <p className="mt-2 text-base-content/70">
                The Tauri bridge will connect this window to the shared Genex
                app-server daemon.
              </p>
            </div>
            <div className="card-actions justify-end">
              <button className="btn btn-primary" type="button">
                Check connection
              </button>
            </div>
          </div>
        </div>
        <Outlet />
      </section>
    </main>
  );
}
