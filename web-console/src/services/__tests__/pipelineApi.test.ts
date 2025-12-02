/**
 * @jest-environment jsdom
 */

import { http, HttpResponse } from "msw";
import { beforeEach, describe, expect, it } from "vitest";
import { server } from "../../test/__mocks__/msw/server";
import type { components } from "../../types/api";
import { APIError, pipelineApi } from "../pipelineApi";

// Type aliases
type Pipeline = components["schemas"]["PipelineDto"];
type PipelineSummary = components["schemas"]["PipelineSummaryDto"];
type CreatePipelineRequest = components["schemas"]["CreatePipelineRequestDto"];
type UpdatePipelineRequest = components["schemas"]["UpdatePipelineRequestDto"];
type Error = components["schemas"]["ErrorResponse"];

describe("PipelineApiService", () => {
  beforeEach(() => {
    server.resetHandlers();
    localStorage.setItem("auth_token", "test-token");
  });

  describe("listPipelines", () => {
    it("should fetch pipelines with default parameters", async () => {
      const mockResponse = {
        items: [
          {
            id: "123e4567-e89b-12d3-a456-426614174000",
            name: "Pipeline 1",
            status: "active",
            description: "Test pipeline",
            step_count: 0,
            created_at: "2025-11-30T10:00:00Z",
            updated_at: "2025-11-30T10:00:00Z",
          } as PipelineSummary,
        ],
        total: 1,
        hasMore: false,
      };

      server.use(
        http.get("http://localhost:8080/api/v1/pipelines", () => {
          return HttpResponse.json(mockResponse);
        }),
      );

      const result = await pipelineApi.listPipelines();

      expect(result).toEqual(mockResponse);
    });

    it("should fetch pipelines with custom parameters", async () => {
      const mockResponse = {
        items: [],
        total: 0,
        hasMore: false,
      };

      server.use(
        http.get("http://localhost:8080/api/v1/pipelines", ({ request }) => {
          const url = new URL(request.url);
          const limit = url.searchParams.get("limit");
          const offset = url.searchParams.get("offset");
          const status = url.searchParams.get("status");

          expect(limit).toBe("20");
          expect(offset).toBe("10");
          expect(status).toBe("active");

          return HttpResponse.json(mockResponse);
        }),
      );

      await pipelineApi.listPipelines({
        limit: 20,
        offset: 10,
        status: "active",
      });
    });

    it("should throw APIError on failed response", async () => {
      const error: Error = {
        code: "UNAUTHORIZED",
        message: "Authentication required",
        timestamp: "2025-11-30T10:00:00Z",
      };

      server.use(
        http.get("http://localhost:8080/api/v1/pipelines", () => {
          return HttpResponse.json(error, { status: 401 });
        }),
      );

      await expect(pipelineApi.listPipelines()).rejects.toThrow(APIError);
      await expect(pipelineApi.listPipelines()).rejects.toThrow(
        "Authentication required",
      );
    });
  });

  describe("getPipeline", () => {
    it("should fetch a single pipeline by ID", async () => {
      const pipeline: Pipeline = {
        id: "123e4567-e89b-12d3-a456-426614174000",
        name: "Test Pipeline",
        description: "Test description",
        status: "active",
        steps: [],
        created_at: "2025-11-30T10:00:00Z",
        updated_at: "2025-11-30T10:00:00Z",
      };

      server.use(
        http.get(
          "http://localhost:8080/api/v1/pipelines/:pipelineId",
          ({ params }) => {
            expect(params.pipelineId).toBe(
              "123e4567-e89b-12d3-a456-426614174000",
            );
            return HttpResponse.json(pipeline);
          },
        ),
      );

      const result = await pipelineApi.getPipeline(
        "123e4567-e89b-12d3-a456-426614174000",
      );

      expect(result).toEqual(pipeline);
    });

    it("should throw APIError when pipeline not found", async () => {
      const error: Error = {
        code: "NOT_FOUND",
        message: "Pipeline not found",
        timestamp: "2025-11-30T10:00:00Z",
      };

      server.use(
        http.get("http://localhost:8080/api/v1/pipelines/:pipelineId", () => {
          return HttpResponse.json(error, { status: 404 });
        }),
      );

      await expect(pipelineApi.getPipeline("non-existent-id")).rejects.toThrow(
        APIError,
      );
    });
  });

  describe("createPipeline", () => {
    it("should create a new pipeline", async () => {
      const request: CreatePipelineRequest = {
        name: "New Pipeline",
        description: "New pipeline description",
        steps: [],
      };

      const createdPipeline: Pipeline = {
        id: "223e4567-e89b-12d3-a456-426614174000",
        name: "New Pipeline",
        description: "New pipeline description",
        status: "active",
        steps: [],
        created_at: "2025-11-30T10:00:00Z",
        updated_at: "2025-11-30T10:00:00Z",
      };

      server.use(
        http.post("http://localhost:8080/api/v1/pipelines", async ({ request }) => {
          const body = await request.json() as CreatePipelineRequest;
          expect(body.name).toBe("New Pipeline");
          expect(body.description).toBe("New pipeline description");

          return HttpResponse.json(createdPipeline, { status: 200 });
        }),
      );

      const result = await pipelineApi.createPipeline(request);

      expect(result).toEqual(createdPipeline);
    });

    it("should validate request structure", async () => {
      const request: CreatePipelineRequest = {
        name: "Minimal Pipeline",
        steps: [],
      };

      server.use(
        http.post("http://localhost:8080/api/v1/pipelines", async ({ request }) => {
          const body = await request.json() as CreatePipelineRequest;
          expect(body.name).toBe("Minimal Pipeline");

          return HttpResponse.json(
            {
              id: "123",
              name: "Minimal Pipeline",
              status: "active",
              steps: [],
              created_at: "2025-11-30T10:00:00Z",
              updated_at: "2025-11-30T10:00:00Z",
            } as Pipeline,
            { status: 200 },
          );
        }),
      );

      const result = await pipelineApi.createPipeline(request);

      expect(result.name).toBe("Minimal Pipeline");
      expect(result.status).toBe("active");
    });

    it("should throw APIError on conflict", async () => {
      const request: CreatePipelineRequest = {
        name: "Duplicate Pipeline",
        steps: [],
      };

      const error: Error = {
        code: "CONFLICT",
        message: "Pipeline with this name already exists",
        timestamp: "2025-11-30T10:00:00Z",
      };

      server.use(
        http.post("http://localhost:8080/api/v1/pipelines", () => {
          return HttpResponse.json(error, { status: 409 });
        }),
      );

      await expect(pipelineApi.createPipeline(request)).rejects.toThrow(
        APIError,
      );
    });
  });

  describe("updatePipeline", () => {
    it("should update an existing pipeline", async () => {
      const pipelineId = "123e4567-e89b-12d3-a456-426614174000";
      const updateRequest: UpdatePipelineRequest = {
        description: "Updated description",
      };

      const updatedPipeline: Pipeline = {
        id: pipelineId,
        name: "Test Pipeline",
        description: "Updated description",
        status: "active",
        steps: [],
        created_at: "2025-11-30T10:00:00Z",
        updated_at: "2025-11-30T11:00:00Z",
      };

      server.use(
        http.put(
          "http://localhost:8080/api/v1/pipelines/:pipelineId",
          async ({ request, params }) => {
            expect(params.pipelineId).toBe(pipelineId);

            const body = await request.json() as UpdatePipelineRequest;
            expect(body.description).toBe("Updated description");

            return HttpResponse.json(updatedPipeline);
          },
        ),
      );

      const result = await pipelineApi.updatePipeline(
        pipelineId,
        updateRequest,
      );

      expect(result.description).toBe("Updated description");
    });
  });

  describe("deletePipeline", () => {
    it("should delete a pipeline successfully", async () => {
      const pipelineId = "123e4567-e89b-12d3-a456-426614174000";

      server.use(
        http.delete(
          "http://localhost:8080/api/v1/pipelines/:pipelineId",
          ({ params }) => {
            expect(params.pipelineId).toBe(pipelineId);
            return new HttpResponse(null, { status: 204 });
          },
        ),
      );

      await pipelineApi.deletePipeline(pipelineId);
    });

    it("should throw APIError when pipeline not found", async () => {
      const error: Error = {
        code: "NOT_FOUND",
        message: "Pipeline not found",
        timestamp: "2025-11-30T10:00:00Z",
      };

      server.use(
        http.delete("http://localhost:8080/api/v1/pipelines/:pipelineId", () => {
          return HttpResponse.json(error, { status: 404 });
        }),
      );

      await expect(
        pipelineApi.deletePipeline("non-existent-id"),
      ).rejects.toThrow(APIError);
    });
  });

  describe("Type Safety", () => {
    it("should ensure all responses match OpenAPI schema", async () => {
      const mockPipeline: Pipeline = {
        id: "123e4567-e89b-12d3-a456-426614174000",
        name: "Test Pipeline",
        status: "active",
        description: "Test description",
        steps: [],
        created_at: "2025-11-30T10:00:00Z",
        updated_at: "2025-11-30T10:00:00Z",
      };

      server.use(
        http.get("http://localhost:8080/api/v1/pipelines/:pipelineId", () => {
          return HttpResponse.json(mockPipeline);
        }),
      );

      const result = await pipelineApi.getPipeline("123");

      // TypeScript will ensure these fields exist
      expect(result.id).toBeDefined();
      expect(result.name).toBeDefined();
      expect(result.status).toBeDefined();
    });
  });

  describe("APIError", () => {
    it("should properly format APIError from response", async () => {
      const error: Error = {
        code: "BAD_REQUEST",
        message: "Invalid input",
        details: '{"field":"name","reason":"too short"}',
        timestamp: "2025-11-30T10:00:00Z",
      };

      server.use(
        http.get("http://localhost:8080/api/v1/pipelines", () => {
          return HttpResponse.json(error, { status: 400 });
        }),
      );

      await expect(pipelineApi.listPipelines()).rejects.toThrow();

      try {
        await pipelineApi.listPipelines();
      } catch (err) {
        expect(err).toBeInstanceOf(APIError);
        expect(err).toHaveProperty("code", "BAD_REQUEST");
        expect(err).toHaveProperty("message", "Invalid input");
        expect(err).toHaveProperty("details", '{"field":"name","reason":"too short"}');
        expect(err).toHaveProperty("timestamp", "2025-11-30T10:00:00Z");
      }
    });
  });
});
