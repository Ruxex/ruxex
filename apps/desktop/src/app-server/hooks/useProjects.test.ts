import { describe, expect, it } from "vitest";

import { appServerQueryKeys } from "../query-keys";
import { projectThreadListParams } from "./useProjects";

describe("project query contracts", () => {
  it("keeps the selected project in the project query key", () => {
    expect(appServerQueryKeys.project("project-1")).toEqual([
      "app-server",
      "project",
      "project-1",
    ]);
  });

  it("passes the selected project to thread/list", () => {
    expect(projectThreadListParams("project-1")).toEqual({
      projectId: "project-1",
      archived: false,
      limit: 50,
      sortKey: "recency_at",
      sortDirection: "desc",
    });
  });
});
