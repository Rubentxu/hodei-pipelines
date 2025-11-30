import * as useRolesHook from '@/hooks/useRoles';
import * as useSecurityHook from '@/hooks/useSecurity';
import * as useUsersHook from '@/hooks/useUsers';
import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { SecurityPage } from '../security-page';

// Mock subcomponents
vi.mock('@/components/ui/security/security-overview', () => ({
    SecurityOverview: () => <div data-testid="security-overview">Security Overview</div>,
}));
vi.mock('@/components/ui/security/audit-log-viewer', () => ({
    AuditLogViewer: () => <div data-testid="audit-log-viewer">Audit Log Viewer</div>,
}));
vi.mock('@/components/ui/security/user-management', () => ({
    UserManagement: () => <div data-testid="user-management">User Management</div>,
}));
vi.mock('@/components/ui/security/role-management', () => ({
    RoleManagement: () => <div data-testid="role-management">Role Management</div>,
}));

// Mock hooks
vi.mock('@/hooks/useSecurity', () => ({
    useSecurityMetrics: vi.fn(),
    useAuditLogs: vi.fn(),
}));
vi.mock('@/hooks/useUsers', () => ({
    useUsers: vi.fn(),
}));
vi.mock('@/hooks/useRoles', () => ({
    useRoles: vi.fn(),
}));

describe('SecurityPage', () => {
    const mockUseSecurityMetrics = useSecurityHook.useSecurityMetrics as any;
    const mockUseAuditLogs = useSecurityHook.useAuditLogs as any;
    const mockUseUsers = useUsersHook.useUsers as any;
    const mockUseRoles = useRolesHook.useRoles as any;

    beforeEach(() => {
        vi.clearAllMocks();
        mockUseSecurityMetrics.mockReturnValue({
            data: { metrics: {}, recentLogs: [] },
            isLoading: false,
        });
        mockUseAuditLogs.mockReturnValue({
            logs: [],
            isLoading: false,
        });
        mockUseUsers.mockReturnValue({
            users: [],
            isLoading: false,
            toggleUserStatus: vi.fn(),
        });
        mockUseRoles.mockReturnValue({
            roles: [],
            isLoading: false,
        });
    });

    it('renders default tab (Overview)', () => {
        render(<SecurityPage />);
        expect(screen.getByText('Centro de Seguridad')).toBeInTheDocument();
        expect(screen.getByTestId('security-overview')).toBeInTheDocument();
        expect(screen.getByTestId('audit-log-viewer')).toBeInTheDocument();
    });

    it('navigates between tabs', () => {
        render(<SecurityPage />);

        // Switch to Users
        fireEvent.click(screen.getByText(/Usuarios/));
        expect(screen.getByTestId('user-management')).toBeInTheDocument();
        expect(screen.queryByTestId('security-overview')).not.toBeInTheDocument();

        // Switch to Roles
        fireEvent.click(screen.getByText(/Roles/));
        expect(screen.getByTestId('role-management')).toBeInTheDocument();

        // Switch to Audit
        fireEvent.click(screen.getByText('Auditoría'));
        expect(screen.getByTestId('audit-log-viewer')).toBeInTheDocument();
    });

    it('renders loading state for Users tab', () => {
        mockUseUsers.mockReturnValue({
            users: [],
            isLoading: true,
        });

        render(<SecurityPage />);
        fireEvent.click(screen.getByText(/Usuarios/));

        // Check for loading skeleton (animate-pulse)
        const skeleton = document.querySelector('.animate-pulse');
        expect(skeleton).toBeInTheDocument();
    });

    it('renders error state for Overview tab', () => {
        mockUseSecurityMetrics.mockReturnValue({
            data: null,
            isLoading: false,
        });

        render(<SecurityPage />);
        expect(screen.getByText('Error al cargar métricas de seguridad')).toBeInTheDocument();
    });
});
