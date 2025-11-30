import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { ResourcePoolList } from "../../components/ResourcePoolList";
import { http, HttpResponse } from "msw";
import { server } from "../../test/__mocks__/msw/server";

const mockPools = [
  {
    id: "pool-1",
    name: "Docker Pool",
    poolType: "Docker",
    providerName: "docker",
    minSize: 1,
    maxSize: 10,
    defaultResources: {
      cpu_m: 2000,
      memory_mb: 4096,
      gpu: null,
    },
    tags: { env: "test" },
  },
];

const mockStatus = {
  name: "Docker Pool",
  poolType: "Docker",
  totalCapacity: 10,
  availableCapacity: 8,
  activeWorkers: 2,
  pendingRequests: 0,
};

describe("ResourcePoolList", () => {
  beforeEach(() => {
    server.resetHandlers();
  });

  it("renders loading state initially", async () => {
    server.use(http.get("/api/v1/worker-pools", () => new Promise(() => {})));

    render(<ResourcePoolList />);

    expect(screen.getByRole("status")).toBeInTheDocument();
  });

  it("displays list of pools after loading", async () => {
    server.use(
      http.get("/api/v1/worker-pools", () => {
        return HttpResponse.json(mockPools);
      }),
      http.get("/api/v1/worker-pools/:id/status", ({ params }) => {
        return HttpResponse.json(mockStatus);
      }),
    );

    render(<ResourcePoolList />);

    await waitFor(() => {
      expect(screen.getByText("Docker Pool")).toBeInTheDocument();
    });

    expect(screen.getByText(/Provider:/)).toBeInTheDocument();
    expect(screen.getByText(/Size: 1 - 10/)).toBeInTheDocument();
  });

  it("displays empty state when no pools exist", async () => {
    server.use(
      http.get("/api/v1/worker-pools", () => {
        return HttpResponse.json([]);
      }),
    );

    render(<ResourcePoolList />);

    await waitFor(() => {
      expect(screen.getByText("No resource pools")).toBeInTheDocument();
    });

    expect(
      screen.getByText("Get started by creating a new resource pool."),
    ).toBeInTheDocument();
  });

  it("displays error state when loading fails", async () => {
    server.use(
      http.get("/api/v1/worker-pools", () => {
        return HttpResponse.json(
          { message: "Failed to load" },
          { status: 500 },
        );
      }),
    );

    render(<ResourcePoolList />);

    await waitFor(() => {
      expect(
        screen.getByText("Error loading resource pools"),
      ).toBeInTheDocument();
    });

    expect(screen.getByText("Failed to load")).toBeInTheDocument();
    expect(screen.getByText("Retry")).toBeInTheDocument();
  });

  it("displays capacity utilization correctly", async () => {
    server.use(
      http.get("/api/v1/worker-pools", () => {
        return HttpResponse.json(mockPools);
      }),
      http.get("/api/v1/worker-pools/:id/status", ({ params }) => {
        return HttpResponse.json({
          ...mockStatus,
          totalCapacity: 100,
          availableCapacity: 30,
          activeWorkers: 70,
        });
      }),
    );

    render(<ResourcePoolList />);

    await waitFor(() => {
      expect(screen.getByText("70%")).toBeInTheDocument();
    });
  });

  it("displays health status badges", async () => {
    server.use(
      http.get("/api/v1/worker-pools", () => {
        return HttpResponse.json(mockPools);
      }),
      http.get("/api/v1/worker-pools/:id/status", ({ params }) => {
        return HttpResponse.json(mockStatus);
      }),
    );

    render(<ResourcePoolList />);

    await waitFor(() => {
      expect(screen.getByText("healthy")).toBeInTheDocument();
    });
  });

  it("handles status fetch errors gracefully", async () => {
    server.use(
      http.get("/api/v1/worker-pools", () => {
        return HttpResponse.json(mockPools);
      }),
      http.get("/api/v1/worker-pools/:id/status", ({ params }) => {
        return HttpResponse.json({ message: "Not found" }, { status: 404 });
      }),
    );

    render(<ResourcePoolList />);

    await waitFor(() => {
      expect(screen.getByText("Docker Pool")).toBeInTheDocument();
    });

    // Should render without status
    expect(screen.getByText(/Provider:/)).toBeInTheDocument();
  });
});
