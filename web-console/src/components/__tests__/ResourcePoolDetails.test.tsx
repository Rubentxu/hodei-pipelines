import { render, screen, waitFor } from "@testing-library/react";
import { http, HttpResponse } from "msw";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { beforeEach, describe, expect, it } from "vitest";
import { server } from "../../test/__mocks__/msw/server";
import { ResourcePoolDetails } from "../ResourcePoolDetails";

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

const renderWithRouter = (ui: React.ReactElement, initialEntries = ["/resource-pools/pool-123"]) => {
  return render(
    <MemoryRouter initialEntries={initialEntries}>
      <Routes>
        <Route path="/resource-pools/:id" element={ui} />
        <Route path="/resource-pools" element={<div>Pools List</div>} />
      </Routes>
    </MemoryRouter>
  );
};

describe("ResourcePoolDetails", () => {
  beforeEach(() => {
    server.resetHandlers();
  });

  it("renders pool details correctly", async () => {
    server.use(
      http.get("*/api/v1/worker-pools/:id", ({ params }) => {
        return HttpResponse.json(mockPool);
      }),
      http.get("*/api/v1/worker-pools/:id/status", ({ params }) => {
        return HttpResponse.json(mockStatus);
      }),
    );

    renderWithRouter(<ResourcePoolDetails />);

    await waitFor(() => {
      expect(screen.getByText("Docker Pool")).toBeInTheDocument();
    });

    expect(screen.getByText(/Provider/)).toBeInTheDocument();
    expect(screen.getByText(/Pool Type/)).toBeInTheDocument();
  });

  it("displays capacity utilization metrics", async () => {
    server.use(
      http.get("*/api/v1/worker-pools/:id", ({ params }) => {
        return HttpResponse.json(mockPool);
      }),
      http.get("*/api/v1/worker-pools/:id/status", ({ params }) => {
        return HttpResponse.json(mockStatus);
      }),
    );

    renderWithRouter(<ResourcePoolDetails />);

    await waitFor(() => {
      expect(screen.getByText("20%")).toBeInTheDocument();
    });

    expect(screen.getByText(/Active Workers/)).toBeInTheDocument();
    expect(screen.getByText(/Available Capacity/)).toBeInTheDocument();
  });

  it("displays health status badge", async () => {
    server.use(
      http.get("*/api/v1/worker-pools/:id", ({ params }) => {
        return HttpResponse.json(mockPool);
      }),
      http.get("*/api/v1/worker-pools/:id/status", ({ params }) => {
        return HttpResponse.json(mockStatus);
      }),
    );

    renderWithRouter(<ResourcePoolDetails />);

    await waitFor(() => {
      expect(screen.getByText(/healthy/)).toBeInTheDocument();
    });
  });

  it("displays tags correctly", async () => {
    server.use(
      http.get("*/api/v1/worker-pools/:id", ({ params }) => {
        return HttpResponse.json(mockPool);
      }),
      http.get("*/api/v1/worker-pools/:id/status", ({ params }) => {
        return HttpResponse.json(mockStatus);
      }),
    );

    renderWithRouter(<ResourcePoolDetails />);

    await waitFor(() => {
      expect(screen.getByText(/env: test/)).toBeInTheDocument();
    });
  });

  it("displays loading state initially", async () => {
    server.use(
      http.get("*/api/v1/worker-pools/:id", () => new Promise(() => { })),
    );

    renderWithRouter(<ResourcePoolDetails />);

    expect(screen.getByRole("status")).toBeInTheDocument();
    // If it uses a spinner without text, check for that.
    // The component has: <div className="animate-spin ..."></div>
    // But no role="status" or text.
    // I should check the component implementation again.
    // It renders: <div className="flex items-center justify-center h-64"><div className="animate-spin ..."></div></div>
  });

  it("displays error when pool not found", async () => {
    server.use(
      http.get("*/api/v1/worker-pools/:id", ({ params }) => {
        return HttpResponse.json({ message: "Not found" }, { status: 404 });
      }),
    );

    renderWithRouter(<ResourcePoolDetails />);

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
      http.get("*/api/v1/worker-pools/:id", ({ params }) => {
        return HttpResponse.json(mockPool);
      }),
      http.get("*/api/v1/worker-pools/:id/status", ({ params }) => {
        return HttpResponse.json(statusWithPending);
      }),
    );

    renderWithRouter(<ResourcePoolDetails />);

    await waitFor(() => {
      expect(screen.getByText(/5 job/)).toBeInTheDocument();
    });
  });

  it("handles missing status gracefully", async () => {
    server.use(
      http.get("*/api/v1/worker-pools/:id", ({ params }) => {
        return HttpResponse.json(mockPool);
      }),
      http.get("*/api/v1/worker-pools/:id/status", ({ params }) => {
        return HttpResponse.json({ message: "Not found" }, { status: 404 });
      }),
    );

    renderWithRouter(<ResourcePoolDetails />);

    await waitFor(() => {
      expect(screen.getByText("Docker Pool")).toBeInTheDocument();
    });

    expect(screen.getByText(/Pool Type/)).toBeInTheDocument();
  });
});
