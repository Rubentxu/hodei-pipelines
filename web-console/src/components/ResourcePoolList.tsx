import React, { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  getResourcePools,
  getResourcePoolStatus,
  type ResourcePool,
  type ResourcePoolStatus,
  getCapacityUtilization,
  isAtCriticalCapacity,
  getPoolHealthStatus,
} from '../services/resourcePoolApi';

export function ResourcePoolList() {
  const [pools, setPools] = useState<ResourcePool[]>([]);
  const [poolStatuses, setPoolStatuses] = useState<Record<string, ResourcePoolStatus>>({});
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [refreshInterval, setRefreshInterval] = useState<number | null>(null);
  const navigate = useNavigate();

  useEffect(() => {
    loadPools();
  }, []);

  useEffect(() => {
    // Auto-refresh every 30 seconds
    const interval = setInterval(() => {
      loadPoolStatuses();
    }, 30000);
    setRefreshInterval(interval);

    return () => {
      if (interval) clearInterval(interval);
    };
  }, [pools]);

  const loadPools = async () => {
    try {
      setLoading(true);
      const data = await getResourcePools();
      setPools(data);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load resource pools');
    } finally {
      setLoading(false);
    }
  };

  const loadPoolStatuses = async () => {
    try {
      const statusPromises = pools.map(async (pool) => {
        try {
          const status = await getResourcePoolStatus(pool.id);
          return { poolId: pool.id, status };
        } catch {
          return { poolId: pool.id, status: null };
        }
      });

      const results = await Promise.all(statusPromises);
      const statusMap: Record<string, ResourcePoolStatus> = {};
      results.forEach(({ poolId, status }) => {
        if (status) {
          statusMap[poolId] = status;
        }
      });
      setPoolStatuses(statusMap);
    } catch (err) {
      console.error('Failed to load pool statuses:', err);
    }
  };

  const handleCreatePool = () => {
    navigate('/resource-pools/new');
  };

  const handleViewPool = (poolId: string) => {
    navigate(`/resource-pools/${poolId}`);
  };

  const getHealthBadgeClass = (status: ResourcePoolStatus) => {
    const health = getPoolHealthStatus(status);
    switch (health) {
      case 'healthy':
        return 'bg-green-100 text-green-800 border-green-300';
      case 'warning':
        return 'bg-yellow-100 text-yellow-800 border-yellow-300';
      case 'critical':
        return 'bg-red-100 text-red-800 border-red-300';
      default:
        return 'bg-gray-100 text-gray-800 border-gray-300';
    }
  };

  const getUtilizationColor = (status: ResourcePoolStatus) => {
    const utilization = getCapacityUtilization(status);
    if (utilization >= 95) return 'text-red-600';
    if (utilization >= 80) return 'text-yellow-600';
    return 'text-green-600';
  };

  if (loading && pools.length === 0) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500"></div>
      </div>
    );
  }

  if (error && pools.length === 0) {
    return (
      <div className="bg-red-50 border border-red-200 rounded-md p-4">
        <h3 className="text-sm font-medium text-red-800">Error loading resource pools</h3>
        <p className="mt-1 text-sm text-red-600">{error}</p>
        <button
          onClick={loadPools}
          className="mt-3 inline-flex items-center px-3 py-2 border border-transparent text-sm leading-4 font-medium rounded-md text-white bg-red-600 hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-red-500"
        >
          Retry
        </button>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Resource Pools</h1>
          <p className="mt-1 text-sm text-gray-500">
            Manage and monitor your compute resource pools
          </p>
        </div>
        <button
          onClick={handleCreatePool}
          className="inline-flex items-center px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
        >
          <svg
            className="-ml-1 mr-2 h-5 w-5"
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 20 20"
            fill="currentColor"
          >
            <path
              fillRule="evenodd"
              d="M10 5a1 1 0 011 1v3h3a1 1 0 110 2h-3v3a1 1 0 11-2 0v-3H6a1 1 0 110-2h3V6a1 1 0 011-1z"
              clipRule="evenodd"
            />
          </svg>
          Create Pool
        </button>
      </div>

      {error && (
        <div className="bg-yellow-50 border border-yellow-200 rounded-md p-4">
          <p className="text-sm text-yellow-800">{error}</p>
        </div>
      )}

      <div className="bg-white shadow overflow-hidden sm:rounded-md">
        <ul className="divide-y divide-gray-200">
          {pools.map((pool) => {
            const status = poolStatuses[pool.id];
            return (
              <li key={pool.id} className="hover:bg-gray-50">
                <div className="px-4 py-4 sm:px-6">
                  <div className="flex items-center justify-between">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center space-x-3">
                        <h3 className="text-lg font-medium text-gray-900 truncate">
                          {pool.name}
                        </h3>
                        {status && (
                          <span
                            className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border ${getHealthBadgeClass(
                              status
                            )}`}
                          >
                            {getPoolHealthStatus(status)}
                          </span>
                        )}
                        <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-blue-100 text-blue-800">
                          {pool.poolType}
                        </span>
                      </div>
                      <div className="mt-2 flex items-center text-sm text-gray-500 space-x-6">
                        <div>
                          <span className="font-medium">Provider:</span> {pool.providerName}
                        </div>
                        <div>
                          <span className="font-medium">Size:</span> {pool.minSize} - {pool.maxSize}
                        </div>
                        {status && (
                          <>
                            <div>
                              <span className="font-medium">Capacity:</span>{' '}
                              <span className={getUtilizationColor(status)}>
                                {getCapacityUtilization(status)}%
                              </span>
                            </div>
                            <div>
                              <span className="font-medium">Workers:</span> {status.activeWorkers}
                            </div>
                            <div>
                              <span className="font-medium">Available:</span>{' '}
                              {status.availableCapacity}/{status.totalCapacity}
                            </div>
                          </>
                        )}
                      </div>
                      {Object.keys(pool.tags).length > 0 && (
                        <div className="mt-2 flex flex-wrap gap-2">
                          {Object.entries(pool.tags).map(([key, value]) => (
                            <span
                              key={key}
                              className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-gray-100 text-gray-800"
                            >
                              {key}: {value}
                            </span>
                          ))}
                        </div>
                      )}
                    </div>
                    <div className="flex items-center space-x-2">
                      {isAtCriticalCapacity(status!) && (
                        <svg
                          className="h-5 w-5 text-red-500"
                          xmlns="http://www.w3.org/2000/svg"
                          viewBox="0 0 20 20"
                          fill="currentColor"
                        >
                          <path
                            fillRule="evenodd"
                            d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z"
                            clipRule="evenodd"
                          />
                        </svg>
                      )}
                      <button
                        onClick={() => handleViewPool(pool.id)}
                        className="text-blue-600 hover:text-blue-900 text-sm font-medium"
                      >
                        View Details
                      </button>
                    </div>
                  </div>
                </div>
              </li>
            );
          })}
        </ul>
      </div>

      {pools.length === 0 && !loading && (
        <div className="text-center py-12 bg-white rounded-md shadow">
          <svg
            className="mx-auto h-12 w-12 text-gray-400"
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"
            />
          </svg>
          <h3 className="mt-2 text-sm font-medium text-gray-900">No resource pools</h3>
          <p className="mt-1 text-sm text-gray-500">
            Get started by creating a new resource pool.
          </p>
          <div className="mt-6">
            <button
              onClick={handleCreatePool}
              className="inline-flex items-center px-4 py-2 border border-transparent shadow-sm text-sm font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
            >
              <svg
                className="-ml-1 mr-2 h-5 w-5"
                xmlns="http://www.w3.org/2000/svg"
                viewBox="0 0 20 20"
                fill="currentColor"
              >
                <path
                  fillRule="evenodd"
                  d="M10 5a1 1 0 011 1v3h3a1 1 0 110 2h-3v3a1 1 0 11-2 0v-3H6a1 1 0 110-2h3V6a1 1 0 011-1z"
                  clipRule="evenodd"
                />
              </svg>
              Create Pool
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
