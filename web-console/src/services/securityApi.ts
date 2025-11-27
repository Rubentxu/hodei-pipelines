import { User, Role, Group, AuditLog, SecurityMetricsResponse } from '@/types';

export async function getUsers(): Promise<User[]> {
  const response = await fetch('/api/security/users');

  if (!response.ok) {
    throw new Error('Failed to fetch users');
  }

  return response.json();
}

export async function getUser(id: string): Promise<User> {
  const response = await fetch(`/api/security/users/${id}`);

  if (!response.ok) {
    throw new Error('Failed to fetch user');
  }

  return response.json();
}

export async function createUser(user: Partial<User>): Promise<User> {
  const response = await fetch('/api/security/users', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(user),
  });

  if (!response.ok) {
    throw new Error('Failed to create user');
  }

  return response.json();
}

export async function updateUser(id: string, updates: Partial<User>): Promise<User> {
  const response = await fetch(`/api/security/users/${id}`, {
    method: 'PATCH',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(updates),
  });

  if (!response.ok) {
    throw new Error('Failed to update user');
  }

  return response.json();
}

export async function deleteUser(id: string): Promise<void> {
  const response = await fetch(`/api/security/users/${id}`, {
    method: 'DELETE',
  });

  if (!response.ok) {
    throw new Error('Failed to delete user');
  }
}

export async function toggleUserStatus(id: string): Promise<User> {
  const response = await fetch(`/api/security/users/${id}/toggle-status`, {
    method: 'POST',
  });

  if (!response.ok) {
    throw new Error('Failed to toggle user status');
  }

  return response.json();
}

export async function getRoles(): Promise<Role[]> {
  const response = await fetch('/api/security/roles');

  if (!response.ok) {
    throw new Error('Failed to fetch roles');
  }

  return response.json();
}

export async function createRole(role: Partial<Role>): Promise<Role> {
  const response = await fetch('/api/security/roles', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(role),
  });

  if (!response.ok) {
    throw new Error('Failed to create role');
  }

  return response.json();
}

export async function updateRole(id: string, updates: Partial<Role>): Promise<Role> {
  const response = await fetch(`/api/security/roles/${id}`, {
    method: 'PATCH',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(updates),
  });

  if (!response.ok) {
    throw new Error('Failed to update role');
  }

  return response.json();
}

export async function deleteRole(id: string): Promise<void> {
  const response = await fetch(`/api/security/roles/${id}`, {
    method: 'DELETE',
  });

  if (!response.ok) {
    throw new Error('Failed to delete role');
  }
}

export async function getGroups(): Promise<Group[]> {
  const response = await fetch('/api/security/groups');

  if (!response.ok) {
    throw new Error('Failed to fetch groups');
  }

  return response.json();
}

export async function getAuditLogs(
  startDate?: string,
  endDate?: string
): Promise<AuditLog[]> {
  const params = new URLSearchParams();
  if (startDate) params.append('startDate', startDate);
  if (endDate) params.append('endDate', endDate);

  const response = await fetch(`/api/security/audit-logs?${params.toString()}`);

  if (!response.ok) {
    throw new Error('Failed to fetch audit logs');
  }

  return response.json();
}

export async function getSecurityMetrics(): Promise<SecurityMetricsResponse> {
  const response = await fetch('/api/security/metrics');

  if (!response.ok) {
    throw new Error('Failed to fetch security metrics');
  }

  return response.json();
}

export async function subscribeToAuditLogs(
  callback: (logs: AuditLog[]) => void
): Promise<() => void> {
  const eventSource = new EventSource('/api/security/audit-logs/stream');

  eventSource.onmessage = (event) => {
    try {
      const data = JSON.parse(event.data);
      callback(data);
    } catch (error) {
      console.error('Failed to parse audit logs data:', error);
    }
  };

  eventSource.onerror = (error) => {
    console.error('EventSource error:', error);
  };

  return () => {
    eventSource.close();
  };
}
