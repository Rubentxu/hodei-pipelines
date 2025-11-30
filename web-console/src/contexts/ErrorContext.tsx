import { createContext, useContext, useState, useEffect, ReactNode } from "react";
import { ApiError, errorHandler, ErrorHandlerConfig } from "../services/errorHandler";

interface ErrorContextValue {
  // Error state
  errors: ApiError[];
  addError: (error: ApiError) => void;
  removeError: (index: number) => void;
  clearErrors: () => void;

  // Network state
  isOnline: boolean;
  pendingRequests: number;

  // Offline queue
  offlineQueueSize: number;
  clearOfflineQueue: () => void;

  // Error notifications
  showNotification: (error: ApiError) => void;
  notifications: ErrorNotification[];
  dismissNotification: (id: string) => void;

  // Configuration
  updateConfig: (config: Partial<ErrorHandlerConfig>) => void;
}

interface ErrorNotification {
  id: string;
  error: ApiError;
  timestamp: Date;
  dismissed: boolean;
}

const ErrorContext = createContext<ErrorContextValue | undefined>(undefined);

export function useError() {
  const context = useContext(ErrorContext);
  if (!context) {
    throw new Error("useError must be used within ErrorProvider");
  }
  return context;
}

interface ErrorProviderProps {
  children: ReactNode;
  config?: Partial<ErrorHandlerConfig>;
}

export function ErrorProvider({ children, config }: ErrorProviderProps) {
  const [errors, setErrors] = useState<ApiError[]>([]);
  const [notifications, setNotifications] = useState<ErrorNotification[]>([]);
  const [isOnline, setIsOnline] = useState(navigator.onLine);
  const [pendingRequests, setPendingRequests] = useState(0);
  const [offlineQueueSize, setOfflineQueueSize] = useState(0);

  // Initialize error handler
  useEffect(() => {
    if (config) {
      errorHandler.updateConfig?.(config as any);
    }
  }, [config]);

  // Listen for online/offline events
  useEffect(() => {
    const handleOnline = () => setIsOnline(true);
    const handleOffline = () => setIsOnline(false);

    window.addEventListener("online", handleOnline);
    window.addEventListener("offline", handleOffline);

    return () => {
      window.removeEventListener("online", handleOnline);
      window.removeEventListener("offline", handleOffline);
    };
  }, []);

  // Listen for errors
  useEffect(() => {
    const handleError = (error: ApiError) => {
      addError(error);
    };

    errorHandler.addErrorListener(handleError);

    return () => {
      errorHandler.removeErrorListener(handleError);
    };
  }, []);

  // Update offline queue size periodically
  useEffect(() => {
    const interval = setInterval(() => {
      setOfflineQueueSize(errorHandler.getOfflineQueueSize());
    }, 1000);

    return () => clearInterval(interval);
  }, []);

  const addError = (error: ApiError) => {
    setErrors((prev) => [...prev, error]);

    // Auto-dismiss non-critical errors after 5 seconds
    if (!error.retryable && error.statusCode !== 500) {
      setTimeout(() => {
        removeError(errors.length);
      }, 5000);
    }
  };

  const removeError = (index: number) => {
    setErrors((prev) => prev.filter((_, i) => i !== index));
  };

  const clearErrors = () => {
    setErrors([]);
  };

  const showNotification = (error: ApiError) => {
    const notification: ErrorNotification = {
      id: `notif_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      error,
      timestamp: new Date(),
      dismissed: false,
    };

    setNotifications((prev) => [...prev, notification]);

    // Auto-dismiss after 5 seconds
    setTimeout(() => {
      dismissNotification(notification.id);
    }, 5000);
  };

  const dismissNotification = (id: string) => {
    setNotifications((prev) => prev.filter((n) => n.id !== id));
  };

  const clearOfflineQueue = () => {
    errorHandler.clearOfflineQueue();
    setOfflineQueueSize(0);
  };

  const updateConfig = (newConfig: Partial<ErrorHandlerConfig>) => {
    // This would update the error handler config
    // Implementation depends on errorHandler.updateConfig method
  };

  const value: ErrorContextValue = {
    errors,
    addError,
    removeError,
    clearErrors,
    isOnline,
    pendingRequests,
    offlineQueueSize,
    clearOfflineQueue,
    showNotification,
    notifications,
    dismissNotification,
    updateConfig,
  };

  return <ErrorContext.Provider value={value}>{children}</ErrorContext.Provider>;
}
