import { cn } from '@/utils/cn';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { StatusBadge } from '@/components/ui/status-badge';
import { Button } from '@/components/ui/button';
import { User } from '@/types';
import { UserIcon, SettingsIcon, TrashIcon, EditIcon, ShieldIcon } from 'lucide-react';

interface UserManagementProps {
  users: User[];
  onEditUser?: (user: User) => void;
  onDeleteUser?: (userId: string) => void;
  onToggleStatus?: (userId: string) => void;
  className?: string;
}

export function UserManagement({
  users,
  onEditUser,
  onDeleteUser,
  onToggleStatus,
  className,
}: UserManagementProps) {
  const handleToggleStatus = (user: User) => {
    onToggleStatus?.(user.id);
  };

  return (
    <Card className={cn('', className)}>
      <CardHeader>
        <CardTitle className="text-nebula-text-primary flex items-center gap-2">
          <UserIcon className="w-5 h-5" />
          Gestión de Usuarios
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-2">
          {users.map((user) => (
            <div
              key={user.id}
              className="flex items-center justify-between p-4 bg-nebula-surface-secondary rounded hover:bg-nebula-surface-secondary/80 transition-colors"
            >
              <div className="flex items-center gap-4 flex-1">
                <div className="w-10 h-10 rounded-full bg-nebula-accent-blue/20 flex items-center justify-center">
                  <UserIcon className="w-5 h-5 text-nebula-accent-blue" />
                </div>
                <div className="flex-1">
                  <div className="flex items-center gap-2">
                    <h4 className="text-sm font-semibold text-nebula-text-primary">
                      {user.name}
                    </h4>
                    {user.mfaEnabled && (
                      <ShieldIcon className="w-4 h-4 text-nebula-accent-green" />
                    )}
                  </div>
                  <div className="text-xs text-nebula-text-secondary">
                    {user.email}
                  </div>
                </div>
                <div>
                  <StatusBadge
                    status={
                      user.status === 'active' ? 'success' :
                      user.status === 'pending' ? 'warning' : 'error'
                    }
                    label={
                      user.status === 'active' ? 'Activo' :
                      user.status === 'pending' ? 'Pendiente' : 'Inactivo'
                    }
                  />
                </div>
                <div className="text-sm text-nebula-text-secondary">
                  <div className="font-medium">{user.role.name}</div>
                  <div className="text-xs">
                    {user.groups.length} grupo{user.groups.length !== 1 ? 's' : ''}
                  </div>
                </div>
                <div className="text-xs text-nebula-text-secondary">
                  <div>
                    Último acceso: {user.lastLogin ?
                      new Date(user.lastLogin).toLocaleDateString('es-ES') :
                      'Nunca'
                    }
                  </div>
                  <div>
                    Creado: {new Date(user.createdAt).toLocaleDateString('es-ES')}
                  </div>
                </div>
              </div>
              <div className="flex gap-2">
                {user.status === 'active' ? (
                  <Button
                    onClick={() => handleToggleStatus(user)}
                    variant="outline"
                    size="sm"
                    className="border-nebula-accent-yellow text-nebula-accent-yellow hover:bg-nebula-accent-yellow/20"
                  >
                    Desactivar
                  </Button>
                ) : (
                  <Button
                    onClick={() => handleToggleStatus(user)}
                    size="sm"
                    className="bg-nebula-accent-green hover:bg-nebula-accent-green/80"
                  >
                    Activar
                  </Button>
                )}
                <Button
                  onClick={() => onEditUser?.(user)}
                  variant="outline"
                  size="sm"
                  className="border-nebula-surface-secondary text-nebula-text-secondary hover:bg-nebula-surface-secondary"
                >
                  <EditIcon className="w-4 h-4" />
                </Button>
                {!user.isSystem && (
                  <Button
                    onClick={() => onDeleteUser?.(user.id)}
                    variant="outline"
                    size="sm"
                    className="border-nebula-accent-red text-nebula-accent-red hover:bg-nebula-accent-red/20"
                  >
                    <TrashIcon className="w-4 h-4" />
                  </Button>
                )}
              </div>
            </div>
          ))}

          {users.length === 0 && (
            <div className="text-center py-8 text-nebula-text-secondary">
              No hay usuarios registrados
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
