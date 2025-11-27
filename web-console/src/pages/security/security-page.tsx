import { useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { SecurityOverview } from '@/components/ui/security/security-overview';
import { AuditLogViewer } from '@/components/ui/security/audit-log-viewer';
import { UserManagement } from '@/components/ui/security/user-management';
import { RoleManagement } from '@/components/ui/security/role-management';
import { useSecurityMetrics, useAuditLogs } from '@/hooks/useSecurity';
import { useUsers } from '@/hooks/useUsers';
import { useRoles } from '@/hooks/useRoles';
import { ShieldIcon, UsersIcon, FileTextIcon, SettingsIcon } from 'lucide-react';
import { User, Role } from '@/types';

type TabType = 'overview' | 'users' | 'roles' | 'audit';

export function SecurityPage() {
  const [activeTab, setActiveTab] = useState<TabType>('overview');
  const { data: metricsData, isLoading: metricsLoading } = useSecurityMetrics();
  const { logs, isLoading: logsLoading } = useAuditLogs();
  const { users, isLoading: usersLoading, toggleUserStatus } = useUsers();
  const { roles, isLoading: rolesLoading } = useRoles();

  const handleEditUser = (user: User) => {
    console.log('Edit user:', user);
  };

  const handleDeleteUser = (userId: string) => {
    console.log('Delete user:', userId);
  };

  const handleEditRole = (role: Role) => {
    console.log('Edit role:', role);
  };

  const handleDeleteRole = (roleId: string) => {
    console.log('Delete role:', roleId);
  };

  const renderTabContent = () => {
    switch (activeTab) {
      case 'overview':
        return (
          <div className="space-y-6">
            {metricsData ? (
              <>
                <SecurityOverview metrics={metricsData.metrics} />
                <AuditLogViewer logs={metricsData.recentLogs} />
              </>
            ) : metricsLoading ? (
              <Card className="p-6">
                <div className="animate-pulse space-y-4">
                  <div className="h-32 bg-nebula-surface-secondary rounded"></div>
                  <div className="h-64 bg-nebula-surface-secondary rounded"></div>
                </div>
              </Card>
            ) : (
              <Card className="p-6 border-red-500/50 bg-red-500/10">
                <p className="text-red-400">Error al cargar métricas de seguridad</p>
              </Card>
            )}
          </div>
        );

      case 'users':
        return usersLoading ? (
          <Card className="p-6">
            <div className="animate-pulse space-y-4">
              {Array.from({ length: 5 }).map((_, i) => (
                <div key={i} className="h-16 bg-nebula-surface-secondary rounded"></div>
              ))}
            </div>
          </Card>
        ) : (
          <UserManagement
            users={users}
            onEditUser={handleEditUser}
            onDeleteUser={handleDeleteUser}
            onToggleStatus={toggleUserStatus}
          />
        );

      case 'roles':
        return rolesLoading ? (
          <Card className="p-6">
            <div className="animate-pulse space-y-4">
              {Array.from({ length: 3 }).map((_, i) => (
                <div key={i} className="h-32 bg-nebula-surface-secondary rounded"></div>
              ))}
            </div>
          </Card>
        ) : (
          <RoleManagement
            roles={roles}
            onEditRole={handleEditRole}
            onDeleteRole={handleDeleteRole}
          />
        );

      case 'audit':
        return logsLoading ? (
          <Card className="p-6">
            <div className="animate-pulse space-y-4">
              {Array.from({ length: 10 }).map((_, i) => (
                <div key={i} className="h-16 bg-nebula-surface-secondary rounded"></div>
              ))}
            </div>
          </Card>
        ) : (
          <AuditLogViewer logs={logs} />
        );

      default:
        return null;
    }
  };

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-nebula-text-primary flex items-center gap-2">
            <ShieldIcon className="w-6 h-6" />
            Centro de Seguridad
          </h1>
          <p className="text-sm text-nebula-text-secondary mt-1">
            Gestión de identidades, roles y auditoría de seguridad
          </p>
        </div>
        <Button
          variant="outline"
          className="border-nebula-surface-secondary text-nebula-text-secondary hover:bg-nebula-surface-secondary"
        >
          <SettingsIcon className="w-4 h-4 mr-2" />
          Configuración
        </Button>
      </div>

      <Card>
        <CardHeader className="pb-3">
          <div className="flex gap-2 overflow-x-auto">
            <Button
              onClick={() => setActiveTab('overview')}
              variant={activeTab === 'overview' ? 'default' : 'outline'}
              className={
                activeTab === 'overview'
                  ? 'bg-nebula-accent-blue hover:bg-nebula-accent-blue/80'
                  : 'border-nebula-surface-secondary text-nebula-text-secondary hover:bg-nebula-surface-secondary'
              }
            >
              <ShieldIcon className="w-4 h-4 mr-2" />
              Resumen
            </Button>
            <Button
              onClick={() => setActiveTab('users')}
              variant={activeTab === 'users' ? 'default' : 'outline'}
              className={
                activeTab === 'users'
                  ? 'bg-nebula-accent-blue hover:bg-nebula-accent-blue/80'
                  : 'border-nebula-surface-secondary text-nebula-text-secondary hover:bg-nebula-surface-secondary'
              }
            >
              <UsersIcon className="w-4 h-4 mr-2" />
              Usuarios ({users.length})
            </Button>
            <Button
              onClick={() => setActiveTab('roles')}
              variant={activeTab === 'roles' ? 'default' : 'outline'}
              className={
                activeTab === 'roles'
                  ? 'bg-nebula-accent-blue hover:bg-nebula-accent-blue/80'
                  : 'border-nebula-surface-secondary text-nebula-text-secondary hover:bg-nebula-surface-secondary'
              }
            >
              <ShieldIcon className="w-4 h-4 mr-2" />
              Roles ({roles.length})
            </Button>
            <Button
              onClick={() => setActiveTab('audit')}
              variant={activeTab === 'audit' ? 'default' : 'outline'}
              className={
                activeTab === 'audit'
                  ? 'bg-nebula-accent-blue hover:bg-nebula-accent-blue/80'
                  : 'border-nebula-surface-secondary text-nebula-text-secondary hover:bg-nebula-surface-secondary'
              }
            >
              <FileTextIcon className="w-4 h-4 mr-2" />
              Auditoría
            </Button>
          </div>
        </CardHeader>
        <CardContent>{renderTabContent()}</CardContent>
      </Card>
    </div>
  );
}
