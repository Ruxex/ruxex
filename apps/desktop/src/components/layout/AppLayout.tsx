import { Link, NavLink, Outlet, useNavigate } from "react-router-dom";

import {
  useAccount,
  useAppServerEvents,
  useAppServerStatus,
  useLogout,
} from "../../app-server/hooks";

export function AppLayout() {
  const navigate = useNavigate();
  const account = useAccount();
  const logout = useLogout();
  const appServer = useAppServerStatus();
  const events = useAppServerEvents();
  const connectionLabel = appServer.isError
    ? "Error"
    : !appServer.isSuccess
      ? "Local"
      : events.connected
        ? "Connected"
        : "Disconnected";

  return (
    <main className="min-h-screen bg-base-200 text-base-content">
      <div className="navbar flex-wrap gap-2 border-b border-base-300 bg-base-100 px-6">
        <div className="flex-1">
          <Link className="text-xl font-semibold" to="/">
            Genex Desktop
          </Link>
        </div>
        <nav className="flex items-center gap-1" aria-label="Main navigation">
          <NavLink className="btn btn-ghost btn-sm" to="/threads">
            Threads
          </NavLink>
          <NavLink className="btn btn-ghost btn-sm" to="/chat/new">
            New chat
          </NavLink>
          <NavLink className="btn btn-ghost btn-sm" to="/account">
            Account
          </NavLink>
        </nav>
        <div className="badge badge-outline gap-2">
          <span
            className={`status ${connectionLabel === "Connected" ? "status-success" : "status-warning"}`}
            aria-hidden="true"
          />
          {connectionLabel}
        </div>
        {account.data?.account && (
          <button
            className="btn btn-ghost btn-sm"
            type="button"
            disabled={logout.isPending}
            onClick={() => {
              void logout
                .mutateAsync()
                .then(() => navigate("/login"))
                .catch(() => undefined);
            }}
          >
            {logout.isPending ? "Logging out…" : "Log out"}
          </button>
        )}
      </div>
      <section className="mx-auto max-w-5xl p-6">
        <Outlet />
      </section>
    </main>
  );
}
