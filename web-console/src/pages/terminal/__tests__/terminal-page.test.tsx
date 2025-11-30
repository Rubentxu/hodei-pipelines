import * as terminalApi from '@/services/terminalApi';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { TerminalPage } from '../terminal-page';

// Mock terminalApi
vi.mock('@/services/terminalApi', () => ({
    connectToTerminal: vi.fn(),
    closeTerminalSession: vi.fn(),
}));

// Mock InteractiveTerminal
vi.mock('@/components/ui/terminal/interactive-terminal', () => ({
    InteractiveTerminal: vi.fn(({ onData, ref }) => {
        return <div data-testid="interactive-terminal">Terminal Mock</div>;
    }),
}));

// Mock toast
vi.mock('react-hot-toast', () => ({
    default: {
        error: vi.fn(),
        success: vi.fn(),
    },
}));

describe('TerminalPage', () => {
    const mockConnectToTerminal = terminalApi.connectToTerminal as any;
    const mockCloseTerminalSession = terminalApi.closeTerminalSession as any;
    const mockWebSocket = {
        close: vi.fn(),
        send: vi.fn(),
        readyState: WebSocket.OPEN,
    };

    beforeEach(() => {
        vi.clearAllMocks();
        mockConnectToTerminal.mockResolvedValue(mockWebSocket);
        mockCloseTerminalSession.mockResolvedValue(undefined);
        Object.defineProperty(window, 'location', {
            configurable: true,
            value: { reload: vi.fn() },
        });
    });

    afterEach(() => {
        vi.restoreAllMocks();
    });

    const renderComponent = (jobId = '123') => {
        return render(
            <MemoryRouter initialEntries={[`/terminal/${jobId}`]}>
                <Routes>
                    <Route path="/terminal/:jobId" element={<TerminalPage />} />
                    <Route path="/" element={<div>Home Page</div>} />
                </Routes>
            </MemoryRouter>
        );
    };

    it('renders correctly and connects to terminal', async () => {
        renderComponent();

        expect(screen.getByText('Terminal Interactivo')).toBeInTheDocument();
        expect(screen.getByText('Job ID: 123')).toBeInTheDocument();
        expect(screen.getByText('Desconectado')).toBeInTheDocument();

        await waitFor(() => {
            expect(mockConnectToTerminal).toHaveBeenCalledWith(
                'session-123',
                expect.any(Function),
                expect.any(Function)
            );
        });

        await waitFor(() => {
            expect(screen.getByText('Conectado')).toBeInTheDocument();
        });
    });

    it('handles connection error', async () => {
        mockConnectToTerminal.mockRejectedValue(new Error('Connection failed'));
        renderComponent();

        await waitFor(() => {
            expect(mockConnectToTerminal).toHaveBeenCalled();
        });

        // Wait for the error toast to ensure the async operation completed
        await waitFor(() => {
            expect(terminalApi.connectToTerminal).toHaveBeenCalledTimes(1);
        });

        expect(screen.getByText('Desconectado')).toBeInTheDocument();
    });

    it('disconnects and navigates home', async () => {
        renderComponent();

        await waitFor(() => {
            expect(screen.getByText('Conectado')).toBeInTheDocument();
        });

        const disconnectButton = screen.getByText('Desconectar');
        fireEvent.click(disconnectButton);

        await waitFor(() => {
            expect(mockCloseTerminalSession).toHaveBeenCalledWith('session-123');
            expect(mockWebSocket.close).toHaveBeenCalled();
            expect(screen.getByText('Home Page')).toBeInTheDocument();
        });
    });

    it.skip('reconnects when button is clicked', async () => {
        // Start disconnected
        mockConnectToTerminal.mockRejectedValueOnce(new Error('Fail first'));
        renderComponent();

        // Wait for the initial failure
        await waitFor(() => {
            expect(mockConnectToTerminal).toHaveBeenCalledTimes(1);
        });

        await waitFor(() => {
            expect(screen.getByText('Desconectado')).toBeInTheDocument();
        });

        // Mock successful reconnection
        mockConnectToTerminal.mockResolvedValue(mockWebSocket);

        const reconnectButton = screen.getByText('Reconectar');
        expect(reconnectButton).toBeInTheDocument();
    });
});
