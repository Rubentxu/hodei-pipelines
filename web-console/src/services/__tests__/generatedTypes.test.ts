/**
 * @jest-environment jsdom
 */

import { describe, it, expect, beforeAll } from "vitest";
import type { components } from "../../types/api";

// Type aliases for easier use
type Pipeline = components["schemas"]["Pipeline"];
type CreatePipelineRequest = components["schemas"]["CreatePipelineRequest"];
type UpdatePipelineRequest = components["schemas"]["UpdatePipelineRequest"];
type WorkerPool = components["schemas"]["WorkerPool"];
type WorkerPoolStatus = components["schemas"]["WorkerPoolStatus"];
type Error = components["schemas"]["Error"];

describe("Generated OpenAPI Types", () => {
  describe("Pipeline Types", () => {
    it("should validate Pipeline type structure", () => {
      const pipeline: Pipeline = {
        id: "123e4567-e89b-12d3-a456-426614174000",
        name: "Test Pipeline",
        description: "A test pipeline",
        status: "active",
        schedule: "0 0 * * *",
        tags: ["test", "ci"],
        createdAt: "2025-11-30T10:00:00Z",
        updatedAt: "2025-11-30T10:00:00Z",
      };

      expect(pipeline.id).toBeDefined();
      expect(pipeline.name).toBeDefined();
      expect(pipeline.status).toMatch(/^(active|inactive|archived)$/);
      expect(Array.isArray(pipeline.tags)).toBe(true);
    });

    it("should validate CreatePipelineRequest type", () => {
      const request: CreatePipelineRequest = {
        name: "New Pipeline",
        description: "Pipeline description",
        schedule: "0 0 * * *",
        tags: ["build"],
      };

      expect(request.name).toBeDefined();
      expect(request.description).toBeDefined();
      expect(request.schedule).toBeDefined();
      expect(request.tags).toBeDefined();
    });

    it("should accept CreatePipelineRequest with minimal fields", () => {
      const minimalRequest: CreatePipelineRequest = {
        name: "Minimal Pipeline",
      };

      expect(minimalRequest.name).toBeDefined();
      expect(minimalRequest.description).toBeUndefined();
      expect(minimalRequest.schedule).toBeUndefined();
      expect(minimalRequest.tags).toBeUndefined();
    });

    it("should validate UpdatePipelineRequest type", () => {
      const request: UpdatePipelineRequest = {
        name: "Updated Pipeline",
        status: "inactive",
      };

      expect(request.name).toBeDefined();
      expect(request.status).toMatch(/^(active|inactive|archived)$/);
      expect(request.description).toBeUndefined();
    });

    it("should validate optional Pipeline fields", () => {
      const minimalPipeline: Pipeline = {
        id: "123e4567-e89b-12d3-a456-426614174000",
        name: "Minimal Pipeline",
        status: "active",
      };

      expect(minimalPipeline.description).toBeUndefined();
      expect(minimalPipeline.schedule).toBeUndefined();
      expect(minimalPipeline.tags).toBeUndefined();
    });
  });

  describe("WorkerPool Types", () => {
    it("should validate WorkerPool type structure", () => {
      const pool: WorkerPool = {
        id: "123e4567-e89b-12d3-a456-426614174000",
        name: "Test Pool",
        poolType: "Docker",
        providerName: "docker-provider",
        minSize: 1,
        maxSize: 10,
        defaultResources: {
          cpu_m: 1000,
          memory_mb: 2048,
          gpu: 0,
        },
        tags: {
          env: "test",
          region: "us-east-1",
        },
      };

      expect(pool.poolType).toMatch(/^(Docker|Kubernetes|Native)$/);
      expect(pool.defaultResources?.cpu_m).toBeGreaterThan(0);
      expect(pool.defaultResources?.memory_mb).toBeGreaterThan(0);
    });

    it("should validate WorkerPoolStatus type", () => {
      const status: WorkerPoolStatus = {
        name: "Test Pool",
        poolType: "Docker",
        totalCapacity: 100,
        availableCapacity: 50,
        activeWorkers: 5,
        pendingRequests: 2,
      };

      expect(status.totalCapacity).toBeGreaterThanOrEqual(0);
      expect(status.availableCapacity).toBeLessThanOrEqual(
        status.totalCapacity,
      );
      expect(status.activeWorkers).toBeGreaterThanOrEqual(0);
      expect(status.pendingRequests).toBeGreaterThanOrEqual(0);
    });
  });

  describe("Error Type", () => {
    it("should validate Error type structure", () => {
      const error: Error = {
        code: "BAD_REQUEST",
        message: "Invalid request",
        details: {
          field: "email",
          reason: "Invalid format",
        },
        timestamp: "2025-11-30T10:00:00Z",
      };

      expect(error.code).toBeDefined();
      expect(error.message).toBeDefined();
      expect(error.timestamp).toBeDefined();
      expect(error.details).toBeDefined();
    });

    it("should accept Error with minimal fields", () => {
      const minimalError: Error = {
        code: "NOT_FOUND",
        message: "Resource not found",
        timestamp: "2025-11-30T10:00:00Z",
      };

      expect(minimalError.details).toBeUndefined();
    });
  });

  describe("Type Guards", () => {
    it("should validate Pipeline status values", () => {
      const validStatuses = ["active", "inactive", "archived"] as const;
      const isValidStatus = (
        status: string,
      ): status is (typeof validStatuses)[number] => {
        return validStatuses.includes(status as (typeof validStatuses)[number]);
      };

      expect(isValidStatus("active")).toBe(true);
      expect(isValidStatus("inactive")).toBe(true);
      expect(isValidStatus("archived")).toBe(true);
      expect(isValidStatus("invalid")).toBe(false);
    });

    it("should validate WorkerPool type values", () => {
      const validPoolTypes = ["Docker", "Kubernetes", "Native"] as const;
      const isValidPoolType = (
        type: string,
      ): type is (typeof validPoolTypes)[number] => {
        return validPoolTypes.includes(type as (typeof validPoolTypes)[number]);
      };

      expect(isValidPoolType("Docker")).toBe(true);
      expect(isValidPoolType("Kubernetes")).toBe(true);
      expect(isValidPoolType("Native")).toBe(true);
      expect(isValidPoolType("ECS")).toBe(false);
    });
  });

  describe("Response Type Validation", () => {
    it("should validate paginated Pipeline response", () => {
      const response = {
        items: [
          {
            id: "123e4567-e89b-12d3-a456-426614174000",
            name: "Pipeline 1",
            status: "active",
          },
          {
            id: "223e4567-e89b-12d3-a456-426614174000",
            name: "Pipeline 2",
            status: "inactive",
          },
        ],
        total: 2,
        hasMore: false,
      };

      expect(response.items).toHaveLength(2);
      expect(response.total).toBeGreaterThan(0);
      expect(typeof response.hasMore).toBe("boolean");
      expect(
        response.items.every(
          (p) =>
            p.status === "active" ||
            p.status === "inactive" ||
            p.status === "archived",
        ),
      ).toBe(true);
    });
  });
});
