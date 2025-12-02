import { ResourceQuota } from '@/types';

// Types matching backend API
export type ResourcePoolType = 'Docker' | 'Kubernetes' | 'Cloud' | 'Static';

export interface ResourcePool {
  id: string;
  name: string;
  poolType: ResourcePoolType;
  providerName: string;
  minSize: number;
  maxSize: number;
  defaultResources: ResourceQuota;
  tags: Record<string, string>;
}

export interface CreateResourcePoolRequest {
  poolType: ResourcePoolType;
  name: string;
  providerName: string;
  minSize: number;
  maxSize: number;
  defaultResources: ResourceQuota;
  tags?: Record<string, string>;
}

export interface UpdateResourcePoolRequest {
  name?: string;
  minSize?: number;
  maxSize?: number;
  tags?: Record<string, string>;
}

export interface ResourcePoolStatus {
  name: string;
  poolType: ResourcePoolType;
  totalCapacity: number;
  availableCapacity: number;
  activeWorkers: number;
  pendingRequests: number;
}

/**
 * Fetch all resource pools from the API
 * @returns Promise containing array of ResourcePool
 * @throws Error if the fetch operation fails
 */
export async function getResourcePools(): Promise<ResourcePool[]> {
  const response = await fetch('/api/v1/worker-pools');

  if (!response.ok) {
    throw new Error('Failed to fetch resource pools');
  }

  return response.json();
}

/**
 * Fetch a single resource pool by ID
 * @param id - The pool ID to fetch
 * @returns Promise containing ResourcePool
 * @throws Error if the pool is not found or fetch fails
 */
export async function getResourcePool(id: string): Promise<ResourcePool> {
  const response = await fetch(`/api/v1/worker-pools/${id}`);

  if (!response.ok) {
    throw new Error('Failed to fetch resource pool');
  }

  return response.json();
}

/**
 * Create a new resource pool
 * @param data - The pool configuration
 * @returns Promise containing the created ResourcePool
 * @throws Error if validation fails or creation fails
 */
export async function createResourcePool(
  data: CreateResourcePoolRequest
): Promise<ResourcePool> {
  // Client-side validation
  if (data.minSize > data.maxSize) {
    throw new Error('min_size cannot be greater than max_size');
  }

  const response = await fetch('/api/v1/worker-pools', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(data),
  });

  if (!response.ok) {
    let errorMessage = 'Failed to create resource pool';
    try {
      const errorData = await response.json();
      if (errorData.message) {
        errorMessage = errorData.message;
      }
    } catch {
      // Ignore JSON parse error
    }
    throw new Error(errorMessage);
  }

  return response.json();
}

/**
 * Update an existing resource pool
 * @param id - The pool ID to update
 * @param data - The update data
 * @returns Promise containing the updated ResourcePool
 * @throws Error if validation fails or update fails
 */
export async function updateResourcePool(
  id: string,
  data: UpdateResourcePoolRequest
): Promise<ResourcePool> {
  // Client-side validation
  if (data.minSize !== undefined && data.maxSize !== undefined) {
    if (data.minSize > data.maxSize) {
      throw new Error('min_size cannot be greater than max_size');
    }
  }

  const response = await fetch(`/api/v1/worker-pools/${id}`, {
    method: 'PATCH',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(data),
  });

  if (!response.ok) {
    throw new Error('Failed to update resource pool');
  }

  return response.json();
}

/**
 * Delete a resource pool
 * @param id - The pool ID to delete
 * @returns Promise that resolves when deletion is complete
 * @throws Error if the pool is not found or deletion fails
 */
export async function deleteResourcePool(id: string): Promise<void> {
  const response = await fetch(`/api/v1/worker-pools/${id}`, {
    method: 'DELETE',
  });

  if (!response.ok) {
    throw new Error('Failed to delete resource pool');
  }
}

/**
 * Fetch the status of a specific resource pool
 * @param id - The pool ID
 * @returns Promise containing ResourcePoolStatus
 * @throws Error if status is not found or fetch fails
 */
export async function getResourcePoolStatus(id: string): Promise<ResourcePoolStatus> {
  const response = await fetch(`/api/v1/worker-pools/${id}/status`);

  if (!response.ok) {
    throw new Error('Failed to fetch pool status');
  }

  return response.json();
}

/**
 * Get capacity utilization percentage for a pool
 * @param status - The pool status
 * @returns Percentage of capacity used (0-100)
 */
export function getCapacityUtilization(status: ResourcePoolStatus): number {
  if (status.totalCapacity === 0) return 0;
  const used = status.totalCapacity - status.availableCapacity;
  return Math.round((used / status.totalCapacity) * 100);
}

/**
 * Check if a pool is at critical capacity (>95%)
 * @param status - The pool status
 * @returns True if pool is at critical capacity
 */
export function isAtCriticalCapacity(status: ResourcePoolStatus): boolean {
  return getCapacityUtilization(status) >= 95;
}

/**
 * Get pool health status based on available capacity and pending requests
 * @param status - The pool status
 * @returns Health status: 'healthy' | 'warning' | 'critical'
 */
export function getPoolHealthStatus(status: ResourcePoolStatus): 'healthy' | 'warning' | 'critical' {
  const utilization = getCapacityUtilization(status);

  if (utilization >= 95 || status.pendingRequests > 10) {
    return 'critical';
  } else if (utilization >= 80 || status.pendingRequests > 5) {
    return 'warning';
  } else {
    return 'healthy';
  }
}
