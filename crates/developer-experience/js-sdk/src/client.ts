/**
 * HTTP client for API communication
 */
import { ClientConfig, SdkError } from './types';

/**
 * HTTP client for making API requests
 */
export class HttpClient {
  private baseUrl: string;
  private apiToken: string;
  private timeout: number;
  private maxRetries: number;

  constructor(config: ClientConfig) {
    this.baseUrl = config.baseUrl;
    this.apiToken = config.apiToken;
    this.timeout = config.timeout || 30000;
    this.maxRetries = config.maxRetries || 3;
  }

  /**
   * Make a GET request
   */
  async get<T>(path: string): Promise<T> {
    return this.request<T>('GET', path);
  }

  /**
   * Make a POST request
   */
  async post<T, B = unknown>(path: string, body?: B): Promise<T> {
    return this.request<T>('POST', path, body);
  }

  /**
   * Make a PUT request
   */
  async put<T, B = unknown>(path: string, body: B): Promise<T> {
    return this.request<T>('PUT', path, body);
  }

  /**
   * Make a DELETE request
   */
  async delete<T>(path: string): Promise<T> {
    return this.request<T>('DELETE', path);
  }

  /**
   * Make a generic HTTP request
   */
  private async request<T>(
    method: string,
    path: string,
    body?: unknown
  ): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    const headers: Record<string, string> = {
      Authorization: `Bearer ${this.apiToken}`,
    };

    if (body !== undefined) {
      headers['Content-Type'] = 'application/json';
    }

    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.timeout);

    try {
      const response = await fetch(url, {
        method,
        headers,
        body: body ? JSON.stringify(body) : undefined,
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      return await this.handleResponse<T>(response);
    } catch (error) {
      clearTimeout(timeoutId);

      if (error instanceof Error) {
        if (error.name === 'AbortError') {
          throw new SdkError('Request timeout');
        }
        throw new SdkError(`Request failed: ${error.message}`, undefined, error);
      }
      throw error;
    }
  }

  /**
   * Handle HTTP response
   */
  private async handleResponse<T>(response: Response): Promise<T> {
    if (response.ok) {
      // Check if response has content
      const contentType = response.headers.get('content-type');
      if (contentType && contentType.includes('application/json')) {
        return await response.json();
      }
      // For non-JSON responses (like empty 204), return empty object
      return {} as T;
    }

    // Handle error responses
    const statusCode = response.status;
    let errorMessage: string;

    try {
      const errorBody = await response.text();
      errorMessage = errorBody || response.statusText;
    } catch {
      errorMessage = response.statusText;
    }

    switch (statusCode) {
      case 401:
      case 403:
        throw new SdkError(`Authentication failed: ${errorMessage}`, statusCode);
      case 404:
        throw new SdkError(`Resource not found: ${errorMessage}`, statusCode);
      default:
        throw new SdkError(`API error: ${errorMessage}`, statusCode);
    }
  }
}
