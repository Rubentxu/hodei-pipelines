/**
 * Standardized API Client
 *
 * Provides a unified interface for making API requests with consistent
 * response handling across the application.
 */

import type {
  PaginatedResponse,
  SingleItemResponse,
  SuccessResponse,
  ErrorResponse,
  StandardApiResponse,
  PaginationMeta,
} from "../types/api-responses";

/**
 * API Client configuration
 */
export interface ApiClientConfig {
  /** Base URL for API requests */
  baseUrl: string;
  /** Default headers to include in all requests */
  defaultHeaders?: Record<string, string>;
  /** Authentication token getter */
  getAuthToken?: () => string | null;
}

/**
 * Request options
 */
export interface RequestOptions {
  /** Request headers */
  headers?: Record<string, string>;
  /** Query parameters */
  params?: Record<string, string | number | boolean | undefined>;
  /** Request body */
  body?: unknown;
  /** Whether to include authentication token */
  auth?: boolean;
}

/**
 * Response data with metadata
 */
export interface ResponseData<T> {
  /** Response data */
  data: T;
  /** Response metadata */
  meta?: {
    pagination?: PaginationMeta;
    [key: string]: unknown;
  };
}

/**
 * Standardized API Error class
 */
export class ApiClientError extends Error {
  /** Error code */
  code: string;
  /** Error details */
  details?: Record<string, unknown>;
  /** When the error occurred */
  timestamp: string;

  constructor(error: ErrorResponse) {
    super(error.message);
    this.name = "ApiClientError";
    this.code = error.code;
    this.details = error.details;
    this.timestamp = error.timestamp;
  }
}

/**
 * Standardized API Client
 */
export class ApiClient {
  private config: ApiClientConfig;

  constructor(config: ApiClientConfig) {
    this.config = config;
  }

  /**
   * Build URL with query parameters
   */
  private buildUrl(
    endpoint: string,
    params?: Record<string, string | number | boolean | undefined>,
  ): string {
    const url = new URL(endpoint, this.config.baseUrl);

    if (params) {
      Object.entries(params).forEach(([key, value]) => {
        if (value !== undefined && value !== null) {
          url.searchParams.append(key, String(value));
        }
      });
    }

    return url.toString();
  }

  /**
   * Build request headers
   */
  private buildHeaders(options?: RequestOptions): Record<string, string> {
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
      ...this.config.defaultHeaders,
      ...options?.headers,
    };

    // Add auth token if requested
    if (options?.auth !== false && this.config.getAuthToken) {
      const token = this.config.getAuthToken();
      if (token) {
        headers["Authorization"] = `Bearer ${token}`;
      }
    }

    return headers;
  }

  /**
   * Handle API response and extract data
   */
  private async handleResponse<T>(
    response: Response,
  ): Promise<ResponseData<T>> {
    const contentType = response.headers.get("content-type");
    const isJson = contentType?.includes("application/json");

    if (!response.ok) {
      if (isJson) {
        const errorData = (await response.json()) as ErrorResponse;
        throw new ApiClientError(errorData);
      } else {
        throw new ApiClientError({
          success: false,
          code: `HTTP_${response.status}`,
          message: `HTTP Error ${response.status}`,
          timestamp: new Date().toISOString(),
        });
      }
    }

    if (isJson) {
      const data = (await response.json()) as T;

      // Try to extract pagination info if available
      let meta: ResponseData<T>["meta"];

      // Check for standard format
      if (typeof data === "object" && data !== null && "meta" in data) {
        meta = (data as any).meta;
      }

      return { data, meta };
    } else {
      // For non-JSON responses, return as-is
      // This is typically used for DELETE requests with 204 status
      const text = await response.text();
      return { data: text as unknown as T };
    }
  }

  /**
   * Perform GET request
   */
  async get<T>(
    endpoint: string,
    options?: RequestOptions,
  ): Promise<ResponseData<T>> {
    const url = this.buildUrl(endpoint, options?.params);
    const headers = this.buildHeaders(options);

    const response = await fetch(url, {
      method: "GET",
      headers,
    });

    return this.handleResponse<T>(response);
  }

  /**
   * Perform POST request
   */
  async post<T>(
    endpoint: string,
    options?: RequestOptions,
  ): Promise<ResponseData<T>> {
    const url = this.buildUrl(endpoint, options?.params);
    const headers = this.buildHeaders(options);

    const response = await fetch(url, {
      method: "POST",
      headers,
      body: options?.body ? JSON.stringify(options.body) : undefined,
    });

    return this.handleResponse<T>(response);
  }

  /**
   * Perform PATCH request
   */
  async patch<T>(
    endpoint: string,
    options?: RequestOptions,
  ): Promise<ResponseData<T>> {
    const url = this.buildUrl(endpoint, options?.params);
    const headers = this.buildHeaders(options);

    const response = await fetch(url, {
      method: "PATCH",
      headers,
      body: options?.body ? JSON.stringify(options.body) : undefined,
    });

    return this.handleResponse<T>(response);
  }

  /**
   * Perform PUT request
   */
  async put<T>(
    endpoint: string,
    options?: RequestOptions,
  ): Promise<ResponseData<T>> {
    const url = this.buildUrl(endpoint, options?.params);
    const headers = this.buildHeaders(options);

    const response = await fetch(url, {
      method: "PUT",
      headers,
      body: options?.body ? JSON.stringify(options.body) : undefined,
    });

    return this.handleResponse<T>(response);
  }

  /**
   * Perform DELETE request
   */
  async delete<T>(
    endpoint: string,
    options?: RequestOptions,
  ): Promise<ResponseData<T>> {
    const url = this.buildUrl(endpoint, options?.params);
    const headers = this.buildHeaders(options);

    const response = await fetch(url, {
      method: "DELETE",
      headers,
    });

    return this.handleResponse<T>(response);
  }

  /**
   * Get paginated results with helper method
   */
  async getPaginated<T>(
    endpoint: string,
    options?: RequestOptions & { page?: number; limit?: number },
  ): Promise<ResponseData<PaginatedResponse<T>>> {
    const params = {
      ...options?.params,
      offset: options?.page ? options.page * (options?.limit || 50) : undefined,
      limit: options?.limit || 50,
    };

    return this.get<PaginatedResponse<T>>(endpoint, { ...options, params });
  }

  /**
   * Get single item
   */
  async getSingle<T>(
    endpoint: string,
    options?: RequestOptions,
  ): Promise<ResponseData<SingleItemResponse<T>>> {
    return this.get<SingleItemResponse<T>>(endpoint, options);
  }

  /**
   * Get list (simple array)
   */
  async getList<T>(
    endpoint: string,
    options?: RequestOptions,
  ): Promise<ResponseData<T[]>> {
    return this.get<T[]>(endpoint, options);
  }
}

/**
 * Default API client instance
 */
const defaultApiClient = new ApiClient({
  baseUrl: import.meta.env.VITE_API_BASE_URL || "http://localhost:8080/v1",
  getAuthToken: () => localStorage.getItem("auth_token"),
});

/**
 * Export default instance
 */
export { defaultApiClient };

/**
 * Helper function to create a new API client with custom config
 */
export function createApiClient(config: Partial<ApiClientConfig>): ApiClient {
  return new ApiClient({
    baseUrl: config.baseUrl || defaultApiClient.config.baseUrl,
    defaultHeaders: config.defaultHeaders,
    getAuthToken: config.getAuthToken,
  });
}
