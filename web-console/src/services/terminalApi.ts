export interface TerminalSession {
  id: string;
  jobId: string;
  status: 'active' | 'inactive';
}

export interface TerminalMessage {
  type: 'output' | 'input' | 'error' | 'exit';
  data: string;
}

export async function createTerminalSession(
  jobId: string
): Promise<TerminalSession> {
  const response = await fetch('/api/terminal/sessions', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ jobId }),
  });

  if (!response.ok) {
    throw new Error('Failed to create terminal session');
  }

  return response.json();
}

export async function connectToTerminal(
  sessionId: string,
  onMessage: (message: TerminalMessage) => void,
  onError?: (error: Event) => void
): Promise<WebSocket> {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(
      `${window.location.protocol === 'https:' ? 'wss' : 'ws'}://${window.location.host}/ws/terminal/${sessionId}`
    );

    ws.onopen = () => {
      resolve(ws);
    };

    ws.onmessage = (event) => {
      try {
        const message: TerminalMessage = JSON.parse(event.data);
        onMessage(message);
      } catch (error) {
        console.error('Failed to parse terminal message:', error);
      }
    };

    ws.onerror = (error) => {
      onError?.(error);
      reject(error);
    };

    ws.onclose = () => {
      console.log('Terminal session closed');
    };
  });
}

export async function sendTerminalInput(
  sessionId: string,
  input: string
): Promise<void> {
  const response = await fetch(`/api/terminal/sessions/${sessionId}/input`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ input }),
  });

  if (!response.ok) {
    throw new Error('Failed to send terminal input');
  }
}

export async function closeTerminalSession(sessionId: string): Promise<void> {
  const response = await fetch(`/api/terminal/sessions/${sessionId}`, {
    method: 'DELETE',
  });

  if (!response.ok) {
    throw new Error('Failed to close terminal session');
  }
}
