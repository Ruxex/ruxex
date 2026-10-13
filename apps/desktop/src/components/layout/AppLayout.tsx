import { Link, NavLink, Outlet, useNavigate, useParams } from "react-router-dom";

import {
  useAccount,
  useAppServerEvents,
  useAppServerStatus,
  useLogout,
} from "../../app-server/hooks";
import { ProjectSidebar } from "../sidebar/ProjectSidebar";

export function AppLayout() {
  const navigate = useNavigate();
  const account = useAccount();
  const logout = useLogout();
  const appServer = useAppServerStatus();
  const events = useAppServerEvents();
  const { projectId } = useParams<{ projectId?: string }>();
  const connectionLabel = appServer.isError
    ? "Error"
    : !appServer.isSuccess
      ? "Local"
      : events.connected
        ? "Connected"
        : "Disconnected";

  return (
    <main className="min-h-screen bg-base-200 text-base-content">
      <div className="flex min-h-screen">
        <ProjectSidebar selectedProjectId={projectId} />
        <div className="min-w-0 flex-1">
          <div className="navbar flex-wrap gap-2 border-b border-base-300 bg-base-100 px-6">
            <div className="flex-1">
              <Link className="text-xl font-semibold" to="/">
                Workspace
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
              <NavLink className="btn btn-ghost btn-sm" to="/approvals">
                Approvals
              </NavLink>
              <NavLink className="btn btn-ghost btn-sm" to="/settings">
                Settings
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
          <section className="mx-auto max-w-6xl p-6">
            <Outlet />
          </section>
        </div>
      </div>
    </main>
  );
}
