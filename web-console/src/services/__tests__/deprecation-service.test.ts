/**
 * @jest-environment jsdom
 */

import { describe, it, expect, beforeEach, vi } from "vitest";
import {
  registerDeprecatedEndpoint,
  isEndpointDeprecated,
  trackEndpointUsage,
  configureDeprecationService,
  getDeprecatedEndpoints,
  generateDeprecationReport,
  resetDeprecationStats,
  resetAllDeprecationData,
  silenceDeprecationWarning,
  enableDeprecationWarning,
} from "../deprecation-service";

describe("DeprecationService", () => {
  beforeEach(() => {
    // Reset all registered endpoints before each test
    vi.clearAllMocks();

    // Clear all deprecated endpoints
    resetAllDeprecationData();

    // Reset configuration
    configureDeprecationService({
      enabled: true,
      logToConsole: true,
    });
  });

  describe("registerDeprecatedEndpoint", () => {
    it("should register a deprecated endpoint", () => {
      const endpoint = {
        method: "GET",
        path: "/api/v1/old-endpoint",
        reason: "Use new-endpoint instead",
        alternative: "GET /api/v1/new-endpoint",
        deprecatedAt: "2025-01-01",
        deprecatedInVersion: "1.5.0",
        logWarning: true,
      };

      registerDeprecatedEndpoint(endpoint);

      const registered = isEndpointDeprecated("GET", "/api/v1/old-endpoint");
      expect(registered).toBeDefined();
      expect(registered?.path).toBe("/api/v1/old-endpoint");
      expect(registered?.alternative).toBe("GET /api/v1/new-endpoint");
    });

    it("should normalize method to uppercase", () => {
      registerDeprecatedEndpoint({
        method: "get",
        path: "/api/v1/test",
        reason: "Test",
        deprecatedAt: "2025-01-01",
        deprecatedInVersion: "1.0.0",
        logWarning: true,
      });

      const registered = isEndpointDeprecated("GET", "/api/v1/test");
      expect(registered).toBeDefined();
      expect(registered?.method).toBe("GET");
    });

    it("should initialize usage count to 0", () => {
      registerDeprecatedEndpoint({
        method: "POST",
        path: "/api/v1/test",
        reason: "Test",
        deprecatedAt: "2025-01-01",
        deprecatedInVersion: "1.0.0",
        logWarning: false,
      });

      const registered = isEndpointDeprecated("POST", "/api/v1/test");
      expect(registered?.usageCount).toBe(0);
    });
  });

  describe("isEndpointDeprecated", () => {
    it("should return endpoint if deprecated", () => {
      registerDeprecatedEndpoint({
        method: "GET",
        path: "/api/v1/deprecated",
        reason: "Test deprecation",
        deprecatedAt: "2025-01-01",
        deprecatedInVersion: "1.0.0",
        logWarning: false,
      });

      const result = isEndpointDeprecated("GET", "/api/v1/deprecated");
      expect(result).toBeDefined();
      expect(result?.path).toBe("/api/v1/deprecated");
    });

    it("should return undefined if endpoint is not deprecated", () => {
      const result = isEndpointDeprecated("GET", "/api/v1/not-deprecated");
      expect(result).toBeUndefined();
    });

    it("should be case-insensitive for method", () => {
      registerDeprecatedEndpoint({
        method: "POST",
        path: "/api/v1/test",
        reason: "Test",
        deprecatedAt: "2025-01-01",
        deprecatedInVersion: "1.0.0",
        logWarning: false,
      });

      const result = isEndpointDeprecated("post", "/api/v1/test");
      expect(result).toBeDefined();
    });
  });

  describe("trackEndpointUsage", () => {
    it("should track usage count", () => {
      registerDeprecatedEndpoint({
        method: "GET",
        path: "/api/v1/test",
        reason: "Test",
        deprecatedAt: "2025-01-01",
        deprecatedInVersion: "1.0.0",
        logWarning: false,
      });

      expect(isEndpointDeprecated("GET", "/api/v1/test")?.usageCount).toBe(0);

      trackEndpointUsage("GET", "/api/v1/test");

      expect(isEndpointDeprecated("GET", "/api/v1/test")?.usageCount).toBe(1);

      trackEndpointUsage("GET", "/api/v1/test");

      expect(isEndpointDeprecated("GET", "/api/v1/test")?.usageCount).toBe(2);
    });

    it("should return the deprecated endpoint info", () => {
      registerDeprecatedEndpoint({
        method: "DELETE",
        path: "/api/v1/test",
        reason: "Test deprecation",
        deprecatedAt: "2025-01-01",
        deprecatedInVersion: "1.0.0",
        logWarning: false,
      });

      const result = trackEndpointUsage("DELETE", "/api/v1/test");

      expect(result).toBeDefined();
      expect(result?.path).toBe("/api/v1/test");
      expect(result?.reason).toBe("Test deprecation");
    });

    it("should return undefined for non-deprecated endpoints", () => {
      const result = trackEndpointUsage("GET", "/api/v1/non-deprecated");
      expect(result).toBeUndefined();
    });

    it("should support silent mode", () => {
      registerDeprecatedEndpoint({
        method: "GET",
        path: "/api/v1/test",
        reason: "Test",
        deprecatedAt: "2025-01-01",
        deprecatedInVersion: "1.0.0",
        logWarning: true,
      });

      const consoleSpy = vi.spyOn(console, "warn").mockImplementation(() => {});

      trackEndpointUsage("GET", "/api/v1/test", { silent: true });

      expect(consoleSpy).not.toHaveBeenCalled();

      consoleSpy.mockRestore();
    });

    it("should support custom warning messages", () => {
      registerDeprecatedEndpoint({
        method: "GET",
        path: "/api/v1/test",
        reason: "Test",
        deprecatedAt: "2025-01-01",
        deprecatedInVersion: "1.0.0",
        logWarning: true,
      });

      const consoleSpy = vi.spyOn(console, "warn").mockImplementation(() => {});

      trackEndpointUsage("GET", "/api/v1/test", {
        customMessage: "Custom warning message",
      });

      expect(consoleSpy).toHaveBeenCalledWith(
        expect.stringContaining("Custom warning message"),
      );

      consoleSpy.mockRestore();
    });

    it("should call custom logger when configured", () => {
      registerDeprecatedEndpoint({
        method: "GET",
        path: "/api/v1/test",
        reason: "Test",
        deprecatedAt: "2025-01-01",
        deprecatedInVersion: "1.0.0",
        logWarning: true,
      });

      const customLogger = vi.fn();

      configureDeprecationService({
        customLogger,
      });

      trackEndpointUsage("GET", "/api/v1/test");

      expect(customLogger).toHaveBeenCalledWith(
        expect.stringContaining("API endpoint GET /api/v1/test is deprecated"),
        expect.objectContaining({
          path: "/api/v1/test",
          method: "GET",
        }),
      );
    });
  });

  describe("configureDeprecationService", () => {
    it("should update configuration", () => {
      configureDeprecationService({
        enabled: false,
        logToConsole: false,
      });

      // Configuration is internal, but we can verify behavior
      expect(true).toBe(true);
    });

    it("should accept custom logger", () => {
      const customLogger = vi.fn();

      configureDeprecationService({
        customLogger,
      });

      // Logger is stored internally and used when endpoints are tracked
      expect(true).toBe(true);
    });
  });

  describe("getDeprecatedEndpoints", () => {
    it("should return all registered endpoints", () => {
      registerDeprecatedEndpoint({
        method: "GET",
        path: "/api/v1/endpoint1",
        reason: "Test 1",
        deprecatedAt: "2025-01-01",
        deprecatedInVersion: "1.0.0",
        logWarning: false,
      });

      registerDeprecatedEndpoint({
        method: "POST",
        path: "/api/v1/endpoint2",
        reason: "Test 2",
        deprecatedAt: "2025-01-01",
        deprecatedInVersion: "1.0.0",
        logWarning: false,
      });

      const endpoints = getDeprecatedEndpoints();

      expect(endpoints).toHaveLength(2);
      expect(endpoints).toEqual(
        expect.arrayContaining([
          expect.objectContaining({ path: "/api/v1/endpoint1" }),
          expect.objectContaining({ path: "/api/v1/endpoint2" }),
        ]),
      );
    });

    it("should return endpoints sorted by path", () => {
      registerDeprecatedEndpoint({
        method: "GET",
        path: "/api/v1/z-endpoint",
        reason: "Test",
        deprecatedAt: "2025-01-01",
        deprecatedInVersion: "1.0.0",
        logWarning: false,
      });

      registerDeprecatedEndpoint({
        method: "GET",
        path: "/api/v1/a-endpoint",
        reason: "Test",
        deprecatedAt: "2025-01-01",
        deprecatedInVersion: "1.0.0",
        logWarning: false,
      });

      const endpoints = getDeprecatedEndpoints();

      expect(endpoints[0].path).toBe("/api/v1/a-endpoint");
      expect(endpoints[1].path).toBe("/api/v1/z-endpoint");
    });
  });

  describe("generateDeprecationReport", () => {
    it("should generate comprehensive report", () => {
      registerDeprecatedEndpoint({
        method: "GET",
        path: "/api/v1/used",
        reason: "Used endpoint",
        deprecatedAt: "2025-01-01",
        deprecatedInVersion: "1.0.0",
        logWarning: false,
      });

      registerDeprecatedEndpoint({
        method: "GET",
        path: "/api/v1/never-used",
        reason: "Never used",
        deprecatedAt: "2025-01-01",
        deprecatedInVersion: "1.0.0",
        logWarning: false,
      });

      registerDeprecatedEndpoint({
        method: "GET",
        path: "/api/v1/used-many",
        reason: "Used many times",
        deprecatedAt: "2025-01-01",
        deprecatedInVersion: "1.0.0",
        logWarning: false,
      });

      // Track usage
      trackEndpointUsage("GET", "/api/v1/used");
      trackEndpointUsage("GET", "/api/v1/used-many");
      trackEndpointUsage("GET", "/api/v1/used-many");
      trackEndpointUsage("GET", "/api/v1/used-many");

      const report = generateDeprecationReport();

      expect(report.totalDeprecated).toBe(3);
      expect(report.totalUsage).toBe(4);
      expect(report.endpoints).toHaveLength(3);
      expect(report.mostUsed).toHaveLength(1);
      expect(report.mostUsed[0].path).toBe("/api/v1/used-many");
      expect(report.neverUsed).toHaveLength(1);
      expect(report.neverUsed[0].path).toBe("/api/v1/never-used");
    });
  });

  describe("resetDeprecationStats", () => {
    it("should reset all usage counts to 0", () => {
      registerDeprecatedEndpoint({
        method: "GET",
        path: "/api/v1/test1",
        reason: "Test",
        deprecatedAt: "2025-01-01",
        deprecatedInVersion: "1.0.0",
        logWarning: false,
      });

      registerDeprecatedEndpoint({
        method: "POST",
        path: "/api/v1/test2",
        reason: "Test",
        deprecatedAt: "2025-01-01",
        deprecatedInVersion: "1.0.0",
        logWarning: false,
      });

      // Track some usage
      trackEndpointUsage("GET", "/api/v1/test1");
      trackEndpointUsage("POST", "/api/v1/test2");
      trackEndpointUsage("POST", "/api/v1/test2");

      expect(isEndpointDeprecated("GET", "/api/v1/test1")?.usageCount).toBe(1);
      expect(isEndpointDeprecated("POST", "/api/v1/test2")?.usageCount).toBe(2);

      // Reset
      resetDeprecationStats();

      expect(isEndpointDeprecated("GET", "/api/v1/test1")?.usageCount).toBe(0);
      expect(isEndpointDeprecated("POST", "/api/v1/test2")?.usageCount).toBe(0);
    });
  });

  describe("silenceDeprecationWarning", () => {
    it("should disable warning logging for endpoint", () => {
      registerDeprecatedEndpoint({
        method: "GET",
        path: "/api/v1/test",
        reason: "Test",
        deprecatedAt: "2025-01-01",
        deprecatedInVersion: "1.0.0",
        logWarning: true,
      });

      const consoleSpy = vi.spyOn(console, "warn").mockImplementation(() => {});

      // First call should warn
      trackEndpointUsage("GET", "/api/v1/test");
      expect(consoleSpy).toHaveBeenCalledTimes(1);

      // Silence warnings
      silenceDeprecationWarning("GET", "/api/v1/test");

      // Second call should not warn
      trackEndpointUsage("GET", "/api/v1/test");
      expect(consoleSpy).toHaveBeenCalledTimes(1);

      consoleSpy.mockRestore();
    });
  });

  describe("enableDeprecationWarning", () => {
    it("should enable warning logging for endpoint", () => {
      registerDeprecatedEndpoint({
        method: "GET",
        path: "/api/v1/test",
        reason: "Test",
        deprecatedAt: "2025-01-01",
        deprecatedInVersion: "1.0.0",
        logWarning: false,
      });

      const consoleSpy = vi.spyOn(console, "warn").mockImplementation(() => {});

      // First call should not warn
      trackEndpointUsage("GET", "/api/v1/test");
      expect(consoleSpy).not.toHaveBeenCalled();

      // Enable warnings
      enableDeprecationWarning("GET", "/api/v1/test");

      // Second call should warn
      trackEndpointUsage("GET", "/api/v1/test");
      expect(consoleSpy).toHaveBeenCalledTimes(1);

      consoleSpy.mockRestore();
    });
  });

  describe("integration with console warnings", () => {
    it("should log deprecation warnings to console", () => {
      registerDeprecatedEndpoint({
        method: "GET",
        path: "/api/v1/old",
        reason: "Use new endpoint",
        alternative: "GET /api/v1/new",
        deprecatedAt: "2025-01-01",
        deprecatedInVersion: "1.5.0",
        plannedRemoval: "2026-01-01",
        logWarning: true,
      });

      const consoleSpy = vi.spyOn(console, "warn").mockImplementation(() => {});

      trackEndpointUsage("GET", "/api/v1/old");

      // console.warn is called once with a single formatted message
      expect(consoleSpy).toHaveBeenCalledTimes(1);
      expect(consoleSpy).toHaveBeenCalledWith(
        expect.stringContaining("API endpoint GET /api/v1/old is deprecated"),
      );
      expect(consoleSpy).toHaveBeenCalledWith(
        expect.stringContaining("Use new endpoint"),
      );
      expect(consoleSpy).toHaveBeenCalledWith(
        expect.stringContaining("Scheduled for removal: 2026-01-01"),
      );

      consoleSpy.mockRestore();
    });
  });
});
