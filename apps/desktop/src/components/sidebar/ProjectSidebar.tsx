import { Link, NavLink } from "react-router-dom";

import { useProjectList } from "../../app-server/hooks";
import { ThreadList } from "./ThreadList";

type ProjectSidebarProps = {
  selectedProjectId?: string;
  selectedThreadId?: string;
};

export function ProjectSidebar({
  selectedProjectId,
  selectedThreadId,
}: ProjectSidebarProps) {
  const projects = useProjectList({
    limit: 100,
    sortKey: "position",
    sortDirection: "asc",
  });

  return (
    <aside className="w-72 shrink-0 border-r border-base-300 bg-base-100 p-4">
      <div className="mb-5 flex items-center justify-between gap-2">
        <Link className="text-lg font-semibold" to="/">
          Ruxex Desktop
        </Link>
        <Link className="btn btn-primary btn-xs" to="/chat/new">
          New chat
        </Link>
      </div>
      <div className="mb-6">
        <div className="mb-2 flex items-center justify-between">
          <p className="text-xs font-semibold uppercase tracking-wide text-base-content/60">
            Projects
          </p>
          <NavLink className="link link-primary text-xs" to="/threads">
            All
          </NavLink>
        </div>
        {projects.isLoading && <span className="loading loading-spinner loading-sm" />}
        {projects.isError && (
          <p className="text-xs text-error">Unable to load projects.</p>
        )}
        <ul className="menu w-full gap-1 p-0">
          {projects.data?.data.map((project) => (
            <li key={project.id}>
              <NavLink
                className={selectedProjectId === project.id ? "active" : undefined}
                to={`/projects/${project.id}`}
              >
                <span className="truncate">{project.name}</span>
              </NavLink>
            </li>
          ))}
        </ul>
        {!projects.isLoading && !projects.data?.data.length && (
          <p className="text-xs text-base-content/60">No projects yet.</p>
        )}
      </div>
      <div>
        <div className="mb-2 flex items-center justify-between">
          <p className="text-xs font-semibold uppercase tracking-wide text-base-content/60">
            {selectedProjectId ? "Project chats" : "Recent chats"}
          </p>
          <Link className="link link-primary text-xs" to="/threads">
            Browse
          </Link>
        </div>
        <ThreadList
          projectId={selectedProjectId}
          selectedThreadId={selectedThreadId}
        />
      </div>
    </aside>
  );
}
