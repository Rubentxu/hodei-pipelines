/**
 * @jest-environment jsdom
 */

import { describe, it, expect, beforeEach } from "vitest";
import { http, HttpResponse } from "msw";
import { server } from "../../test/__mocks__/msw/server";
import { ApiClient, ApiClientError, createApiClient } from "../api-client";
import type {
  PaginatedResponse,
  ErrorResponse,
} from "../../types/api-responses";

describe("ApiClient", () => {
  beforeEach(() => {
    server.resetHandlers();
  });

  describe("Constructor", () => {
    it("should create client with base URL", () => {
      const client = new ApiClient({
        baseUrl: "https://api.example.com",
      });

      expect(client).toBeInstanceOf(ApiClient);
    });

    it("should create client with custom configuration", () => {
      const client = new ApiClient({
        baseUrl: "https://api.example.com",
        defaultHeaders: {
          "X-Custom-Header": "value",
        },
        getAuthToken: () => "test-token",
      });

      expect(client).toBeInstanceOf(ApiClient);
    });
  });

  describe("GET requests", () => {
    it("should perform GET request successfully", async () => {
      const mockData = { id: 1, name: "Test" };

      server.use(
        http.get("https://api.example.com/test", () => {
          return HttpResponse.json(mockData);
        }),
      );

      const client = new ApiClient({
        baseUrl: "https://api.example.com",
      });

      const result = await client.get("/test");

      expect(result.data).toEqual(mockData);
    });

    it("should handle GET request with query parameters", async () => {
      const mockData = { items: [] };

      server.use(
        http.get("https://api.example.com/test", ({ request }) => {
          const url = new URL(request.url);
          expect(url.searchParams.get("filter")).toBe("active");
          expect(url.searchParams.get("limit")).toBe("10");

          return HttpResponse.json(mockData);
        }),
      );

      const client = new ApiClient({
        baseUrl: "https://api.example.com",
      });

      const result = await client.get("/test", {
        params: {
          filter: "active",
          limit: 10,
        },
      });

      expect(result.data).toEqual(mockData);
    });

    it("should include authentication token when configured", async () => {
      const mockData = { id: 1 };

      server.use(
        http.get("https://api.example.com/test", ({ request }) => {
          const authHeader = request.headers.get("Authorization");
          expect(authHeader).toBe("Bearer test-token-123");

          return HttpResponse.json(mockData);
        }),
      );

      const client = new ApiClient({
        baseUrl: "https://api.example.com",
        getAuthToken: () => "test-token-123",
      });

      const result = await client.get("/test", { auth: true });

      expect(result.data).toEqual(mockData);
    });

    it("should skip authentication when auth is false", async () => {
      const mockData = { id: 1 };

      server.use(
        http.get("https://api.example.com/test", ({ request }) => {
          const authHeader = request.headers.get("Authorization");
          expect(authHeader).toBeNull();

          return HttpResponse.json(mockData);
        }),
      );

      const client = new ApiClient({
        baseUrl: "https://api.example.com",
        getAuthToken: () => "test-token-123",
      });

      const result = await client.get("/test", { auth: false });

      expect(result.data).toEqual(mockData);
    });

    it("should throw ApiClientError on failed response", async () => {
      const errorResponse: ErrorResponse = {
        success: false,
        code: "NOT_FOUND",
        message: "Resource not found",
        timestamp: "2025-11-30T10:00:00Z",
      };

      server.use(
        http.get("https://api.example.com/test", () => {
          return HttpResponse.json(errorResponse, { status: 404 });
        }),
      );

      const client = new ApiClient({
        baseUrl: "https://api.example.com",
      });

      await expect(client.get("/test")).rejects.toThrow(ApiClientError);
      await expect(client.get("/test")).rejects.toThrow("Resource not found");
    });
  });

  describe("POST requests", () => {
    it("should perform POST request with body", async () => {
      const requestBody = { name: "New Item" };
      const responseData = { id: 1, name: "New Item" };

      server.use(
        http.post("https://api.example.com/test", async ({ request }) => {
          const body = await request.json();
          expect(body).toEqual(requestBody);

          return HttpResponse.json(responseData, { status: 201 });
        }),
      );

      const client = new ApiClient({
        baseUrl: "https://api.example.com",
      });

      const result = await client.post("/test", {
        body: requestBody,
      });

      expect(result.data).toEqual(responseData);
    });
  });

  describe("PATCH requests", () => {
    it("should perform PATCH request with body", async () => {
      const requestBody = { name: "Updated Item" };
      const responseData = { id: 1, name: "Updated Item" };

      server.use(
        http.patch("https://api.example.com/test/:id", async ({ request }) => {
          const body = await request.json();
          expect(body).toEqual(requestBody);

          return HttpResponse.json(responseData);
        }),
      );

      const client = new ApiClient({
        baseUrl: "https://api.example.com",
      });

      const result = await client.patch("/test/1", {
        body: requestBody,
      });

      expect(result.data).toEqual(responseData);
    });
  });

  describe("PUT requests", () => {
    it("should perform PUT request with body", async () => {
      const requestBody = { name: "Replaced Item" };
      const responseData = { id: 1, name: "Replaced Item" };

      server.use(
        http.put("https://api.example.com/test/:id", async ({ request }) => {
          const body = await request.json();
          expect(body).toEqual(requestBody);

          return HttpResponse.json(responseData);
        }),
      );

      const client = new ApiClient({
        baseUrl: "https://api.example.com",
      });

      const result = await client.put("/test/1", {
        body: requestBody,
      });

      expect(result.data).toEqual(responseData);
    });
  });

  describe("DELETE requests", () => {
    it("should perform DELETE request", async () => {
      server.use(
        http.delete("https://api.example.com/test/:id", ({ params }) => {
          expect(params.id).toBe("1");
          return new HttpResponse(null, { status: 204 });
        }),
      );

      const client = new ApiClient({
        baseUrl: "https://api.example.com",
      });

      const result = await client.delete("/test/1");

      expect(result.data).toBe("");
    });
  });

  describe("Helper methods", () => {
    describe("getPaginated", () => {
      it("should handle paginated requests", async () => {
        const mockResponse: PaginatedResponse<string> = {
          items: ["item1", "item2"],
          total: 50,
          hasMore: true,
        };

        server.use(
          http.get("https://api.example.com/test", ({ request }) => {
            const url = new URL(request.url);
            expect(url.searchParams.get("offset")).toBe("20");
            expect(url.searchParams.get("limit")).toBe("20");

            return HttpResponse.json(mockResponse);
          }),
        );

        const client = new ApiClient({
          baseUrl: "https://api.example.com",
        });

        const result = await client.getPaginated("/test", {
          page: 1,
          limit: 20,
        });

        expect(result.data).toEqual(mockResponse);
      });
    });

    describe("getSingle", () => {
      it("should handle single item requests", async () => {
        const mockItem = { id: 1, name: "Test Item" };

        server.use(
          http.get("https://api.example.com/test/:id", () => {
            return HttpResponse.json(mockItem);
          }),
        );

        const client = new ApiClient({
          baseUrl: "https://api.example.com",
        });

        const result = await client.getSingle("/test/1");

        expect(result.data).toEqual(mockItem);
      });
    });

    describe("getList", () => {
      it("should handle list requests", async () => {
        const mockList = ["item1", "item2", "item3"];

        server.use(
          http.get("https://api.example.com/test", () => {
            return HttpResponse.json(mockList);
          }),
        );

        const client = new ApiClient({
          baseUrl: "https://api.example.com",
        });

        const result = await client.getList("/test");

        expect(result.data).toEqual(mockList);
      });
    });
  });

  describe("createApiClient", () => {
    it("should create client with partial config", () => {
      const client = createApiClient({
        baseUrl: "https://api.example.com",
        defaultHeaders: {
          "X-Custom": "value",
        },
      });

      expect(client).toBeInstanceOf(ApiClient);
    });

    it("should use default config for missing values", () => {
      const defaultClient = new ApiClient({
        baseUrl: "https://default.com",
      });

      const customClient = createApiClient({
        defaultHeaders: {
          "X-Custom": "value",
        },
      });

      // Both should work with their respective configs
      expect(defaultClient).toBeInstanceOf(ApiClient);
      expect(customClient).toBeInstanceOf(ApiClient);
    });
  });

  describe("Error handling", () => {
    it("should handle non-JSON error responses", async () => {
      server.use(
        http.get("https://api.example.com/test", () => {
          return new Response("Internal Server Error", {
            status: 500,
            headers: {
              "Content-Type": "text/plain",
            },
          });
        }),
      );

      const client = new ApiClient({
        baseUrl: "https://api.example.com",
      });

      await expect(client.get("/test")).rejects.toThrow("HTTP Error 500");
    });

    it("should handle JSON error responses", async () => {
      server.use(
        http.get("https://api.example.com/test", () => {
          return HttpResponse.json(
            { code: "NETWORK_ERROR", message: "Network error" },
            { status: 503 },
          );
        }),
      );

      const client = new ApiClient({
        baseUrl: "https://api.example.com",
      });

      await expect(client.get("/test")).rejects.toThrow();
    });
  });

  describe("Metadata extraction", () => {
    it("should extract meta from response when available", async () => {
      const responseWithMeta = {
        data: ["item1", "item2"],
        meta: {
          pagination: {
            page: 0,
            limit: 10,
          },
        },
      };

      server.use(
        http.get("https://api.example.com/test", () => {
          return HttpResponse.json(responseWithMeta);
        }),
      );

      const client = new ApiClient({
        baseUrl: "https://api.example.com",
      });

      const result = await client.get("/test");

      expect(result.data).toEqual(responseWithMeta);
      expect(result.meta?.pagination?.page).toBe(0);
    });
  });
});
