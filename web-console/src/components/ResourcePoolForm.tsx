import React, { useState, useEffect } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import {
  createResourcePool,
  updateResourcePool,
  getResourcePool,
  type ResourcePool,
  type CreateResourcePoolRequest,
  type UpdateResourcePoolRequest,
  type ResourcePoolType,
} from '../services/resourcePoolApi';

interface FormData {
  poolType: ResourcePoolType;
  name: string;
  providerName: string;
  minSize: number;
  maxSize: number;
  cpu_m: number;
  memory_mb: number;
  gpu: number | null;
  tags: Record<string, string>;
}

export function ResourcePoolForm() {
  const navigate = useNavigate();
  const { id } = useParams();
  const isEditing = !!id;

  const [formData, setFormData] = useState<FormData>({
    poolType: 'Docker',
    name: '',
    providerName: 'docker',
    minSize: 1,
    maxSize: 10,
    cpu_m: 2000,
    memory_mb: 4096,
    gpu: null,
    tags: {},
  });
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [tagKey, setTagKey] = useState('');
  const [tagValue, setTagValue] = useState('');

  useEffect(() => {
    if (isEditing) {
      loadPool();
    }
  }, [id]);

  const loadPool = async () => {
    if (!id) return;

    try {
      setLoading(true);
      const pool = await getResourcePool(id);
      setFormData({
        poolType: pool.poolType,
        name: pool.name,
        providerName: pool.providerName,
        minSize: pool.minSize,
        maxSize: pool.maxSize,
        cpu_m: pool.defaultResources.cpu_m,
        memory_mb: pool.defaultResources.memory_mb,
        gpu: pool.defaultResources.gpu,
        tags: pool.tags,
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load pool');
    } finally {
      setLoading(false);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);

    // Validate
    if (!formData.name.trim()) {
      setError('Pool name is required');
      return;
    }
    if (formData.minSize > formData.maxSize) {
      setError('Minimum size cannot be greater than maximum size');
      return;
    }
    if (formData.minSize < 0 || formData.maxSize < 0) {
      setError('Pool size must be non-negative');
      return;
    }

    try {
      setSaving(true);

      const defaultResources = {
        cpu_m: formData.cpu_m,
        memory_mb: formData.memory_mb,
        gpu: formData.gpu,
      };

      if (isEditing) {
        const updateData: UpdateResourcePoolRequest = {
          name: formData.name,
          minSize: formData.minSize,
          maxSize: formData.maxSize,
          tags: Object.keys(formData.tags).length > 0 ? formData.tags : undefined,
        };
        await updateResourcePool(id!, updateData);
      } else {
        const createData: CreateResourcePoolRequest = {
          poolType: formData.poolType,
          name: formData.name,
          providerName: formData.providerName,
          minSize: formData.minSize,
          maxSize: formData.maxSize,
          defaultResources,
          tags: Object.keys(formData.tags).length > 0 ? formData.tags : undefined,
        };
        await createResourcePool(createData);
      }

      navigate('/resource-pools');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to save pool');
    } finally {
      setSaving(false);
    }
  };

  const handleChange = (
    e: React.ChangeEvent<
      HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement
    >
  ) => {
    const { name, value } = e.target;
    setFormData((prev) => ({
      ...prev,
      [name]:
        name === 'minSize' || name === 'maxSize' || name === 'cpu_m' || name === 'memory_mb'
          ? parseInt(value) || 0
          : name === 'gpu'
          ? value === ''
            ? null
            : parseInt(value)
          : value,
    }));
  };

  const handlePoolTypeChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const poolType = e.target.value as ResourcePoolType;
    const providerMap: Record<ResourcePoolType, string> = {
      Docker: 'docker',
      Kubernetes: 'kubernetes',
      Cloud: 'cloud',
      Static: 'static',
    };
    setFormData((prev) => ({
      ...prev,
      poolType,
      providerName: providerMap[poolType],
    }));
  };

  const handleAddTag = () => {
    if (tagKey.trim() && tagValue.trim()) {
      setFormData((prev) => ({
        ...prev,
        tags: {
          ...prev.tags,
          [tagKey.trim()]: tagValue.trim(),
        },
      }));
      setTagKey('');
      setTagValue('');
    }
  };

  const handleRemoveTag = (key: string) => {
    setFormData((prev) => {
      const newTags = { ...prev.tags };
      delete newTags[key];
      return { ...prev, tags: newTags };
    });
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500"></div>
      </div>
    );
  }

  return (
    <div className="max-w-4xl mx-auto py-6 px-4 sm:px-6 lg:px-8">
      <div className="mb-6">
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
          Back to Pools
        </button>
      </div>

      <div className="bg-white shadow-sm rounded-lg">
        <div className="px-4 py-5 sm:p-6">
          <h1 className="text-2xl font-bold text-gray-900 mb-6">
            {isEditing ? 'Edit Resource Pool' : 'Create Resource Pool'}
          </h1>

          {error && (
            <div className="mb-4 bg-red-50 border border-red-200 rounded-md p-4">
              <p className="text-sm text-red-800">{error}</p>
            </div>
          )}

          <form onSubmit={handleSubmit} className="space-y-6">
            <div className="grid grid-cols-1 gap-6 sm:grid-cols-2">
              <div>
                <label htmlFor="poolType" className="block text-sm font-medium text-gray-700">
                  Pool Type
                </label>
                <select
                  id="poolType"
                  name="poolType"
                  value={formData.poolType}
                  onChange={handlePoolTypeChange}
                  disabled={isEditing}
                  className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm disabled:bg-gray-100"
                >
                  <option value="Docker">Docker</option>
                  <option value="Kubernetes">Kubernetes</option>
                  <option value="Cloud">Cloud</option>
                  <option value="Static">Static</option>
                </select>
              </div>

              <div>
                <label htmlFor="providerName" className="block text-sm font-medium text-gray-700">
                  Provider
                </label>
                <input
                  type="text"
                  id="providerName"
                  name="providerName"
                  value={formData.providerName}
                  onChange={handleChange}
                  disabled={isEditing}
                  className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm disabled:bg-gray-100"
                />
              </div>

              <div className="sm:col-span-2">
                <label htmlFor="name" className="block text-sm font-medium text-gray-700">
                  Pool Name
                </label>
                <input
                  type="text"
                  id="name"
                  name="name"
                  value={formData.name}
                  onChange={handleChange}
                  required
                  className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm"
                  placeholder="e.g., Production Docker Pool"
                />
              </div>

              <div>
                <label htmlFor="minSize" className="block text-sm font-medium text-gray-700">
                  Minimum Size
                </label>
                <input
                  type="number"
                  id="minSize"
                  name="minSize"
                  value={formData.minSize}
                  onChange={handleChange}
                  min="0"
                  required
                  className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm"
                />
              </div>

              <div>
                <label htmlFor="maxSize" className="block text-sm font-medium text-gray-700">
                  Maximum Size
                </label>
                <input
                  type="number"
                  id="maxSize"
                  name="maxSize"
                  value={formData.maxSize}
                  onChange={handleChange}
                  min="0"
                  required
                  className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm"
                />
              </div>

              <div>
                <label htmlFor="cpu_m" className="block text-sm font-medium text-gray-700">
                  Default CPU (millicores)
                </label>
                <input
                  type="number"
                  id="cpu_m"
                  name="cpu_m"
                  value={formData.cpu_m}
                  onChange={handleChange}
                  min="0"
                  required
                  className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm"
                />
              </div>

              <div>
                <label htmlFor="memory_mb" className="block text-sm font-medium text-gray-700">
                  Default Memory (MB)
                </label>
                <input
                  type="number"
                  id="memory_mb"
                  name="memory_mb"
                  value={formData.memory_mb}
                  onChange={handleChange}
                  min="0"
                  required
                  className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm"
                />
              </div>

              <div>
                <label htmlFor="gpu" className="block text-sm font-medium text-gray-700">
                  Default GPU (optional)
                </label>
                <input
                  type="number"
                  id="gpu"
                  name="gpu"
                  value={formData.gpu ?? ''}
                  onChange={handleChange}
                  min="0"
                  className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm"
                />
              </div>

              <div className="sm:col-span-2">
                <label className="block text-sm font-medium text-gray-700 mb-2">Tags</label>
                <div className="flex gap-2 mb-3">
                  <input
                    type="text"
                    value={tagKey}
                    onChange={(e) => setTagKey(e.target.value)}
                    placeholder="Key"
                    className="flex-1 rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm"
                  />
                  <input
                    type="text"
                    value={tagValue}
                    onChange={(e) => setTagValue(e.target.value)}
                    placeholder="Value"
                    className="flex-1 rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm"
                  />
                  <button
                    type="button"
                    onClick={handleAddTag}
                    className="px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
                  >
                    Add
                  </button>
                </div>
                <div className="flex flex-wrap gap-2">
                  {Object.entries(formData.tags).map(([key, value]) => (
                    <span
                      key={key}
                      className="inline-flex items-center px-3 py-1 rounded-full text-sm bg-blue-100 text-blue-800"
                    >
                      {key}: {value}
                      <button
                        type="button"
                        onClick={() => handleRemoveTag(key)}
                        className="ml-2 text-blue-600 hover:text-blue-800"
                      >
                        ×
                      </button>
                    </span>
                  ))}
                </div>
              </div>
            </div>

            <div className="flex justify-end space-x-3 pt-6 border-t border-gray-200">
              <button
                type="button"
                onClick={() => navigate('/resource-pools')}
                className="px-4 py-2 border border-gray-300 rounded-md shadow-sm text-sm font-medium text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={saving}
                className="px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50"
              >
                {saving ? (
                  <span className="flex items-center">
                    <svg
                      className="animate-spin -ml-1 mr-2 h-4 w-4 text-white"
                      xmlns="http://www.w3.org/2000/svg"
                      fill="none"
                      viewBox="0 0 24 24"
                    >
                      <circle
                        className="opacity-25"
                        cx="12"
                        cy="12"
                        r="10"
                        stroke="currentColor"
                        strokeWidth="4"
                      />
                      <path
                        className="opacity-75"
                        fill="currentColor"
                        d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                      />
                    </svg>
                    Saving...
                  </span>
                ) : (
                  <span>{isEditing ? 'Update Pool' : 'Create Pool'}</span>
                )}
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  );
}
