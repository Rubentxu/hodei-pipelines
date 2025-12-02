/**
 * Centralized Error Handler for API Requests
 * Provides error interception, retry logic, offline queue, and user-friendly messages
 */

export interface ErrorContext {
  timestamp: Date;
  url: string;
  method: string;
  statusCode?: number;
  requestId?: string;
  userId?: string;
  sessionId?: string;
}

export interface ApiError {
  code: string;
  message: string;
  details?: Record<string, any>;
  statusCode?: number;
  retryable: boolean;
  context: ErrorContext;
}

export interface RetryConfig {
  maxAttempts: number;
  initialDelay: number;
  maxDelay: number;
  backoffMultiplier: number;
  retryableStatuses: number[];
  retryableErrors: string[];
}

export interface OfflineRequest {
  id: string;
  url: string;
  method: string;
  headers: Record<string, string>;
  body?: any;
  timestamp: Date;
  attempts: number;
}

export interface ErrorHandlerConfig {
  retry: RetryConfig;
  offlineQueue: {
    enabled: boolean;
    maxSize: number;
    persistInterval: number;
  };
  timeout: number;
  enableRetry: boolean;
  enableOfflineQueue: boolean;
  enableErrorTracking: boolean;
}

export class ErrorHandler {
  private config: ErrorHandlerConfig;
  private retryQueue: Map<string, Promise<any>> = new Map();
  private offlineQueue: OfflineRequest[] = [];
  private errorListeners: Set<(error: ApiError) => void> = new Set();
  private isOnline: boolean = navigator.onLine;

  constructor(config: Partial<ErrorHandlerConfig> = {}) {
    this.config = {
      retry: {
        maxAttempts: 3,
        initialDelay: 1000,
        maxDelay: 10000,
        backoffMultiplier: 2,
        retryableStatuses: [408, 429, 500, 502, 503, 504],
        retryableErrors: [
          "NETWORK_ERROR",
          "TIMEOUT",
          "CONNECTION_LOST",
        ],
      },
      offlineQueue: {
        enabled: true,
        maxSize: 100,
        persistInterval: 5000,
      },
      timeout: 30000,
      enableRetry: true,
      enableOfflineQueue: true,
      enableErrorTracking: true,
      ...config,
    };

    // Listen for online/offline events
    window.addEventListener("online", this.handleOnline.bind(this));
    window.addEventListener("offline", this.handleOffline.bind(this));

    // Load offline queue from storage
    this.loadOfflineQueue();

    // Periodic flush of offline queue
    setInterval(() => {
      this.flushOfflineQueue();
    }, this.config.offlineQueue.persistInterval);
  }

  /**
   * Handle fetch error and determine retry logic
   */
  async handleError(
    error: Error | TypeError,
    context: Omit<ErrorContext, "timestamp">,
  ): Promise<never> {
    const errorContext: ErrorContext = {
      ...context,
      timestamp: new Date(),
    };

    const apiError = this.createApiError(error, errorContext);

    // Track error if enabled
    if (this.config.enableErrorTracking) {
      this.trackError(apiError);
    }

    // Notify listeners
    this.notifyListeners(apiError);

    // Handle network errors
    if (this.isNetworkError(error)) {
      if (this.config.enableOfflineQueue && !this.isOnline) {
        // Queue the request for later
        throw this.queueRequest(context as any);
      }

      if (this.config.enableRetry && this.isRetryableError(apiError)) {
        // Retry the request
        throw this.retryRequest(error, errorContext);
      }
    }

    // Don't retry other errors
    throw apiError;
  }

  /**
   * Create standardized API error
   */
  private createApiError(
    error: Error | TypeError,
    context: ErrorContext,
  ): ApiError {
    let code = "UNKNOWN_ERROR";
    let message = error.message || "An unexpected error occurred";
    let statusCode: number | undefined;
    let retryable = false;

    // Handle different error types
    if (error instanceof TypeError) {
      code = "NETWORK_ERROR";
      message = "Network connection error";
      retryable = true;
    } else if ("status" in error && typeof (error as any).status === "number") {
      statusCode = (error as any).status;
      code = this.getErrorCode(statusCode);
      retryable = this.isRetryableStatus(statusCode);
    }

    // Extract error details if available
    let details: Record<string, any> | undefined;
    if ((error as any).response?.data) {
      details = (error as any).response.data;
    }

    return {
      code,
      message,
      details,
      statusCode,
      retryable,
      context,
    };
  }

  /**
   * Get error code from HTTP status
   */
  private getErrorCode(statusCode: number): string {
    if (statusCode >= 400 && statusCode < 500) {
      switch (statusCode) {
        case 400:
          return "BAD_REQUEST";
        case 401:
          return "UNAUTHORIZED";
        case 403:
          return "FORBIDDEN";
        case 404:
          return "NOT_FOUND";
        case 409:
          return "CONFLICT";
        case 422:
          return "VALIDATION_ERROR";
        case 429:
          return "TOO_MANY_REQUESTS";
        default:
          return "CLIENT_ERROR";
      }
    } else if (statusCode >= 500 && statusCode < 600) {
      return "SERVER_ERROR";
    }
    return "UNKNOWN_ERROR";
  }

  /**
   * Check if error is retryable
   */
  private isRetryableError(error: ApiError): boolean {
    if (error.statusCode && this.isRetryableStatus(error.statusCode)) {
      return true;
    }

    if (this.config.retry.retryableErrors.includes(error.code)) {
      return true;
    }

    return false;
  }

  /**
   * Check if status code is retryable
   */
  private isRetryableStatus(statusCode: number): boolean {
    return this.config.retry.retryableStatuses.includes(statusCode);
  }

  /**
   * Check if error is a network error
   */
  private isNetworkError(error: Error | TypeError): boolean {
    return (
      error instanceof TypeError ||
      error.message.includes("Network Error") ||
      error.message.includes("Failed to fetch")
    );
  }

  /**
   * Retry a failed request
   */
  private async retryRequest(
    error: Error | TypeError,
    context: ErrorContext,
  ): Promise<never> {
    const requestKey = this.getRequestKey(context);

    // Check if already retrying
    if (this.retryQueue.has(requestKey)) {
      throw new Error("Request already in retry queue");
    }

    // Calculate delay with exponential backoff
    const delay = this.calculateDelay(0);

    // Create retry promise
    const retryPromise = new Promise<never>((resolve, reject) => {
      setTimeout(async () => {
        try {
          // Attempt to retry
          await this.attemptRetry(context);
          resolve() as any;
        } catch (err) {
          reject(err);
        }
      }, delay);
    });

    this.retryQueue.set(requestKey, retryPromise);

    try {
      await retryPromise;
    } finally {
      this.retryQueue.delete(requestKey);
    }

    throw error;
  }

  /**
   * Attempt to retry a request
   */
  private async attemptRetry(context: ErrorContext): Promise<void> {
    // Implementation would depend on the request context
    // This is a placeholder for the retry logic
    return Promise.resolve();
  }

  /**
   * Calculate delay for exponential backoff
   */
  private calculateDelay(attempt: number): number {
    const delay = this.config.retry.initialDelay *
      Math.pow(this.config.retry.backoffMultiplier, attempt);

    return Math.min(delay, this.config.retry.maxDelay);
  }

  /**
   * Get unique key for request
   */
  private getRequestKey(context: ErrorContext): string {
    return `${context.method}:${context.url}`;
  }

  /**
   * Queue request for offline retry
   */
  private queueRequest(context: any): Error {
    const request: OfflineRequest = {
      id: this.generateRequestId(),
      url: context.url,
      method: context.method,
      headers: context.headers || {},
      body: context.body,
      timestamp: new Date(),
      attempts: 0,
    };

    // Add to queue
    this.offlineQueue.push(request);

    // Limit queue size
    if (this.offlineQueue.length > this.config.offlineQueue.maxSize) {
      this.offlineQueue.shift();
    }

    // Persist to storage
    this.persistOfflineQueue();

    return {
      code: "OFFLINE_QUEUE",
      message: "Request queued for retry when online",
      retryable: false,
    } as ApiError;
  }

  /**
   * Generate unique request ID
   */
  private generateRequestId(): string {
    return `req_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }

  /**
   * Persist offline queue to localStorage
   */
  private persistOfflineQueue(): void {
    try {
      localStorage.setItem(
        "offline_request_queue",
        JSON.stringify(this.offlineQueue),
      );
    } catch (error) {
      console.error("Failed to persist offline queue:", error);
    }
  }

  /**
   * Load offline queue from localStorage
   */
  private loadOfflineQueue(): void {
    try {
      const stored = localStorage.getItem("offline_request_queue");
      if (stored) {
        this.offlineQueue = JSON.parse(stored);
      }
    } catch (error) {
      console.error("Failed to load offline queue:", error);
      this.offlineQueue = [];
    }
  }

  /**
   * Flush offline queue when back online
   */
  private async flushOfflineQueue(): Promise<void> {
    if (!this.isOnline || this.offlineQueue.length === 0) {
      return;
    }

    const requests = [...this.offlineQueue];
    this.offlineQueue = [];

    for (const request of requests) {
      try {
        await this.replayRequest(request);
      } catch (error) {
        console.error("Failed to replay offline request:", error);
      }
    }

    this.persistOfflineQueue();
  }

  /**
   * Replay an offline request
   */
  private async replayRequest(request: OfflineRequest): Promise<void> {
    // Implementation would replay the request
    // This is a placeholder
    return Promise.resolve();
  }

  /**
   * Handle online event
   */
  private handleOnline(): void {
    this.isOnline = true;
    this.flushOfflineQueue();
  }

  /**
   * Handle offline event
   */
  private handleOffline(): void {
    this.isOnline = false;
  }

  /**
   * Add error listener
   */
  addErrorListener(listener: (error: ApiError) => void): void {
    this.errorListeners.add(listener);
  }

  /**
   * Remove error listener
   */
  removeErrorListener(listener: (error: ApiError) => void): void {
    this.errorListeners.delete(listener);
  }

  /**
   * Notify all error listeners
   */
  private notifyListeners(error: ApiError): void {
    this.errorListeners.forEach((listener) => {
      try {
        listener(error);
      } catch (err) {
        console.error("Error in error listener:", err);
      }
    });
  }

  /**
   * Track error for analytics
   */
  private trackError(error: ApiError): void {
    // Track to analytics service
    if (typeof window !== "undefined" && (window as any).gtag) {
      (window as any).gtag("event", "api_error", {
        error_code: error.code,
        status_code: error.statusCode,
        url: error.context.url,
        retryable: error.retryable,
      });
    }

    // Log to console in development
    if (process.env.NODE_ENV === "development") {
      console.error("API Error:", error);
    }
  }

  /**
   * Get user-friendly error message
   */
  getErrorMessage(error: ApiError): string {
    const errorMessages: Record<string, string> = {
      NETWORK_ERROR: "Unable to connect to the server. Please check your internet connection.",
      TIMEOUT: "The request is taking too long. Please try again.",
      UNAUTHORIZED: "You are not authorized to perform this action. Please log in.",
      FORBIDDEN: "You don't have permission to access this resource.",
      NOT_FOUND: "The requested resource was not found.",
      SERVER_ERROR: "The server encountered an error. Please try again later.",
      BAD_REQUEST: "Invalid request. Please check your input.",
      OFFLINE_QUEUE: "You're currently offline. Your request will be processed when you're back online.",
    };

    return errorMessages[error.code] || error.message;
  }

  /**
   * Check if online
   */
  isNetworkOnline(): boolean {
    return this.isOnline;
  }

  /**
   * Get offline queue size
   */
  getOfflineQueueSize(): number {
    return this.offlineQueue.length;
  }

  /**
   * Clear offline queue
   */
  clearOfflineQueue(): void {
    this.offlineQueue = [];
    this.persistOfflineQueue();
  }
}

// Export singleton instance
export const errorHandler = new ErrorHandler();

// Export types

