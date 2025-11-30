import { useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import {
  deleteResourcePool,
  getCapacityUtilization,
  getPoolHealthStatus,
  getResourcePool,
  getResourcePoolStatus,
  isAtCriticalCapacity,
  type ResourcePool,
  type ResourcePoolStatus,
} from '../services/resourcePoolApi';

export function ResourcePoolDetails() {
  const navigate = useNavigate();
  const { id } = useParams();
  const [pool, setPool] = useState<ResourcePool | null>(null);
  const [status, setStatus] = useState<ResourcePoolStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showDeleteModal, setShowDeleteModal] = useState(false);
  const [deleting, setDeleting] = useState(false);

  useEffect(() => {
    if (!id) {
      navigate('/resource-pools');
      return;
    }
    loadPoolData();
  }, [id]);

  const loadPoolData = async () => {
    if (!id) return;

    try {
      setLoading(true);
      const [poolData, statusData] = await Promise.all([
        getResourcePool(id),
        getResourcePoolStatus(id).catch(() => null),
      ]);
      setPool(poolData);
      setStatus(statusData);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load pool data');
    } finally {
      setLoading(false);
    }
  };

  const handleDelete = async () => {
    if (!id || !pool) return;

    try {
      setDeleting(true);
      await deleteResourcePool(id);
      navigate('/resource-pools');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to delete pool');
      setShowDeleteModal(false);
    } finally {
      setDeleting(false);
    }
  };

  const getUtilizationBar = (percentage: number) => {
    const clamped = Math.min(100, Math.max(0, percentage));
    let color = 'bg-green-500';
    if (clamped >= 90) color = 'bg-red-500';
    else if (clamped >= 70) color = 'bg-yellow-500';

    return (
      <div className="w-full bg-gray-200 rounded-full h-2">
        <div
          className={`h-2 rounded-full ${color}`}
          style={{ width: `${clamped}%` }}
        />
      </div>
    );
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

  const getPoolTypeIcon = (type: string) => {
    switch (type) {
      case 'Docker':
        return (
          <svg className="h-6 w-6" viewBox="0 0 24 24" fill="currentColor">
            <path d="M13.5 2H10.5C9.39543 2 8.5 2.89543 8.5 4V7H13.5V2ZM14.5 8H9.5V20C9.5 21.1046 10.3954 22 11.5 22H12.5C13.6046 22 14.5 21.1046 14.5 20V8ZM17 7H16C16 5.89543 16.8954 5 18 5H19V18C19 19.1046 18.1046 20 17 20V7ZM6 9H5C3.89543 9 3 9.89543 3 11V20C3 21.1046 3.89543 22 5 22H6C7.10457 22 8 21.1046 8 20V11C8 9.89543 7.10457 9 6 9Z" />
          </svg>
        );
      case 'Kubernetes':
        return (
          <svg className="h-6 w-6" viewBox="0 0 24 24" fill="currentColor">
            <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 17.93c-3.94-.49-7-3.85-7-7.93 0-.62.08-1.21.21-1.79L9 15v1c0 1.1.9 2 2 2v1.93zm6.9-2.54c-.26-.81-1-1.39-1.9-1.39h-1v-3c0-.55-.45-1-1-1H8v-2h2c.55 0 1-.45 1-1V7h2c1.1 0 2-.9 2-2v-.41c2.93 1.19 5 4.06 5 7.41 0 2.08-.8 3.97-2.1 5.39z" />
          </svg>
        );
      default:
        return (
          <svg className="h-6 w-6" viewBox="0 0 24 24" fill="currentColor">
            <path d="M20 6h-2.18c.11-.31.18-.65.18-1a2.996 2.996 0 0 0-5.5-1.65l-.5.67-.5-.68C10.96 2.54 10.05 2 9 2 7.34 2 6 3.34 6 5c0 .35.07.69.18 1H4c-1.11 0-1.99.89-1.99 2L2 19c0 1.11.89 2 2 2h16c1.11 0 2-.89 2-2V8c0-1.11-.89-2-2-2zm-5-2c.55 0 1 .45 1 1s-.45 1-1 1-1-.45-1-1 .45-1 1-1zM9 4c.55 0 1 .45 1 1s-.45 1-1 1-1-.45-1-1 .45-1 1-1z" />
          </svg>
        );
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64" role="status" aria-label="Loading">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500"></div>
      </div>
    );
  }

  if (error || !pool) {
    return (
      <div className="bg-red-50 border border-red-200 rounded-md p-4">
        <h3 className="text-sm font-medium text-red-800">Error loading pool</h3>
        <p className="mt-1 text-sm text-red-600">{error || 'Pool not found'}</p>
        <button
          onClick={() => navigate('/resource-pools')}
          className="mt-3 inline-flex items-center px-3 py-2 border border-transparent text-sm leading-4 font-medium rounded-md text-white bg-red-600 hover:bg-red-700"
        >
          Back to Pools
        </button>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-start">
        <div className="flex items-center space-x-4">
          <button
            onClick={() => navigate('/resource-pools')}
            className="inline-flex items-center text-sm text-gray-500 hover:text-gray-700"
          >
            <svg
              className="mr-2 h-5 w-5"
              xmlns="http://www.w3.org/2000/svg"
              viewBox="0 0 20 20"
              fill="currentColor"
            >
              <path
                fillRule="evenodd"
                d="M9.707 16.707a1 1 0 01-1.414 0l-6-6a1 1 0 010-1.414l6-6a1 1 0 011.414 1.414L5.414 9H17a1 1 0 110 2H5.414l4.293 4.293a1 1 0 010 1.414z"
                clipRule="evenodd"
              />
            </svg>
            Back
          </button>
          <div className="flex items-center space-x-3">
            <div className="text-blue-500">{getPoolTypeIcon(pool.poolType)}</div>
            <div>
              <h1 className="text-2xl font-bold text-gray-900">{pool.name}</h1>
              <p className="text-sm text-gray-500">{pool.providerName} • {pool.poolType}</p>
            </div>
          </div>
        </div>
        <div className="flex space-x-3">
          <button
            onClick={() => navigate(`/resource-pools/${id}/edit`)}
            className="inline-flex items-center px-4 py-2 border border-gray-300 rounded-md shadow-sm text-sm font-medium text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
          >
            Edit Pool
          </button>
          <button
            onClick={() => setShowDeleteModal(true)}
            className="inline-flex items-center px-4 py-2 border border-red-300 rounded-md shadow-sm text-sm font-medium text-red-700 bg-white hover:bg-red-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-red-500"
          >
            Delete Pool
          </button>
        </div>
      </div>

      {error && (
        <div className="bg-yellow-50 border border-yellow-200 rounded-md p-4">
          <p className="text-sm text-yellow-800">{error}</p>
        </div>
      )}

      {status && (
        <div className="grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-4">
          <div className="bg-white overflow-hidden shadow rounded-lg">
            <div className="p-5">
              <div className="flex items-center">
                <div className="flex-1">
                  <dt className="text-sm font-medium text-gray-500 truncate">Capacity Utilization</dt>
                  <dd className="mt-1 text-3xl font-semibold text-gray-900">
                    {getCapacityUtilization(status)}%
                  </dd>
                </div>
              </div>
              <div className="mt-4">
                {getUtilizationBar(getCapacityUtilization(status))}
              </div>
            </div>
          </div>

          <div className="bg-white overflow-hidden shadow rounded-lg">
            <div className="p-5">
              <dt className="text-sm font-medium text-gray-500 truncate">Active Workers</dt>
              <dd className="mt-1 text-3xl font-semibold text-gray-900">{status.activeWorkers}</dd>
              <p className="mt-1 text-sm text-gray-500">of {status.totalCapacity} total</p>
            </div>
          </div>

          <div className="bg-white overflow-hidden shadow rounded-lg">
            <div className="p-5">
              <dt className="text-sm font-medium text-gray-500 truncate">Available Capacity</dt>
              <dd className="mt-1 text-3xl font-semibold text-gray-900">{status.availableCapacity}</dd>
              <p className="mt-1 text-sm text-gray-500">workers ready</p>
            </div>
          </div>

          <div className="bg-white overflow-hidden shadow rounded-lg">
            <div className="p-5">
              <dt className="text-sm font-medium text-gray-500 truncate">Health Status</dt>
              <dd className="mt-1">
                <span
                  className={`inline-flex items-center px-3 py-1 rounded-full text-sm font-medium border ${getHealthBadgeClass(
                    status
                  )}`}
                >
                  {getPoolHealthStatus(status)}
                </span>
              </dd>
              {isAtCriticalCapacity(status) && (
                <p className="mt-2 text-sm text-red-600 font-medium">
                  ⚠️ Critical capacity threshold reached
                </p>
              )}
            </div>
          </div>
        </div>
      )}

      <div className="bg-white shadow rounded-lg overflow-hidden">
        <div className="px-4 py-5 sm:p-6">
          <h3 className="text-lg font-medium text-gray-900 mb-4">Pool Configuration</h3>
          <dl className="grid grid-cols-1 gap-x-4 gap-y-6 sm:grid-cols-2">
            <div>
              <dt className="text-sm font-medium text-gray-500">Pool Type</dt>
              <dd className="mt-1 text-sm text-gray-900">{pool.poolType}</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-gray-500">Provider</dt>
              <dd className="mt-1 text-sm text-gray-900">{pool.providerName}</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-gray-500">Minimum Size</dt>
              <dd className="mt-1 text-sm text-gray-900">{pool.minSize} workers</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-gray-500">Maximum Size</dt>
              <dd className="mt-1 text-sm text-gray-900">{pool.maxSize} workers</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-gray-500">Default CPU</dt>
              <dd className="mt-1 text-sm text-gray-900">{pool.defaultResources.cpu_m} millicores</dd>
            </div>
            <div>
              <dt className="text-sm font-medium text-gray-500">Default Memory</dt>
              <dd className="mt-1 text-sm text-gray-900">{pool.defaultResources.memory_mb} MB</dd>
            </div>
            {pool.defaultResources.gpu !== null && (
              <div>
                <dt className="text-sm font-medium text-gray-500">Default GPU</dt>
                <dd className="mt-1 text-sm text-gray-900">{pool.defaultResources.gpu} GPU</dd>
              </div>
            )}
          </dl>

          {Object.keys(pool.tags).length > 0 && (
            <div className="mt-6">
              <dt className="text-sm font-medium text-gray-500 mb-2">Tags</dt>
              <div className="flex flex-wrap gap-2">
                {Object.entries(pool.tags).map(([key, value]) => (
                  <span
                    key={key}
                    className="inline-flex items-center px-3 py-1 rounded-full text-sm font-medium bg-blue-100 text-blue-800"
                  >
                    {key}: {value}
                  </span>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>

      {status && (
        <div className="bg-white shadow rounded-lg overflow-hidden">
          <div className="px-4 py-5 sm:p-6">
            <h3 className="text-lg font-medium text-gray-900 mb-4">Resource Status</h3>
            <div className="space-y-4">
              <div>
                <div className="flex justify-between text-sm text-gray-600 mb-1">
                  <span>Total Capacity</span>
                  <span>{status.totalCapacity} workers</span>
                </div>
                {getUtilizationBar(getCapacityUtilization(status))}
                <div className="flex justify-between text-xs text-gray-500 mt-1">
                  <span>Active: {status.activeWorkers}</span>
                  <span>Available: {status.availableCapacity}</span>
                </div>
              </div>
              <div>
                <dt className="text-sm font-medium text-gray-500">Pending Requests</dt>
                <dd className="mt-1 text-sm text-gray-900">
                  {status.pendingRequests === 0 ? (
                    'No pending requests'
                  ) : (
                    <span className="text-orange-600 font-medium">
                      {status.pendingRequests} job{status.pendingRequests > 1 ? 's' : ''} queued
                    </span>
                  )}
                </dd>
              </div>
            </div>
          </div>
        </div>
      )}

      {showDeleteModal && (
        <div className="fixed inset-0 bg-gray-500 bg-opacity-75 flex items-center justify-center z-50">
          <div className="bg-white rounded-lg p-6 max-w-md w-full">
            <h3 className="text-lg font-medium text-gray-900 mb-4">Delete Resource Pool</h3>
            <p className="text-sm text-gray-500 mb-6">
              Are you sure you want to delete the pool "{pool.name}"? This action cannot be undone.
            </p>
            <div className="flex justify-end space-x-3">
              <button
                onClick={() => setShowDeleteModal(false)}
                disabled={deleting}
                className="px-4 py-2 border border-gray-300 rounded-md shadow-sm text-sm font-medium text-gray-700 bg-white hover:bg-gray-50 disabled:opacity-50"
              >
                Cancel
              </button>
              <button
                onClick={handleDelete}
                disabled={deleting}
                className="px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-red-600 hover:bg-red-700 disabled:opacity-50"
              >
                {deleting ? 'Deleting...' : 'Delete Pool'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
