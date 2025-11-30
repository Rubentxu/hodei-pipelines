/**
 * Standard API Response Types
 *
 * This module defines standardized response types for all API operations
 * to ensure consistency across the application.
 */

/**
 * Standard format for paginated list responses
 */
export interface PaginatedResponse<T> {
  /** Array of items */
  items: T[];
  /** Total number of items available */
  total: number;
  /** Whether there are more items to fetch */
  hasMore: boolean;
}

/**
 * Standard format for single item responses
 */
export interface SingleItemResponse<T> {
  /** The requested item */
  item: T;
}

/**
 * Standard format for success responses with optional data
 */
export interface SuccessResponse<T = void> {
  /** Always true for successful responses */
  success: true;
  /** Optional message */
  message?: string;
  /** Response data (optional for simple success responses) */
  data?: T;
}

/**
 * Standard format for error responses
 */
export interface ErrorResponse {
  /** Always false for error responses */
  success: false;
  /** Error code (e.g., "NOT_FOUND", "VALIDATION_ERROR") */
  code: string;
  /** Human-readable error message */
  message: string;
  /** Additional error details */
  details?: Record<string, unknown>;
  /** When the error occurred */
  timestamp: string;
}

/**
 * Union type for all API responses
 */
export type ApiResponse<T> =
  | PaginatedResponse<T>
  | SingleItemResponse<T>
  | SuccessResponse<T>
  | ErrorResponse;

/**
 * Response metadata for pagination
 */
export interface PaginationMeta {
  /** Current page number (0-indexed) */
  page: number;
  /** Number of items per page */
  limit: number;
  /** Total number of items (when available) */
  total?: number;
  /** Total number of pages (when total is available) */
  totalPages?: number;
}

/**
 * Extended metadata for responses
 */
export interface ResponseMeta {
  /** Pagination information */
  pagination?: PaginationMeta;
  /** Additional metadata key-value pairs */
  [key: string]: unknown;
}

/**
 * Extended paginated response with metadata
 */
export interface PaginatedResponseWithMeta<T> extends PaginatedResponse<T> {
  /** Additional response metadata */
  meta?: ResponseMeta;
}

/**
 * Standard API response wrapper
 */
export interface StandardApiResponse<T = unknown> {
  /** Response status */
  success: boolean;
  /** Response data */
  data?: T;
  /** Error information (when success is false) */
  error?: {
    code: string;
    message: string;
    details?: Record<string, unknown>;
    timestamp: string;
  };
  /** Response metadata */
  meta?: ResponseMeta;
}

/**
 * Utility type guard to check if response is paginated
 */
export function isPaginatedResponse<T>(response: ApiResponse<T>): response is PaginatedResponse<T> {
  return (
    typeof response === 'object' &&
    response !== null &&
    'items' in response &&
    'total' in response &&
    'hasMore' in response
  );
}

/**
 * Utility type guard to check if response is a single item
 */
export function isSingleItemResponse<T>(response: ApiResponse<T>): response is SingleItemResponse<T> {
  return (
    typeof response === 'object' &&
    response !== null &&
    'item' in response &&
    !('items' in response)
  );
}

/**
 * Utility type guard to check if response is an error
 */
export function isErrorResponse(response: ApiResponse<unknown>): response is ErrorResponse {
  return (
    typeof response === 'object' &&
    response !== null &&
    'success' in response &&
    response.success === false
  );
}

/**
 * Utility type guard to check if response is successful
 */
export function isSuccessResponse<T>(response: ApiResponse<T>): response is SuccessResponse<T> {
  return (
    typeof response === 'object' &&
    response !== null &&
    'success' in response &&
    response.success === true
  );
}
