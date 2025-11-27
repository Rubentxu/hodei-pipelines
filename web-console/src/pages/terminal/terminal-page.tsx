import { useEffect, useRef, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { InteractiveTerminal, TerminalHandle } from '@/components/ui/terminal/interactive-terminal';
import { connectToTerminal, closeTerminalSession, TerminalMessage } from '@/services/terminalApi';
import { ArrowLeftIcon, TerminalIcon } from 'lucide-react';
import toast from 'react-hot-toast';

export function TerminalPage() {
  const { jobId } = useParams<{ jobId: string }>();
  const navigate = useNavigate();
  const terminalRef = useRef<TerminalHandle>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const [isConnected, setIsConnected] = useState(false);
  const [sessionId, setSessionId] = useState<string | null>(null);

  useEffect(() => {
    if (!jobId) {
      toast.error('ID de job no proporcionado');
      navigate('/');
      return;
    }

    const connectTerminal = async () => {
      try {
        const ws = await connectToTerminal(
          `session-${jobId}`,
          (message: TerminalMessage) => {
            switch (message.type) {
              case 'output':
                terminalRef.current?.write(message.data);
                break;
              case 'error':
                terminalRef.current?.write(`\r\n\x1b[31mError: ${message.data}\x1b[0m\r\n`);
                break;
              case 'exit':
                terminalRef.current?.write(`\r\n\x1b[33mProcess exited with code ${message.data}\x1b[0m\r\n`);
                setIsConnected(false);
                break;
            }
          },
          (error) => {
            console.error('Terminal WebSocket error:', error);
            setIsConnected(false);
            toast.error('Error de conexión con terminal');
          }
        );

        wsRef.current = ws;
        setSessionId(`session-${jobId}`);
        setIsConnected(true);
        terminalRef.current?.write(`\r\n\x1b[32mConectado al terminal del job ${jobId}\x1b[0m\r\n\n`);
        terminalRef.current?.focus();
      } catch (error) {
        console.error('Failed to connect terminal:', error);
        toast.error('No se pudo conectar al terminal');
      }
    };

    connectTerminal();

    return () => {
      if (sessionId) {
        closeTerminalSession(sessionId).catch(console.error);
      }
      if (wsRef.current) {
        wsRef.current.close();
      }
    };
  }, [jobId, navigate, sessionId]);

  const handleInput = (data: string) => {
    if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify({ type: 'input', data }));
    }
  };

  const handleDisconnect = async () => {
    if (sessionId) {
      try {
        await closeTerminalSession(sessionId);
      } catch (error) {
        console.error('Failed to close session:', error);
      }
    }
    if (wsRef.current) {
      wsRef.current.close();
    }
    navigate('/');
  };

  const handleReconnect = () => {
    if (sessionId) {
      closeTerminalSession(sessionId).catch(console.error);
    }
    if (wsRef.current) {
      wsRef.current.close();
    }
    window.location.reload();
  };

  return (
    <div className="p-6 space-y-6 h-screen flex flex-col">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <Button
            onClick={() => navigate('/')}
            variant="outline"
            className="border-nebula-surface-secondary text-nebula-text-secondary hover:bg-nebula-surface-secondary"
          >
            <ArrowLeftIcon className="w-4 h-4 mr-2" />
            Volver
          </Button>
          <div>
            <h1 className="text-2xl font-bold text-nebula-text-primary flex items-center gap-2">
              <TerminalIcon className="w-6 h-6" />
              Terminal Interactivo
            </h1>
            <p className="text-sm text-nebula-text-secondary">
              Job ID: {jobId}
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <div className={`w-2 h-2 rounded-full ${isConnected ? 'bg-nebula-accent-green animate-pulse' : 'bg-nebula-accent-red'}`} />
          <span className="text-sm text-nebula-text-secondary">
            {isConnected ? 'Conectado' : 'Desconectado'}
          </span>
          {!isConnected && (
            <Button
              onClick={handleReconnect}
              size="sm"
              className="bg-nebula-accent-blue hover:bg-nebula-accent-blue/80"
            >
              Reconectar
            </Button>
          )}
          <Button
            onClick={handleDisconnect}
            size="sm"
            variant="outline"
            className="border-nebula-surface-secondary text-nebula-text-secondary hover:bg-nebula-surface-secondary"
          >
            Desconectar
          </Button>
        </div>
      </div>

      <Card className="flex-1 flex flex-col overflow-hidden">
        <CardHeader className="pb-3">
          <CardTitle className="text-sm font-medium text-nebula-text-secondary">
            Console de Terminal
          </CardTitle>
        </CardHeader>
        <CardContent className="flex-1 p-4 overflow-hidden">
          <div className="h-full border border-nebula-surface-secondary rounded">
            <InteractiveTerminal
              ref={terminalRef}
              onData={handleInput}
              className="terminal-container"
            />
          </div>
        </CardContent>
      </Card>

      <div className="text-xs text-nebula-text-secondary text-center">
        Presiona Ctrl+C para interrumpir el proceso actual
      </div>
    </div>
  );
}
