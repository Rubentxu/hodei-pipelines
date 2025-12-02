import { http, HttpResponse } from "msw";
import { beforeEach, describe, expect, it } from "vitest";
import { server } from "../../test/__mocks__/msw/server";
import {
  aggregateLogs,
  exportLogs,
  getLogLevels,
  getLogs,
  searchLogs
} from "../logsExplorerApi";

const setupServer = () => {
  return server;
};

describe("logsExplorerApi", () => {
  beforeEach(() => {
    server.resetHandlers();
  });

  describe("getLogs", () => {
    it("should fetch logs successfully", async () => {
      const mockResponse = {
        logs: [
          {
            id: "log-1",
            timestamp: "2025-11-30T09:30:00Z",
            level: "error",
            service: "worker-01",
            message: "Failed to process job",
            traceId: "trace-123",
            spanId: "span-456",
            metadata: { jobId: "job-789" },
          },
          {
            id: "log-2",
            timestamp: "2025-11-30T09:29:45Z",
            level: "info",
            service: "worker-01",
            message: "Job completed successfully",
          },
        ],
        total: 2,
        hasMore: false,
      };

      server.use(
        http.get("/api/v1/logs", () => {
          return HttpResponse.json(mockResponse);
        }),
      );

      const result = await getLogs();
      expect(result).toEqual(mockResponse);
      expect(result.logs).toHaveLength(2);
    });

    it("should filter logs by level", async () => {
      const mockResponse = {
        logs: [
          {
            id: "log-1",
            timestamp: "2025-11-30T09:30:00Z",
            level: "error",
            service: "worker-01",
            message: "Error message",
          },
        ],
        total: 1,
      };

      server.use(
        http.get("/api/v1/logs", ({ request }) => {
          const url = new URL(request.url);
          expect(url.searchParams.get("level")).toBe("error");
          return HttpResponse.json(mockResponse);
        }),
      );

      const result = await getLogs({ level: "error" });
      expect(result.logs[0].level).toBe("error");
    });

    it("should filter logs by date range", async () => {
      const mockResponse = {
        logs: [],
        total: 0,
      };

      server.use(
        http.get("/api/v1/logs", ({ request }) => {
          const url = new URL(request.url);
          expect(url.searchParams.get("startTime")).toBe(
            "2025-11-30T00:00:00Z",
          );
          expect(url.searchParams.get("endTime")).toBe("2025-11-30T23:59:59Z");
          return HttpResponse.json(mockResponse);
        }),
      );

      await getLogs({
        startTime: "2025-11-30T00:00:00Z",
        endTime: "2025-11-30T23:59:59Z",
      });
    });

    it("should filter logs by search term", async () => {
      const mockResponse = {
        logs: [
          {
            id: "log-1",
            timestamp: "2025-11-30T09:30:00Z",
            level: "info",
            service: "worker-01",
            message: "Processing payment transaction",
          },
        ],
        total: 1,
      };

      server.use(
        http.get("/api/v1/logs", ({ request }) => {
          const url = new URL(request.url);
          expect(url.searchParams.get("search")).toBe("payment");
          return HttpResponse.json(mockResponse);
        }),
      );

      const result = await getLogs({ search: "payment" });
      expect(result.logs[0].message).toContain("payment");
    });

    it("should apply pagination", async () => {
      const mockResponse = {
        logs: [],
        total: 100,
      };

      server.use(
        http.get("/api/v1/logs", ({ request }) => {
          const url = new URL(request.url);
          expect(url.searchParams.get("limit")).toBe("50");
          expect(url.searchParams.get("offset")).toBe("0");
          return HttpResponse.json(mockResponse);
        }),
      );

      await getLogs({ limit: 50, offset: 0 });
    });

    it("should throw error on fetch failure", async () => {
      server.use(
        http.get("/api/v1/logs", () => {
          return HttpResponse.json(
            { message: "Server error" },
            { status: 500 },
          );
        }),
      );

      await expect(getLogs()).rejects.toThrow("Failed to fetch logs");
    });
  });

  describe("getLogLevels", () => {
    it("should return available log levels", async () => {
      const mockLevels = ["debug", "info", "warn", "error", "fatal"];

      server.use(
        http.get("/api/v1/logs/levels", () => {
          return HttpResponse.json(mockLevels);
        }),
      );

      const levels = await getLogLevels();
      expect(levels).toEqual(mockLevels);
    });
  });

  describe("exportLogs", () => {
    it("should export logs as CSV", async () => {
      const mockResponse =
        "timestamp,level,service,message\n2025-11-30T09:30:00Z,error,worker-01,Error message\n";

      server.use(
        http.get("/api/v1/logs/export", ({ request }) => {
          const url = new URL(request.url);
          expect(url.searchParams.get("format")).toBe("csv");
          return new HttpResponse(mockResponse, {
            headers: {
              "Content-Type": "text/csv",
              "Content-Disposition": 'attachment; filename="logs.csv"',
            },
          });
        }),
      );

      const blob = await exportLogs(undefined, { format: "csv" });
      expect(blob).toBeInstanceOf(Blob);
    });

    it("should export logs as JSON", async () => {
      const mockData = [{ id: "log-1", level: "error", message: "Test" }];

      server.use(
        http.get("/api/v1/logs/export", () => {
          return new HttpResponse(JSON.stringify(mockData), {
            headers: {
              "Content-Type": "application/json",
              "Content-Disposition": 'attachment; filename="logs.json"',
            },
          });
        }),
      );

      const blob = await exportLogs(undefined, { format: "json" });
      expect(blob).toBeInstanceOf(Blob);
    });
  });

  describe("aggregateLogs", () => {
    it("should aggregate logs by service", async () => {
      const mockAggregation = {
        byService: {
          "worker-01": 45,
          "worker-02": 32,
          "worker-03": 28,
        },
        total: 105,
      };

      server.use(
        http.get("/api/v1/logs/aggregate", ({ request }) => {
          const url = new URL(request.url);
          expect(url.searchParams.get("groupBy")).toBe("service");
          return HttpResponse.json(mockAggregation);
        }),
      );

      const result = await aggregateLogs({ groupBy: "service" });
      expect(result.byService).toHaveProperty("worker-01");
      expect(result.total).toBe(105);
    });

    it("should aggregate logs by level", async () => {
      const mockAggregation = {
        byLevel: {
          error: 12,
          warn: 8,
          info: 85,
        },
        total: 105,
      };

      server.use(
        http.get("/api/v1/logs/aggregate", ({ request }) => {
          const url = new URL(request.url);
          expect(url.searchParams.get("groupBy")).toBe("level");
          return HttpResponse.json(mockAggregation);
        }),
      );

      const result = await aggregateLogs({ groupBy: "level" });
      expect(result.byLevel).toHaveProperty("error");
      expect(result.byLevel?.error).toBe(12);
    });
  });

  describe("searchLogs", () => {
    it("should search logs with advanced query", async () => {
      const mockResponse = {
        logs: [
          {
            id: "log-1",
            timestamp: "2025-11-30T09:30:00Z",
            level: "error",
            service: "worker-01",
            message: "Database connection failed",
          },
        ],
        total: 1,
        searchQuery: "database AND connection",
        searchTime: 0.045,
      };

      server.use(
        http.post("/api/v1/logs/search", async ({ request }) => {
          const body = (await request.json()) as any;
          expect(body.query.query).toBe("database AND connection");
          expect(body.query.fields).toEqual(["message", "traceId"]);
          return HttpResponse.json(mockResponse);
        }),
      );

      const result = await searchLogs({
        query: "database AND connection",
        fields: ["message", "traceId"],
        fuzzy: false,
        caseSensitive: false,
      });

      expect(result.searchQuery).toBe("database AND connection");
      expect(result.searchTime).toBe(0.045);
      expect(result.logs).toHaveLength(1);
    });
  });
});
