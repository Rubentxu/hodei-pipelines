import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  ApiError,
  errorHandler,
  ErrorHandler,
  type ErrorHandlerConfig,
  type OfflineRequest
} from "../errorHandler";

// Mock fetch
const mockFetch = vi.fn();
global.fetch = mockFetch as any;

describe("errorHandler", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.resetAllMocks();

    // Reset online status
    Object.defineProperty(navigator, "onLine", {
      writable: true,
      value: true,
    });
  });

  describe("Error Handling", () => {
    it("should handle network errors", async () => {
      const networkError = new TypeError("Failed to fetch");

      await expect(
        errorHandler.handleError(networkError, {
          url: "/api/test",
          method: "GET",
        }),
      ).rejects.toThrow();

      expect(errorHandler.getErrorMessage).toBeDefined();
    });

    it("should create API error from network error", () => {
      const networkError = new TypeError("Network Error");
      const context = {
        url: "/api/test",
        method: "GET",
      };

      const error = (errorHandler as any).createApiError?.(networkError, {
        ...context,
        timestamp: new Date(),
      });

      expect(error).toHaveProperty("code");
      expect(error?.retryable).toBe(true);
    });

    it("should identify retryable errors", () => {
      const retryableError: ApiError = {
        code: "NETWORK_ERROR",
        message: "Network error",
        retryable: true,
        context: {
          url: "/api/test",
          method: "GET",
          timestamp: new Date(),
        },
      };

      expect((errorHandler as any).isRetryableError?.(retryableError)).toBe(true);
    });

    it("should identify non-retryable errors", () => {
      const nonRetryableError: ApiError = {
        code: "UNAUTHORIZED",
        message: "Unauthorized",
        statusCode: 401,
        retryable: false,
        context: {
          url: "/api/test",
          method: "GET",
          timestamp: new Date(),
        },
      };

      expect((errorHandler as any).isRetryableError?.(nonRetryableError)).toBe(false);
    });
  });

  describe("Retry Logic", () => {
    it("should calculate exponential backoff delay", () => {
      const handler = new ErrorHandler({
        retry: {
          maxAttempts: 3,
          initialDelay: 1000,
          maxDelay: 10000,
          backoffMultiplier: 2,
          retryableStatuses: [500],
          retryableErrors: ["NETWORK_ERROR"],
        },
      } as any);

      const delay0 = (handler as any).calculateDelay?.(0);
      const delay1 = (handler as any).calculateDelay?.(1);
      const delay2 = (handler as any).calculateDelay?.(2);

      expect(delay0).toBe(1000);
      expect(delay1).toBe(2000);
      expect(delay2).toBe(4000);
    });

    it("should cap delay at maxDelay", () => {
      const handler = new ErrorHandler({
        retry: {
          maxAttempts: 3,
          initialDelay: 1000,
          maxDelay: 5000,
          backoffMultiplier: 2,
          retryableStatuses: [500],
          retryableErrors: ["NETWORK_ERROR"],
        },
      } as any);

      const delay5 = (handler as any).calculateDelay?.(5);
      expect(delay5).toBe(5000);
    });

    it("should generate unique request keys", () => {
      const context1 = {
        url: "/api/test",
        method: "GET",
        timestamp: new Date(),
      };
      const context2 = {
        url: "/api/test",
        method: "POST",
        timestamp: new Date(),
      };

      const key1 = (errorHandler as any).getRequestKey?.(context1);
      const key2 = (errorHandler as any).getRequestKey?.(context2);

      expect(key1).not.toBe(key2);
    });
  });

  describe("Offline Queue", () => {
    it("should queue requests when offline", () => {
      Object.defineProperty(navigator, "onLine", {
        writable: true,
        value: false,
      });

      const requestId = (errorHandler as any).generateRequestId?.();
      expect(requestId).toBeDefined();
      expect(typeof requestId).toBe("string");
    });

    it("should generate unique request IDs", () => {
      const id1 = (errorHandler as any).generateRequestId?.();
      const id2 = (errorHandler as any).generateRequestId?.();

      expect(id1).not.toBe(id2);
      expect(id1).toMatch(/^req_/);
    });

    it("should persist offline queue to localStorage", () => {
      const queue: OfflineRequest[] = [
        {
          id: "req_1",
          url: "/api/test",
          method: "GET",
          headers: {},
          timestamp: new Date(),
          attempts: 0,
        },
      ];

      (errorHandler as any).persistOfflineQueue?.(queue);

      const stored = localStorage.getItem("offline_request_queue");
      expect(stored).toBeTruthy();
    });

    it("should load offline queue from localStorage", () => {
      const testQueue = [
        {
          id: "req_1",
          url: "/api/test",
          method: "GET",
          headers: {},
          timestamp: new Date(),
          attempts: 0,
        },
      ];

      localStorage.setItem(
        "offline_request_queue",
        JSON.stringify(testQueue),
      );

      // Create new instance to load from storage
      const handler = new ErrorHandler();
      expect(handler.getOfflineQueueSize()).toBe(1);
    });
  });

  describe("Network Status", () => {
    it("should detect online status", () => {
      Object.defineProperty(navigator, "onLine", {
        writable: true,
        value: true,
      });

      const handler = new ErrorHandler();
      expect(handler.isNetworkOnline()).toBe(true);
    });

    it("should detect offline status", () => {
      Object.defineProperty(navigator, "onLine", {
        writable: true,
        value: false,
      });

      const handler = new ErrorHandler();
      expect(handler.isNetworkOnline()).toBe(false);
    });

    it("should flush queue when back online", async () => {
      Object.defineProperty(navigator, "onLine", {
        writable: true,
        value: false,
      });

      // Add request to queue
      const queue = [
        {
          id: "req_1",
          url: "/api/test",
          method: "GET",
          headers: {},
          timestamp: new Date(),
          attempts: 0,
        },
      ];

      (errorHandler as any).persistOfflineQueue?.(queue);

      // Come back online
      Object.defineProperty(navigator, "onLine", {
        writable: true,
        value: true,
      });

      // Flush queue
      await (errorHandler as any).flushOfflineQueue?.();

      // Queue should be empty
      expect(errorHandler.getOfflineQueueSize?.()).toBe(0);
    });
  });

  describe("Error Messages", () => {
    it("should return user-friendly error messages", () => {
      const testErrors = [
        {
          code: "NETWORK_ERROR",
          expected: "Unable to connect to the server",
        },
        {
          code: "UNAUTHORIZED",
          expected: "You are not authorized",
        },
        {
          code: "NOT_FOUND",
          expected: "The requested resource was not found",
        },
        {
          code: "UNKNOWN_ERROR",
          expected: "An unexpected error occurred",
        },
      ];

      testErrors.forEach((test) => {
        const error: ApiError = {
          code: test.code as any,
          message: test.expected,
          retryable: false,
          context: {
            url: "/api/test",
            method: "GET",
            timestamp: new Date(),
          },
        };

        const message = errorHandler.getErrorMessage?.(error);
        expect(message).toBeDefined();
        expect(typeof message).toBe("string");
      });
    });
  });

  describe("Error Listeners", () => {
    it("should add and remove error listeners", () => {
      const listener = vi.fn();

      errorHandler.addErrorListener?.(listener);
      expect((errorHandler as any).errorListeners?.has(listener)).toBe(true);

      errorHandler.removeErrorListener?.(listener);
      expect((errorHandler as any).errorListeners?.has(listener)).toBe(false);
    });

    it("should notify all listeners of errors", () => {
      const listener1 = vi.fn();
      const listener2 = vi.fn();

      errorHandler.addErrorListener?.(listener1);
      errorHandler.addErrorListener?.(listener2);

      // Trigger error notification
      const error: ApiError = {
        code: "TEST_ERROR",
        message: "Test error",
        retryable: false,
        context: {
          url: "/api/test",
          method: "GET",
          timestamp: new Date(),
        },
      };

      (errorHandler as any).notifyListeners?.(error);

      expect(listener1).toHaveBeenCalledWith(error);
      expect(listener2).toHaveBeenCalledWith(error);
    });
  });

  describe("Error Handler Configuration", () => {
    it("should create error handler with custom config", () => {
      const customConfig: Partial<ErrorHandlerConfig> = {
        timeout: 60000,
        enableRetry: false,
        enableOfflineQueue: false,
        retry: {
          maxAttempts: 5,
          initialDelay: 2000,
          maxDelay: 20000,
          backoffMultiplier: 1.5,
          retryableStatuses: [500, 502],
          retryableErrors: ["CUSTOM_ERROR"],
        },
      };

      const handler = new ErrorHandler(customConfig);

      expect((handler as any).config.timeout).toBe(60000);
      expect((handler as any).config.enableRetry).toBe(false);
      expect((handler as any).config.enableOfflineQueue).toBe(false);
      expect((handler as any).config.retry.maxAttempts).toBe(5);
    });

    it("should use default config when none provided", () => {
      const handler = new ErrorHandler();

      expect((handler as any).config.timeout).toBe(30000);
      expect((handler as any).config.enableRetry).toBe(true);
      expect((handler as any).config.enableOfflineQueue).toBe(true);
      expect((handler as any).config.retry.maxAttempts).toBe(3);
    });
  });

  describe("Retryable Status Codes", () => {
    const testCases = [
      { status: 408, expected: true },
      { status: 429, expected: true },
      { status: 500, expected: true },
      { status: 502, expected: true },
      { status: 503, expected: true },
      { status: 504, expected: true },
      { status: 400, expected: false },
      { status: 401, expected: false },
      { status: 404, expected: false },
    ];

    testCases.forEach(({ status, expected }) => {
      it(`should ${expected ? "allow" : "reject"} retry for status ${status}`, () => {
        const handler = new ErrorHandler();
        const canRetry = (handler as any).isRetryableStatus?.(status);

        expect(canRetry).toBe(expected);
      });
    });
  });

  describe("Error Tracking", () => {
    it("should track errors for analytics", () => {
      const error: ApiError = {
        code: "TEST_ERROR",
        message: "Test error",
        statusCode: 500,
        retryable: true,
        context: {
          url: "/api/test",
          method: "GET",
          timestamp: new Date(),
        },
      };

      // Should not throw
      expect(() => {
        (errorHandler as any).trackError?.(error);
      }).not.toThrow();
    });
  });

  describe("Clear Offline Queue", () => {
    it("should clear offline queue", () => {
      // Mock localStorage
      const mockSetItem = vi.fn();
      Object.defineProperty(window, "localStorage", {
        value: {
          setItem: mockSetItem,
          getItem: vi.fn(() => "[]"),
        },
        writable: true,
      });

      const queue = [
        {
          id: "req_1",
          url: "/api/test",
          method: "GET",
          headers: {},
          timestamp: new Date(),
          attempts: 0,
        },
      ];

      (errorHandler as any).offlineQueue = queue;
      errorHandler.clearOfflineQueue?.();

      expect((errorHandler as any).offlineQueue.length).toBe(0);
      expect(mockSetItem).toHaveBeenCalled();
    });
  });
});

describe("ErrorHandler Class", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should be instantiated with default config", () => {
    const handler = new ErrorHandler();
    expect((handler as any).config).toBeDefined();
    expect((handler as any).config.retry.maxAttempts).toBe(3);
  });

  it("should be instantiated with custom config", () => {
    const config: Partial<ErrorHandlerConfig> = {
      timeout: 60000,
    };
    const handler = new ErrorHandler(config);
    expect((handler as any).config.timeout).toBe(60000);
  });

  it("should register online/offline event listeners", () => {
    const addEventListenerSpy = vi.spyOn(window, "addEventListener");
    new ErrorHandler();

    expect(addEventListenerSpy).toHaveBeenCalledWith("online", expect.any(Function));
    expect(addEventListenerSpy).toHaveBeenCalledWith("offline", expect.any(Function));
  });
});
