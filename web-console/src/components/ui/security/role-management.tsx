import { cn } from '@/utils/cn';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { StatusBadge } from '@/components/ui/status-badge';
import { Button } from '@/components/ui/button';
import { Role } from '@/types';
import { ShieldIcon, EditIcon, TrashIcon, UsersIcon } from 'lucide-react';

interface RoleManagementProps {
  roles: Role[];
  onEditRole?: (role: Role) => void;
  onDeleteRole?: (roleId: string) => void;
  className?: string;
}

export function RoleManagement({
  roles,
  onEditRole,
  onDeleteRole,
  className,
}: RoleManagementProps) {
  return (
    <Card className={cn('', className)}>
      <CardHeader>
        <CardTitle className="text-nebula-text-primary flex items-center gap-2">
          <ShieldIcon className="w-5 h-5" />
          Gestión de Roles
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-3">
          {roles.map((role) => (
            <div
              key={role.id}
              className="p-4 bg-nebula-surface-secondary rounded hover:bg-nebula-surface-secondary/80 transition-colors"
            >
              <div className="flex items-start justify-between">
                <div className="flex-1">
                  <div className="flex items-center gap-2 mb-2">
                    <h4 className="text-sm font-semibold text-nebula-text-primary">
                      {role.name}
                    </h4>
                    {role.isSystem && (
                      <StatusBadge status="warning" label="Sistema" />
                    )}
                  </div>
                  <p className="text-xs text-nebula-text-secondary mb-3">
                    {role.description}
                  </p>

                  <div className="space-y-2">
                    <div className="text-xs text-nebula-text-secondary font-medium">
                      Permisos ({role.permissions.length}):
                    </div>
                    <div className="flex flex-wrap gap-2">
                      {role.permissions.map((permission) => (
                        <div
                          key={permission.id}
                          className="bg-nebula-surface-primary px-2 py-1 rounded text-xs"
                        >
                          <span className="text-nebula-text-secondary">
                            {permission.resource}
                          </span>
                          <span className="text-nebula-accent-blue mx-1">
                            {permission.action}
                          </span>
                          <span className="text-nebula-text-secondary">
                            ({permission.scope})
                          </span>
                        </div>
                      ))}
                    </div>
                  </div>
                </div>
                <div className="flex flex-col items-end gap-2">
                  <div className="flex items-center gap-1 text-xs text-nebula-text-secondary">
                    <UsersIcon className="w-3 h-3" />
                    <span>{role.userCount || 0} usuarios</span>
                  </div>
                  <div className="flex gap-2">
                    <Button
                      onClick={() => onEditRole?.(role)}
                      variant="outline"
                      size="sm"
                      className="border-nebula-surface-secondary text-nebula-text-secondary hover:bg-nebula-surface-secondary"
                    >
                      <EditIcon className="w-4 h-4" />
                    </Button>
                    {!role.isSystem && (
                      <Button
                        onClick={() => onDeleteRole?.(role.id)}
                        variant="outline"
                        size="sm"
                        className="border-nebula-accent-red text-nebula-accent-red hover:bg-nebula-accent-red/20"
                      >
                        <TrashIcon className="w-4 h-4" />
                      </Button>
                    )}
                  </div>
                </div>
              </div>
            </div>
          ))}

          {roles.length === 0 && (
            <div className="text-center py-8 text-nebula-text-secondary">
              No hay roles configurados
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
