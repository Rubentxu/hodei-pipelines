import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import LogsExplorer from "../LogsExplorer";

// Mock the API module
vi.mock("../../services/logsExplorerApi", () => ({
  getLogs: vi.fn(),
  getLogLevels: vi.fn(),
  getServices: vi.fn(),
  getLogStats: vi.fn(),
  formatTimestamp: vi.fn((timestamp) => new Date(timestamp).toLocaleTimeString()),
  getLogLevelColor: vi.fn((level) => `color-${level}`),
}));

// Import the mocked module
import {
  getLogs,
  getLogLevels,
  getServices,
  getLogStats,
  formatTimestamp,
  getLogLevelColor,
} from "../../services/logsExplorerApi";

// Mock the LogEntry type
const mockLogEntry = {
  id: "log-1",
  timestamp: "2025-11-30T09:30:00Z",
  level: "error" as const,
  service: "worker-01",
  message: "Database connection failed",
  traceId: "trace-123",
  metadata: { errorCode: "DB_CONN_ERROR" },
};

describe("LogsExplorer", () => {
  beforeEach(() => {
    vi.clearAllMocks();

    // Setup default mocks
    (getLogLevels as vi.MockedFunction<typeof getLogLevels>).mockResolvedValue([
      "debug",
      "info",
      "warn",
      "error",
      "fatal",
    ]);

    (getServices as vi.MockedFunction<typeof getServices>).mockResolvedValue([
      "worker-01",
      "worker-02",
      "worker-03",
    ]);

    (getLogs as vi.MockedFunction<typeof getLogs>).mockResolvedValue({
      logs: [mockLogEntry],
      total: 1,
      hasMore: false,
    });

    (getLogStats as vi.MockedFunction<typeof getLogStats>).mockResolvedValue({
      totalLogs: 115,
      errorRate: 0.11,
      topServices: [
        { service: "worker-01", count: 45 },
        { service: "worker-02", count: 32 },
      ],
      topLevels: [
        { level: "info", count: 85 },
        { level: "error", count: 12 },
      ],
    });
  });

  it("renders logs explorer header and filters", async () => {
    render(<LogsExplorer />);

    await waitFor(() => {
      expect(screen.getByText("Logs Explorer")).toBeInTheDocument();
    });

    expect(screen.getByLabelText("Filter by level")).toBeInTheDocument();
    expect(screen.getByLabelText("Filter by service")).toBeInTheDocument();
    expect(screen.getByPlaceholderText("Search logs...")).toBeInTheDocument();
  });

  it("displays logs list with proper formatting", async () => {
    render(<LogsExplorer />);

    await waitFor(() => {
      expect(screen.getByText("Database connection failed")).toBeInTheDocument();
    });

    expect(screen.getByText("worker-01")).toBeInTheDocument();
    expect(screen.getByText("trace-123")).toBeInTheDocument();
  });

  it("filters logs by level", async () => {
    const user = userEvent.setup();
    render(<LogsExplorer />);

    const levelFilter = screen.getByLabelText("Filter by level");
    await user.selectOptions(levelFilter, "error");

    await waitFor(() => {
      expect(getLogs).toHaveBeenCalledWith({
        level: "error",
        limit: 100,
      });
    });
  });

  it("filters logs by service", async () => {
    const user = userEvent.setup();
    render(<LogsExplorer />);

    const serviceFilter = screen.getByLabelText("Filter by service");
    await user.selectOptions(serviceFilter, "worker-01");

    await waitFor(() => {
      expect(getLogs).toHaveBeenCalledWith({
        service: "worker-01",
        limit: 100,
      });
    });
  });

  it("searches logs with text input", async () => {
    const user = userEvent.setup();
    render(<LogsExplorer />);

    const searchInput = screen.getByPlaceholderText("Search logs...");
    await user.type(searchInput, "database");

    await waitFor(
      () => {
        expect(getLogs).toHaveBeenCalledWith({
          search: "database",
          limit: 100,
        });
      },
      { timeout: 500 },
    );
  });

  it("displays statistics dashboard", async () => {
    render(<LogsExplorer />);

    await waitFor(() => {
      expect(screen.getByText("115")).toBeInTheDocument();
      expect(screen.getByText("11%")).toBeInTheDocument();
      expect(screen.getByText("worker-01")).toBeInTheDocument();
      expect(screen.getByText("info")).toBeInTheDocument();
    });
  });

  it("exports logs as CSV", async () => {
    const user = userEvent.setup();
    const mockBlob = new Blob(["test"], { type: "text/csv" });
    const mockCreateElement = vi
      .spyOn(document, "createElement")
      .mockImplementation(() => {
        return {
          click: vi.fn(),
          remove: vi.fn(),
          style: {},
        } as any;
      });

    // Mock fetch to return blob
    global.fetch = vi.fn().mockResolvedValue({
      blob: () => Promise.resolve(mockBlob),
    }) as any;

    render(<LogsExplorer />);

    await waitFor(() => {
      expect(screen.getByText("Logs Explorer")).toBeInTheDocument();
    });

    const exportButton = screen.getByText("Export");
    await user.click(exportButton);

    await waitFor(() => {
      expect(fetch).toHaveBeenCalled();
    });

    mockCreateElement.mockRestore();
  });

  it("auto-refreshes logs every 30 seconds", async () => {
    vi.useFakeTimers();
    render(<LogsExplorer />);

    // Initial call
    await waitFor(() => {
      expect(getLogs).toHaveBeenCalledTimes(1);
    });

    // Advance time by 30 seconds
    vi.advanceTimersByTime(30000);

    await waitFor(() => {
      expect(getLogs).toHaveBeenCalledTimes(2);
    });

    vi.useRealTimers();
  });

  it("handles loading state", async () => {
    (getLogs as vi.MockedFunction<typeof getLogs>).mockImplementation(
      () =>
        new Promise((resolve) => {
          setTimeout(() => {
            resolve({
              logs: [mockLogEntry],
              total: 1,
              hasMore: false,
            });
          }, 100);
        }),
    );

    render(<LogsExplorer />);

    expect(screen.getByText("Loading logs...")).toBeInTheDocument();

    await waitFor(() => {
      expect(screen.queryByText("Loading logs...")).not.toBeInTheDocument();
    });
  });

  it("handles empty logs state", async () => {
    (getLogs as vi.MockedFunction<typeof getLogs>).mockResolvedValue({
      logs: [],
      total: 0,
      hasMore: false,
    });

    render(<LogsExplorer />);

    await waitFor(() => {
      expect(
        screen.getByText("No logs found matching your criteria"),
      ).toBeInTheDocument();
    });
  });

  it("displays log entries with proper styling based on level", async () => {
    render(<LogsExplorer />);

    await waitFor(() => {
      expect(screen.getByText("Database connection failed")).toBeInTheDocument();
    });

    // Verify that the log entry has the correct level styling
    const logMessage = screen.getByText("Database connection failed");
    expect(logMessage.closest(".log-entry")).toBeInTheDocument();
    expect(getLogLevelColor).toHaveBeenCalledWith("error");
  });
});
