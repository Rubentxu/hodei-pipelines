import { useState } from 'react';
import { Card } from '../card';
import { Button } from '../button';
import { StatusBadge } from '../status-badge';
import { useAlertRules, useCreateAlertRule, useUpdateAlertRule, useDeleteAlertRule } from '../../../hooks/useObservability';
import { AlertRule } from '../../../types';

export function AlertRules() {
  const { data: rules = [], isLoading } = useAlertRules();
  const createMutation = useCreateAlertRule();
  const updateMutation = useUpdateAlertRule();
  const deleteMutation = useDeleteAlertRule();

  const [showForm, setShowForm] = useState(false);
  const [editingRule, setEditingRule] = useState<AlertRule | null>(null);

  const handleSubmit = async (formData: any) => {
    try {
      if (editingRule) {
        await updateMutation.mutateAsync({
          ruleId: editingRule.id,
          updates: formData,
        });
      } else {
        await createMutation.mutateAsync(formData);
      }
      setShowForm(false);
      setEditingRule(null);
    } catch (error) {
      console.error('Failed to save rule:', error);
    }
  };

  const handleDelete = async (ruleId: string) => {
    if (confirm('Are you sure you want to delete this alert rule?')) {
      try {
        await deleteMutation.mutateAsync(ruleId);
      } catch (error) {
        console.error('Failed to delete rule:', error);
      }
    }
  };

  const handleToggle = async (rule: AlertRule) => {
    try {
      await updateMutation.mutateAsync({
        ruleId: rule.id,
        updates: { enabled: !rule.enabled },
      });
    } catch (error) {
      console.error('Failed to toggle rule:', error);
    }
  };

  if (showForm) {
    return (
      <AlertRuleForm
        rule={editingRule}
        onSubmit={handleSubmit}
        onCancel={() => {
          setShowForm(false);
          setEditingRule(null);
        }}
      />
    );
  }

  return (
    <Card className="h-[600px] flex flex-col">
      <div className="p-4 border-b border-gray-700 flex items-center justify-between">
        <h3 className="text-lg font-semibold text-gray-100">Alert Rules</h3>
        <Button onClick={() => setShowForm(true)}>Create Rule</Button>
      </div>

      <div className="flex-1 overflow-auto p-4">
        {isLoading ? (
          <div className="flex items-center justify-center h-full">
            <div className="text-gray-400">Loading alert rules...</div>
          </div>
        ) : rules.length === 0 ? (
          <div className="flex items-center justify-center h-full">
            <div className="text-center">
              <div className="text-gray-400 mb-2">No alert rules configured</div>
              <Button onClick={() => setShowForm(true)} variant="outline" size="sm">
                Create Your First Rule
              </Button>
            </div>
          </div>
        ) : (
          <div className="space-y-3">
            {rules.map((rule) => (
              <AlertRuleCard
                key={rule.id}
                rule={rule}
                onEdit={() => {
                  setEditingRule(rule);
                  setShowForm(true);
                }}
                onDelete={() => handleDelete(rule.id)}
                onToggle={() => handleToggle(rule)}
              />
            ))}
          </div>
        )}
      </div>
    </Card>
  );
}

function AlertRuleCard({
  rule,
  onEdit,
  onDelete,
  onToggle,
}: {
  rule: AlertRule;
  onEdit: () => void;
  onDelete: () => void;
  onToggle: () => void;
}) {
  const formatTimestamp = (timestamp: string) => {
    if (!timestamp) return 'Never';
    return new Date(timestamp).toLocaleString();
  };

  return (
    <div className="bg-gray-800 rounded-lg p-4 hover:bg-gray-750 transition-colors">
      <div className="flex items-start justify-between mb-3">
        <div className="flex items-center gap-3">
          <StatusBadge
            status={rule.enabled ? (rule.status === 'active' ? 'success' : 'warning') : 'error'}
          />
          <div>
            <h4 className="text-gray-100 font-medium">{rule.name}</h4>
            <p className="text-sm text-gray-400">{rule.description}</p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button onClick={onToggle} variant="outline" size="sm">
            {rule.enabled ? 'Disable' : 'Enable'}
          </Button>
          <Button onClick={onEdit} variant="outline" size="sm">
            Edit
          </Button>
          <Button onClick={onDelete} variant="outline" size="sm" className="text-red-400">
            Delete
          </Button>
        </div>
      </div>

      <div className="grid grid-cols-2 md:grid-cols-4 gap-3 text-sm">
        <div>
          <span className="text-gray-500">Severity:</span>
          <span className={`ml-2 ${
            rule.severity === 'critical'
              ? 'text-red-400'
              : rule.severity === 'warning'
              ? 'text-yellow-400'
              : 'text-blue-400'
          }`}>
            {rule.severity}
          </span>
        </div>
        <div>
          <span className="text-gray-500">Status:</span>
          <span className="ml-2 text-gray-300">{rule.status}</span>
        </div>
        <div>
          <span className="text-gray-500">Last Triggered:</span>
          <span className="ml-2 text-gray-300">{formatTimestamp(rule.lastTriggered || '')}</span>
        </div>
        <div>
          <span className="text-gray-500">Created:</span>
          <span className="ml-2 text-gray-300">{formatTimestamp(rule.createdAt)}</span>
        </div>
      </div>

      <div className="mt-3 pt-3 border-t border-gray-700">
        <div className="text-xs text-gray-500">Condition:</div>
        <code className="text-sm text-gray-300 bg-gray-900 px-2 py-1 rounded mt-1 inline-block">
          {rule.condition}
        </code>
      </div>
    </div>
  );
}

function AlertRuleForm({
  rule,
  onSubmit,
  onCancel,
}: {
  rule: AlertRule | null;
  onSubmit: (data: any) => void;
  onCancel: () => void;
}) {
  const [formData, setFormData] = useState({
    name: rule?.name || '',
    description: rule?.description || '',
    condition: rule?.condition || '',
    severity: rule?.severity || 'info',
    enabled: rule?.enabled ?? true,
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit(formData);
  };

  return (
    <Card className="p-6">
      <h3 className="text-lg font-semibold text-gray-100 mb-4">
        {rule ? 'Edit Alert Rule' : 'Create Alert Rule'}
      </h3>

      <form onSubmit={handleSubmit} className="space-y-4">
        <div>
          <label className="block text-sm font-medium text-gray-400 mb-1">
            Name
          </label>
          <input
            type="text"
            value={formData.name}
            onChange={(e) => setFormData({ ...formData, name: e.target.value })}
            className="w-full bg-gray-800 border border-gray-600 rounded-lg px-3 py-2 text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
            required
          />
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-400 mb-1">
            Description
          </label>
          <textarea
            value={formData.description}
            onChange={(e) => setFormData({ ...formData, description: e.target.value })}
            className="w-full bg-gray-800 border border-gray-600 rounded-lg px-3 py-2 text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
            rows={3}
            required
          />
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-400 mb-1">
            Condition
          </label>
          <input
            type="text"
            value={formData.condition}
            onChange={(e) => setFormData({ ...formData, condition: e.target.value })}
            className="w-full bg-gray-800 border border-gray-600 rounded-lg px-3 py-2 text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
            placeholder="e.g., cpu_usage > 80%"
            required
          />
          <p className="text-xs text-gray-500 mt-1">
            Define the condition that will trigger this alert
          </p>
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-400 mb-1">
            Severity
          </label>
          <select
            value={formData.severity}
            onChange={(e) => setFormData({ ...formData, severity: e.target.value as any })}
            className="w-full bg-gray-800 border border-gray-600 rounded-lg px-3 py-2 text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value="info">Info</option>
            <option value="warning">Warning</option>
            <option value="critical">Critical</option>
          </select>
        </div>

        <div className="flex items-center gap-2">
          <input
            type="checkbox"
            id="enabled"
            checked={formData.enabled}
            onChange={(e) => setFormData({ ...formData, enabled: e.target.checked })}
            className="w-4 h-4 bg-gray-800 border border-gray-600 rounded focus:ring-2 focus:ring-blue-500"
          />
          <label htmlFor="enabled" className="text-sm text-gray-300">
            Enable this alert rule
          </label>
        </div>

        <div className="flex items-center gap-3 pt-4">
          <Button type="submit">
            {rule ? 'Update Rule' : 'Create Rule'}
          </Button>
          <Button type="button" onClick={onCancel} variant="outline">
            Cancel
          </Button>
        </div>
      </form>
    </Card>
  );
}
