import { useQuery } from '@tanstack/react-query';

const API_BASE = '/api/developer';

export interface ApiEndpoint {
  id: string;
  method: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';
  path: string;
  description: string;
  parameters?: Array<{
    name: string;
    type: string;
    required: boolean;
    description: string;
  }>;
  requestBody?: {
    type: string;
    schema: any;
  };
  responses: Array<{
    status: number;
    description: string;
    schema: any;
  }>;
  examples?: Array<{
    name: string;
    request: any;
    response: any;
  }>;
}

export interface CodeExample {
  id: string;
  title: string;
  description: string;
  language: 'curl' | 'javascript' | 'typescript' | 'python' | 'go' | 'java';
  code: string;
  endpoint?: string;
}

export const developerApi = {
  async getApiDocumentation(): Promise<{
    title: string;
    version: string;
    description: string;
    endpoints: ApiEndpoint[];
  }> {
    const response = await fetch(`${API_BASE}/openapi`);

    if (!response.ok) {
      throw new Error('Failed to fetch API documentation');
    }

    return response.json();
  },

  async getCodeExamples(): Promise<CodeExample[]> {
    const response = await fetch(`${API_BASE}/examples`);

    if (!response.ok) {
      throw new Error('Failed to fetch code examples');
    }

    return response.json();
  },

  async testEndpoint(params: {
    method: string;
    path: string;
    headers?: Record<string, string>;
    body?: any;
  }): Promise<any> {
    const response = await fetch(`${API_BASE}/test`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(params),
    });

    if (!response.ok) {
      throw new Error('Failed to test endpoint');
    }

    return response.json();
  },

  async getSdkDownloads(): Promise<{
    language: string;
    version: string;
    downloadUrl: string;
    documentationUrl: string;
  }[]> {
    const response = await fetch(`${API_BASE}/sdks`);

    if (!response.ok) {
      throw new Error('Failed to fetch SDK downloads');
    }

    return response.json();
  },

  async getTutorials(): Promise<Array<{
    id: string;
    title: string;
    description: string;
    difficulty: 'beginner' | 'intermediate' | 'advanced';
    duration: string;
    content: string;
  }>> {
    const response = await fetch(`${API_BASE}/tutorials`);

    if (!response.ok) {
      throw new Error('Failed to fetch tutorials');
    }

    return response.json();
  },
};
