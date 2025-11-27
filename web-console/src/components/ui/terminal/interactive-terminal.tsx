import { useEffect, useRef, useImperativeHandle, forwardRef } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { WebLinksAddon } from '@xterm/addon-web-links';
import '@xterm/xterm/css/xterm.css';

export interface TerminalHandle {
  write: (data: string) => void;
  clear: () => void;
  focus: () => void;
}

interface InteractiveTerminalProps {
  onData?: (data: string) => void;
  onExit?: () => void;
  className?: string;
}

export const InteractiveTerminal = forwardRef<TerminalHandle, InteractiveTerminalProps>(
  ({ onData, onExit, className }, ref) => {
    const terminalRef = useRef<HTMLDivElement>(null);
    const xtermRef = useRef<Terminal | null>(null);
    const fitAddonRef = useRef<FitAddon | null>(null);

    useImperativeHandle(ref, () => ({
      write: (data: string) => {
        xtermRef.current?.write(data);
      },
      clear: () => {
        xtermRef.current?.clear();
      },
      focus: () => {
        xtermRef.current?.focus();
      },
    }));

    useEffect(() => {
      if (!terminalRef.current) return;

      const terminal = new Terminal({
        cursorBlink: true,
        fontSize: 14,
        fontFamily: 'Menlo, Monaco, "Courier New", monospace',
        theme: {
          background: '#0f172a',
          foreground: '#f1f5f9',
          cursor: '#3b82f6',
          black: '#0f172a',
          red: '#ef4444',
          green: '#10b981',
          yellow: '#f59e0b',
          blue: '#3b82f6',
          magenta: '#a855f7',
          cyan: '#06b6d4',
          white: '#f1f5f9',
          brightBlack: '#64748b',
          brightRed: '#ef4444',
          brightGreen: '#10b981',
          brightYellow: '#f59e0b',
          brightBlue: '#3b82f6',
          brightMagenta: '#a855f7',
          brightCyan: '#06b6d4',
          brightWhite: '#f1f5f9',
        },
        cols: 80,
        rows: 24,
      });

      const fitAddon = new FitAddon();
      const webLinksAddon = new WebLinksAddon();

      terminal.loadAddon(fitAddon);
      terminal.loadAddon(webLinksAddon);

      terminal.open(terminalRef.current);
      fitAddon.fit();

      terminal.writeln('Hodei Interactive Terminal');
      terminal.writeln('Type your commands below...\r\n');
      terminal.write('$ ');

      let currentLine = '';
      let buffer = '';

      const handleData = (data: string) => {
        const code = data.charCodeAt(0);

        if (code === 13) {
          terminal.writeln('');
          if (currentLine.trim()) {
            onData?.(currentLine + '\n');
          }
          currentLine = '';
          buffer = '';
          terminal.write('$ ');
        } else if (code === 127) {
          if (currentLine.length > 0) {
            currentLine = currentLine.slice(0, -1);
            terminal.write('\b \b');
          }
        } else if (code >= 32 && code <= 126) {
          currentLine += data;
          buffer += data;
          terminal.write(data);
        }
      };

      terminal.onData(handleData);

      const handleResize = () => {
        fitAddon.fit();
      };

      window.addEventListener('resize', handleResize);

      xtermRef.current = terminal;
      fitAddonRef.current = fitAddon;

      return () => {
        window.removeEventListener('resize', handleResize);
        terminal.dispose();
      };
    }, [onData]);

    return (
      <div
        ref={terminalRef}
        className={`w-full h-full ${className}`}
        style={{ minHeight: '400px' }}
      />
    );
  }
);

InteractiveTerminal.displayName = 'InteractiveTerminal';
