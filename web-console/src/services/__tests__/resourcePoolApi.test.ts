import { describe, it, expect, beforeEach } from "vitest";
import {
  getResourcePools,
  getResourcePool,
  createResourcePool,
  updateResourcePool,
  deleteResourcePool,
  getResourcePoolStatus,
  getCapacityUtilization,
  isAtCriticalCapacity,
  getPoolHealthStatus,
  type ResourcePool,
  type CreateResourcePoolRequest,
  type UpdateResourcePoolRequest,
  type ResourcePoolStatus,
} from "../resourcePoolApi";
import { http, HttpResponse } from "msw";
import { server } from "../../test/__mocks__/msw/server";

describe("resourcePoolApi", () => {
  beforeEach(() => {
    server.resetHandlers();
  });

  describe("getResourcePools", () => {
    it("should fetch all resource pools successfully", async () => {
      const mockPools: ResourcePool[] = [
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
        {
          id: "pool-2",
          name: "Kubernetes Pool",
          poolType: "Kubernetes",
          providerName: "kubernetes",
          minSize: 2,
          maxSize: 20,
          defaultResources: {
            cpu_m: 4000,
            memory_mb: 8192,
            gpu: null,
          },
          tags: { env: "prod" },
        },
      ];

      server.use(
        http.get("/api/v1/worker-pools", () => {
          return HttpResponse.json(mockPools);
        }),
      );

      const result = await getResourcePools();
      expect(result).toEqual(mockPools);
    });

    it("should throw error when fetch fails", async () => {
      server.use(
        http.get("/api/v1/worker-pools", () => {
          return HttpResponse.json(
            { message: "Internal Server Error" },
            { status: 500 },
          );
        }),
      );

      await expect(getResourcePools()).rejects.toThrow(
        "Failed to fetch resource pools",
      );
    });
  });

  describe("getResourcePool", () => {
    it("should fetch a single resource pool by id", async () => {
      const mockPool: ResourcePool = {
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
      };

      server.use(
        http.get("/api/v1/worker-pools/:id", ({ params }) => {
          const { id } = params;
          if (id === "non-existent") {
            return HttpResponse.json(
              { message: "Pool not found" },
              { status: 404 },
            );
          }
          return HttpResponse.json(mockPool);
        }),
      );

      const result = await getResourcePool("pool-1");
      expect(result).toEqual(mockPool);
    });

    it("should throw error when pool not found", async () => {
      server.use(
        http.get("/api/v1/worker-pools/:id", ({ params }) => {
          const { id } = params;
          if (id === "non-existent") {
            return HttpResponse.json(
              { message: "Pool not found" },
              { status: 404 },
            );
          }
          return HttpResponse.json({});
        }),
      );

      await expect(getResourcePool("non-existent")).rejects.toThrow(
        "Failed to fetch resource pool",
      );
    });
  });

  describe("createResourcePool", () => {
    it("should create a new resource pool successfully", async () => {
      const request: CreateResourcePoolRequest = {
        poolType: "Docker",
        name: "New Docker Pool",
        providerName: "docker",
        minSize: 1,
        maxSize: 10,
        defaultResources: {
          cpu_m: 2000,
          memory_mb: 4096,
          gpu: null,
        },
        tags: { env: "dev" },
      };

      const mockCreatedPool: ResourcePool = {
        id: "new-pool-id",
        name: "New Docker Pool",
        poolType: "Docker",
        providerName: "docker",
        minSize: 1,
        maxSize: 10,
        defaultResources: {
          cpu_m: 2000,
          memory_mb: 4096,
          gpu: null,
        },
        tags: { env: "dev" },
      };

      server.use(
        http.post("/api/v1/worker-pools", async ({ request }) => {
          const body = (await request.json()) as any;
          if (body.name === "Existing Pool") {
            return HttpResponse.json(
              { message: "Pool already exists" },
              { status: 409 },
            );
          }
          return HttpResponse.json({
            id: "new-pool-id",
            ...body,
          });
        }),
      );

      const result = await createResourcePool(request);
      expect(result).toEqual(mockCreatedPool);
    });

    it("should throw error when pool name already exists", async () => {
      const request: CreateResourcePoolRequest = {
        poolType: "Docker",
        name: "Existing Pool",
        providerName: "docker",
        minSize: 1,
        maxSize: 10,
        defaultResources: {
          cpu_m: 2000,
          memory_mb: 4096,
          gpu: null,
        },
      };

      server.use(
        http.post("/api/v1/worker-pools", async ({ request }) => {
          const body = (await request.json()) as any;
          if (body.name === "Existing Pool") {
            return HttpResponse.json(
              { message: "Pool already exists" },
              { status: 409 },
            );
          }
          return HttpResponse.json({});
        }),
      );

      await expect(createResourcePool(request)).rejects.toThrow(
        "Failed to create resource pool",
      );
    });

    it("should validate min_size <= max_size", async () => {
      const request: CreateResourcePoolRequest = {
        poolType: "Docker",
        name: "Invalid Pool",
        providerName: "docker",
        minSize: 10,
        maxSize: 5,
        defaultResources: {
          cpu_m: 2000,
          memory_mb: 4096,
          gpu: null,
        },
      };

      await expect(createResourcePool(request)).rejects.toThrow(
        "min_size cannot be greater than max_size",
      );
    });
  });

  describe("updateResourcePool", () => {
    it("should update a resource pool successfully", async () => {
      const request: UpdateResourcePoolRequest = {
        name: "Updated Pool Name",
        minSize: 2,
        maxSize: 20,
        tags: { env: "prod", team: "data" },
      };

      const mockUpdatedPool: ResourcePool = {
        id: "pool-1",
        name: "Updated Pool Name",
        poolType: "Docker",
        providerName: "docker",
        minSize: 2,
        maxSize: 20,
        defaultResources: {
          cpu_m: 2000,
          memory_mb: 4096,
          gpu: null,
        },
        tags: { env: "prod", team: "data" },
      };

      server.use(
        http.patch("/api/v1/worker-pools/:id", async ({ params, request }) => {
          const { id } = params;
          if (id === "non-existent") {
            return HttpResponse.json(
              { message: "Pool not found" },
              { status: 404 },
            );
          }
          const body = (await request.json()) as any;
          return HttpResponse.json({
            id,
            name: body.name || "Updated Pool",
            poolType: "Docker",
            providerName: "docker",
            minSize: body.minSize || 1,
            maxSize: body.maxSize || 10,
            defaultResources: {
              cpu_m: 2000,
              memory_mb: 4096,
              gpu: null,
            },
            tags: body.tags || {},
          });
        }),
      );

      const result = await updateResourcePool("pool-1", request);
      expect(result).toEqual(mockUpdatedPool);
    });

    it("should handle partial updates", async () => {
      const request: UpdateResourcePoolRequest = {
        name: "Updated Name Only",
      };

      server.use(
        http.patch("/api/v1/worker-pools/:id", async ({ params, request }) => {
          const { id } = params;
          if (id === "non-existent") {
            return HttpResponse.json(
              { message: "Pool not found" },
              { status: 404 },
            );
          }
          const body = (await request.json()) as any;
          return HttpResponse.json({
            id,
            name: body.name || "Updated Pool",
            poolType: "Docker",
            providerName: "docker",
            minSize: 1,
            maxSize: 10,
            defaultResources: {
              cpu_m: 2000,
              memory_mb: 4096,
              gpu: null,
            },
            tags: body.tags || {},
          });
        }),
      );

      const result = await updateResourcePool("pool-1", request);
      expect(result.name).toBe("Updated Name Only");
    });
  });

  describe("deleteResourcePool", () => {
    it("should delete a resource pool successfully", async () => {
      server.use(
        http.delete("/api/v1/worker-pools/:id", ({ params }) => {
          const { id } = params;
          if (id === "non-existent") {
            return HttpResponse.json(
              { message: "Pool not found" },
              { status: 404 },
            );
          }
          return new HttpResponse(null, { status: 204 });
        }),
      );

      await expect(deleteResourcePool("pool-1")).resolves.toBeUndefined();
    });

    it("should throw error when pool not found", async () => {
      server.use(
        http.delete("/api/v1/worker-pools/:id", ({ params }) => {
          const { id } = params;
          if (id === "non-existent") {
            return HttpResponse.json(
              { message: "Pool not found" },
              { status: 404 },
            );
          }
          return new HttpResponse(null, { status: 204 });
        }),
      );

      await expect(deleteResourcePool("non-existent")).rejects.toThrow(
        "Failed to delete resource pool",
      );
    });
  });

  describe("getResourcePoolStatus", () => {
    it("should fetch resource pool status successfully", async () => {
      const mockStatus: ResourcePoolStatus = {
        name: "Docker Pool",
        poolType: "Docker",
        totalCapacity: 10,
        availableCapacity: 8,
        activeWorkers: 2,
        pendingRequests: 0,
      };

      server.use(
        http.get("/api/v1/worker-pools/:id/status", ({ params }) => {
          const { id } = params;
          if (id === "non-existent") {
            return HttpResponse.json(
              { message: "Pool status not found" },
              { status: 404 },
            );
          }
          return HttpResponse.json(mockStatus);
        }),
      );

      const result = await getResourcePoolStatus("pool-1");
      expect(result).toEqual(mockStatus);
    });

    it("should throw error when status not found", async () => {
      server.use(
        http.get("/api/v1/worker-pools/:id/status", ({ params }) => {
          const { id } = params;
          if (id === "non-existent") {
            return HttpResponse.json(
              { message: "Pool status not found" },
              { status: 404 },
            );
          }
          return HttpResponse.json({});
        }),
      );

      await expect(getResourcePoolStatus("non-existent")).rejects.toThrow(
        "Failed to fetch pool status",
      );
    });
  });

  describe("Utility Functions", () => {
    describe("getCapacityUtilization", () => {
      it("should calculate capacity utilization correctly", () => {
        const status: ResourcePoolStatus = {
          name: "Test Pool",
          poolType: "Docker",
          totalCapacity: 100,
          availableCapacity: 20,
          activeWorkers: 80,
          pendingRequests: 0,
        };
        expect(getCapacityUtilization(status)).toBe(80);
      });

      it("should handle zero capacity", () => {
        const status: ResourcePoolStatus = {
          name: "Empty Pool",
          poolType: "Docker",
          totalCapacity: 0,
          availableCapacity: 0,
          activeWorkers: 0,
          pendingRequests: 0,
        };
        expect(getCapacityUtilization(status)).toBe(0);
      });
    });

    describe("isAtCriticalCapacity", () => {
      it("should identify critical capacity correctly", () => {
        const criticalStatus: ResourcePoolStatus = {
          name: "Critical Pool",
          poolType: "Docker",
          totalCapacity: 100,
          availableCapacity: 4,
          activeWorkers: 96,
          pendingRequests: 0,
        };
        expect(isAtCriticalCapacity(criticalStatus)).toBe(true);

        const normalStatus: ResourcePoolStatus = {
          name: "Normal Pool",
          poolType: "Docker",
          totalCapacity: 100,
          availableCapacity: 10,
          activeWorkers: 90,
          pendingRequests: 0,
        };
        expect(isAtCriticalCapacity(normalStatus)).toBe(false);
      });
    });

    describe("getPoolHealthStatus", () => {
      it("should return correct health status", () => {
        const healthy: ResourcePoolStatus = {
          name: "Healthy Pool",
          poolType: "Docker",
          totalCapacity: 100,
          availableCapacity: 30,
          activeWorkers: 70,
          pendingRequests: 2,
        };
        expect(getPoolHealthStatus(healthy)).toBe("healthy");

        const warning: ResourcePoolStatus = {
          name: "Warning Pool",
          poolType: "Docker",
          totalCapacity: 100,
          availableCapacity: 15,
          activeWorkers: 85,
          pendingRequests: 7,
        };
        expect(getPoolHealthStatus(warning)).toBe("warning");

        const critical: ResourcePoolStatus = {
          name: "Critical Pool",
          poolType: "Docker",
          totalCapacity: 100,
          availableCapacity: 3,
          activeWorkers: 97,
          pendingRequests: 12,
        };
        expect(getPoolHealthStatus(critical)).toBe("critical");
      });
    });
  });
});
