/**
 * Pipeline API Service
 *
 * This service demonstrates TypeScript code generation from OpenAPI specifications.
 * All types are auto-generated from docs/openapi.yaml using the openapi-typescript package.
 */

import type { components, operations } from "../types/api";

// Type aliases for better readability
export type Pipeline = components["schemas"]["PipelineDto"];
export type CreatePipelineRequest = components["schemas"]["CreatePipelineRequestDto"];
export type UpdatePipelineRequest = components["schemas"]["UpdatePipelineRequestDto"];
export type Error = components["schemas"]["ErrorResponse"];

// Response types from operations
export type ListPipelinesResponse =
  operations["list_pipelines_handler"]["responses"][200]["content"]["application/json"];
export type CreatePipelineResponse =
  operations["create_pipeline_handler"]["responses"][200]["content"]["application/json"];
export type GetPipelineResponse =
  operations["get_pipeline_handler"]["responses"][200]["content"]["application/json"];
export type ExecutePipelineResponse =
  operations["execute_pipeline_handler"]["responses"][200]["content"]["application/json"];

const API_BASE_URL =
  import.meta.env.VITE_API_BASE_URL || "http://localhost:8080/api/v1";

// Generic API error class
export class APIError extends Error {
  code: string;
  details?: string | null;
  timestamp: string;

  constructor(error: Error) {
    super(error.message);
    this.name = "APIError";
    this.code = error.code;
    this.details = error.details;
    this.timestamp = error.timestamp;
  }
}

/**
 * Pipeline API client with type-safe operations
 */
export class PipelineApiService {
  private baseUrl: string;

  constructor(baseUrl: string = API_BASE_URL) {
    this.baseUrl = baseUrl;
  }

  /**
   * List all pipelines with optional filtering and pagination
   */
  async listPipelines(params?: {
    limit?: number;
    offset?: number;
    status?: string;
  }): Promise<ListPipelinesResponse> {
    const searchParams = new URLSearchParams();

    if (params?.limit !== undefined) {
      searchParams.append("limit", params.limit.toString());
    }
    if (params?.offset !== undefined) {
      searchParams.append("offset", params.offset.toString());
    }
    if (params?.status !== undefined) {
      searchParams.append("status", params.status);
    }

    const response = await fetch(
      `${this.baseUrl}/pipelines?${searchParams.toString()}`,
      {
        method: "GET",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${localStorage.getItem("auth_token") || ""}`,
        },
      },
    );

    if (!response.ok) {
      const error = (await response.json()) as Error;
      throw new APIError(error);
    }

    return response.json() as Promise<ListPipelinesResponse>;
  }

  /**
   * Get a single pipeline by ID
   */
  async getPipeline(pipelineId: string): Promise<GetPipelineResponse> {
    const response = await fetch(`${this.baseUrl}/pipelines/${pipelineId}`, {
      method: "GET",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${localStorage.getItem("auth_token") || ""}`,
      },
    });

    if (!response.ok) {
      const error = (await response.json()) as Error;
      throw new APIError(error);
    }

    return response.json() as Promise<GetPipelineResponse>;
  }

  /**
   * Create a new pipeline
   */
  async createPipeline(
    request: CreatePipelineRequest,
  ): Promise<CreatePipelineResponse> {
    const response = await fetch(`${this.baseUrl}/pipelines`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${localStorage.getItem("auth_token") || ""}`,
      },
      body: JSON.stringify(request),
    });

    if (!response.ok) {
      const error = (await response.json()) as Error;
      throw new APIError(error);
    }

    return response.json() as Promise<CreatePipelineResponse>;
  }

  /**
   * Update an existing pipeline
   */
  async updatePipeline(
    pipelineId: string,
    request: UpdatePipelineRequest,
  ): Promise<GetPipelineResponse> {
    const response = await fetch(`${this.baseUrl}/pipelines/${pipelineId}`, {
      method: "PUT",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${localStorage.getItem("auth_token") || ""}`,
      },
      body: JSON.stringify(request),
    });

    if (!response.ok) {
      const error = (await response.json()) as Error;
      throw new APIError(error);
    }

    return response.json() as Promise<GetPipelineResponse>;
  }

  /**
   * Delete a pipeline
   */
  async deletePipeline(pipelineId: string): Promise<void> {
    const response = await fetch(`${this.baseUrl}/pipelines/${pipelineId}`, {
      method: "DELETE",
      headers: {
        Authorization: `Bearer ${localStorage.getItem("auth_token") || ""}`,
      },
    });

    if (!response.ok) {
      const error = (await response.json()) as Error;
      throw new APIError(error);
    }
  }

  /**
   * Execute a pipeline
   */
  async executePipeline(pipelineId: string): Promise<ExecutePipelineResponse> {
    const response = await fetch(`${this.baseUrl}/pipelines/${pipelineId}/execute`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${localStorage.getItem("auth_token") || ""}`,
      },
      body: JSON.stringify({}), // Empty body for now
    });

    if (!response.ok) {
      const error = (await response.json()) as Error;
      throw new APIError(error);
    }

    return response.json() as Promise<ExecutePipelineResponse>;
  }
}

// Export singleton instance
export const pipelineApi = new PipelineApiService();

// Export types for use in components
export type {
  Error as ApiError
};

