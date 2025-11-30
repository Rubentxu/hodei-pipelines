import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ResourcePoolDetails } from "../ResourcePoolDetails";
import { http, HttpResponse } from "msw";
import { server } from "../../test/__mocks__/msw/server";

const mockPool = {
  id: "pool-123",
  name: "Docker Pool",
  poolType: "Docker" as const,
  providerName: "docker",
  minSize: 1,
  maxSize: 10,
  defaultResources: {
    cpu_m: 2000,
    memory_mb: 4096,
    gpu: null,
  },
  tags: { env: "test" },
};

const mockStatus = {
  name: "Docker Pool",
  poolType: "Docker" as const,
  totalCapacity: 10,
  availableCapacity: 8,
  activeWorkers: 2,
  pendingRequests: 0,
};

describe("ResourcePoolDetails", () => {
  beforeEach(() => {
    server.resetHandlers();
  });

  it("renders pool details correctly", async () => {
    server.use(
      http.get("/api/v1/worker-pools/:id", ({ params }) => {
        return HttpResponse.json(mockPool);
      }),
      http.get("/api/v1/worker-pools/:id/status", ({ params }) => {
        return HttpResponse.json(mockStatus);
      }),
    );

    render(<ResourcePoolDetails />);

    await waitFor(() => {
      expect(screen.getByText("Docker Pool")).toBeInTheDocument();
    });

    expect(screen.getByText(/Provider:/)).toBeInTheDocument();
    expect(screen.getByText(/Pool Type/)).toBeInTheDocument();
  });

  it("displays capacity utilization metrics", async () => {
    server.use(
      http.get("/api/v1/worker-pools/:id", ({ params }) => {
        return HttpResponse.json(mockPool);
      }),
      http.get("/api/v1/worker-pools/:id/status", ({ params }) => {
        return HttpResponse.json(mockStatus);
      }),
    );

    render(<ResourcePoolDetails />);

    await waitFor(() => {
      expect(screen.getByText("20%")).toBeInTheDocument();
    });

    expect(screen.getByText(/Active Workers/)).toBeInTheDocument();
    expect(screen.getByText(/Available Capacity/)).toBeInTheDocument();
  });

  it("displays health status badge", async () => {
    server.use(
      http.get("/api/v1/worker-pools/:id", ({ params }) => {
        return HttpResponse.json(mockPool);
      }),
      http.get("/api/v1/worker-pools/:id/status", ({ params }) => {
        return HttpResponse.json(mockStatus);
      }),
    );

    render(<ResourcePoolDetails />);

    await waitFor(() => {
      expect(screen.getByText(/healthy/)).toBeInTheDocument();
    });
  });

  it("displays tags correctly", async () => {
    server.use(
      http.get("/api/v1/worker-pools/:id", ({ params }) => {
        return HttpResponse.json(mockPool);
      }),
      http.get("/api/v1/worker-pools/:id/status", ({ params }) => {
        return HttpResponse.json(mockStatus);
      }),
    );

    render(<ResourcePoolDetails />);

    await waitFor(() => {
      expect(screen.getByText(/env: test/)).toBeInTheDocument();
    });
  });

  it("displays loading state initially", async () => {
    server.use(
      http.get("/api/v1/worker-pools/:id", () => new Promise(() => {})),
    );

    render(<ResourcePoolDetails />);

    expect(screen.getByRole("status")).toBeInTheDocument();
  });

  it("displays error when pool not found", async () => {
    server.use(
      http.get("/api/v1/worker-pools/:id", ({ params }) => {
        return HttpResponse.json({ message: "Not found" }, { status: 404 });
      }),
    );

    render(<ResourcePoolDetails />);

    await waitFor(() => {
      expect(screen.getByText("Error loading pool")).toBeInTheDocument();
    });
  });

  it("displays pending requests when they exist", async () => {
    const statusWithPending = {
      ...mockStatus,
      pendingRequests: 5,
    };

    server.use(
      http.get("/api/v1/worker-pools/:id", ({ params }) => {
        return HttpResponse.json(mockPool);
      }),
      http.get("/api/v1/worker-pools/:id/status", ({ params }) => {
        return HttpResponse.json(statusWithPending);
      }),
    );

    render(<ResourcePoolDetails />);

    await waitFor(() => {
      expect(screen.getByText(/5 job/)).toBeInTheDocument();
    });
  });

  it("handles missing status gracefully", async () => {
    server.use(
      http.get("/api/v1/worker-pools/:id", ({ params }) => {
        return HttpResponse.json(mockPool);
      }),
      http.get("/api/v1/worker-pools/:id/status", ({ params }) => {
        return HttpResponse.json({ message: "Not found" }, { status: 404 });
      }),
    );

    render(<ResourcePoolDetails />);

    await waitFor(() => {
      expect(screen.getByText("Docker Pool")).toBeInTheDocument();
    });

    expect(screen.getByText(/Pool Type/)).toBeInTheDocument();
  });
});
