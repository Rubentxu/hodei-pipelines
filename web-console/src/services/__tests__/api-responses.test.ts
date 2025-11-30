/**
 * @jest-environment jsdom
 */

import { describe, it, expect } from 'vitest';
import {
  PaginatedResponse,
  SingleItemResponse,
  SuccessResponse,
  ErrorResponse,
  StandardApiResponse,
  isPaginatedResponse,
  isSingleItemResponse,
  isErrorResponse,
  isSuccessResponse,
} from '../../types/api-responses';

describe('API Response Types', () => {
  describe('PaginatedResponse', () => {
    it('should validate paginated response structure', () => {
      const response: PaginatedResponse<string> = {
        items: ['item1', 'item2', 'item3'],
        total: 3,
        hasMore: false,
      };

      expect(response.items).toHaveLength(3);
      expect(response.total).toBe(3);
      expect(response.hasMore).toBe(false);
    });

    it('should handle empty paginated response', () => {
      const response: PaginatedResponse<string> = {
        items: [],
        total: 0,
        hasMore: false,
      };

      expect(response.items).toHaveLength(0);
      expect(response.total).toBe(0);
      expect(response.hasMore).toBe(false);
    });

    it('should indicate when more items are available', () => {
      const response: PaginatedResponse<string> = {
        items: ['item1', 'item2'],
        total: 50,
        hasMore: true,
      };

      expect(response.items).toHaveLength(2);
      expect(response.total).toBe(50);
      expect(response.hasMore).toBe(true);
    });
  });

  describe('SingleItemResponse', () => {
    it('should validate single item response structure', () => {
      const response: SingleItemResponse<{ id: string; name: string }> = {
        item: {
          id: '123',
          name: 'Test Item',
        },
      };

      expect(response.item).toBeDefined();
      expect(response.item.id).toBe('123');
      expect(response.item.name).toBe('Test Item');
    });

    it('should handle different item types', () => {
      const response: SingleItemResponse<number> = {
        item: 42,
      };

      expect(response.item).toBe(42);
    });
  });

  describe('SuccessResponse', () => {
    it('should validate success response with data', () => {
      const response: SuccessResponse<{ message: string }> = {
        success: true,
        message: 'Operation completed',
        data: { message: 'Success' },
      };

      expect(response.success).toBe(true);
      expect(response.message).toBe('Operation completed');
      expect(response.data?.message).toBe('Success');
    });

    it('should validate success response without data', () => {
      const response: SuccessResponse<void> = {
        success: true,
        message: 'Operation completed',
      };

      expect(response.success).toBe(true);
      expect(response.message).toBe('Operation completed');
      expect(response.data).toBeUndefined();
    });

    it('should use default success value', () => {
      const response: SuccessResponse = {
        success: true,
      };

      expect(response.success).toBe(true);
    });
  });

  describe('ErrorResponse', () => {
    it('should validate error response structure', () => {
      const response: ErrorResponse = {
        success: false,
        code: 'VALIDATION_ERROR',
        message: 'Invalid input data',
        details: {
          field: 'email',
          reason: 'Invalid format',
        },
        timestamp: '2025-11-30T10:00:00Z',
      };

      expect(response.success).toBe(false);
      expect(response.code).toBe('VALIDATION_ERROR');
      expect(response.message).toBe('Invalid input data');
      expect(response.details?.field).toBe('email');
      expect(response.timestamp).toBeDefined();
    });

    it('should handle error with minimal fields', () => {
      const response: ErrorResponse = {
        success: false,
        code: 'NOT_FOUND',
        message: 'Resource not found',
        timestamp: '2025-11-30T10:00:00Z',
      };

      expect(response.success).toBe(false);
      expect(response.code).toBe('NOT_FOUND');
      expect(response.message).toBe('Resource not found');
      expect(response.details).toBeUndefined();
    });
  });

  describe('Type Guards', () => {
    describe('isPaginatedResponse', () => {
      it('should identify paginated responses', () => {
        const response = {
          items: [1, 2, 3],
          total: 3,
          hasMore: false,
        };

        expect(isPaginatedResponse(response)).toBe(true);
      });

      it('should reject non-paginated responses', () => {
        const response1 = {
          item: 'single',
        };

        const response2 = {
          success: true,
        };

        const response3 = {
          items: [1, 2],
        };

        expect(isPaginatedResponse(response1)).toBe(false);
        expect(isPaginatedResponse(response2)).toBe(false);
        expect(isPaginatedResponse(response3)).toBe(false);
      });
    });

    describe('isSingleItemResponse', () => {
      it('should identify single item responses', () => {
        const response = {
          item: 'single item',
        };

        expect(isSingleItemResponse(response)).toBe(true);
      });

      it('should reject responses with items array', () => {
        const response = {
          items: ['item1'],
          total: 1,
        };

        expect(isSingleItemResponse(response)).toBe(false);
      });
    });

    describe('isErrorResponse', () => {
      it('should identify error responses', () => {
        const response = {
          success: false,
          code: 'ERROR',
          message: 'Something went wrong',
          timestamp: '2025-11-30T10:00:00Z',
        };

        expect(isErrorResponse(response)).toBe(true);
      });

      it('should reject success responses', () => {
        const response = {
          success: true,
        };

        expect(isErrorResponse(response)).toBe(false);
      });
    });

    describe('isSuccessResponse', () => {
      it('should identify success responses', () => {
        const response = {
          success: true,
          message: 'OK',
        };

        expect(isSuccessResponse(response)).toBe(true);
      });

      it('should reject error responses', () => {
        const response = {
          success: false,
          code: 'ERROR',
          message: 'Failed',
        };

        expect(isSuccessResponse(response)).toBe(false);
      });
    });
  });

  describe('StandardApiResponse', () => {
    it('should handle standard response format with data', () => {
      const response: StandardApiResponse<{ id: string }> = {
        success: true,
        data: { id: '123' },
      };

      expect(response.success).toBe(true);
      expect(response.data?.id).toBe('123');
    });

    it('should handle standard response format with error', () => {
      const response: StandardApiResponse = {
        success: false,
        error: {
          code: 'BAD_REQUEST',
          message: 'Invalid request',
          timestamp: '2025-11-30T10:00:00Z',
        },
      };

      expect(response.success).toBe(false);
      expect(response.error?.code).toBe('BAD_REQUEST');
      expect(response.data).toBeUndefined();
    });

    it('should handle standard response format with metadata', () => {
      const response: StandardApiResponse<string[]> = {
        success: true,
        data: ['item1', 'item2'],
        meta: {
          pagination: {
            page: 0,
            limit: 10,
            total: 25,
            totalPages: 3,
          },
        },
      };

      expect(response.success).toBe(true);
      expect(response.meta?.pagination?.page).toBe(0);
      expect(response.meta?.pagination?.total).toBe(25);
    });
  });

  describe('ResponseMetadata', () => {
    it('should handle pagination metadata', () => {
      const meta = {
        pagination: {
          page: 1,
          limit: 20,
          total: 100,
          totalPages: 5,
        },
      };

      expect(meta.pagination.page).toBe(1);
      expect(meta.pagination.limit).toBe(20);
      expect(meta.pagination.total).toBe(100);
      expect(meta.pagination.totalPages).toBe(5);
    });

    it('should handle additional metadata', () => {
      const meta = {
        pagination: {
          page: 0,
          limit: 50,
        },
        customField: 'custom value',
        anotherField: 123,
      };

      expect(meta.customField).toBe('custom value');
      expect(meta.anotherField).toBe(123);
    });
  });
});
