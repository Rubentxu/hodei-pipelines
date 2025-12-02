import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi, type MockedFunction } from "vitest";
import { LogsExplorer } from "../LogsExplorer";

// Mock the API module
vi.mock("../../services/logsExplorerApi", () => ({
  getLogs: vi.fn(),
  getLogLevels: vi.fn(),
  getServices: vi.fn(),
  getLogStats: vi.fn(),
  formatTimestamp: vi.fn((timestamp) => new Date(timestamp).toLocaleTimeString()),
  getLogLevelColor: vi.fn((level) => `color-${level}`),
  highlightSearchTerms: vi.fn((text) => text),
  searchLogs: vi.fn(),
  exportLogs: vi.fn(),
}));

// Import the mocked module
import {
  exportLogs,
  getLogLevelColor,
  getLogLevels,
  getLogs,
  getLogStats,
  getServices,
  searchLogs,
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
    (getLogLevels as MockedFunction<typeof getLogLevels>).mockResolvedValue([
      "debug",
      "info",
      "warn",
      "error",
      "fatal",
    ]);

    (getServices as MockedFunction<typeof getServices>).mockResolvedValue([
      "worker-01",
      "worker-02",
      "worker-03",
    ]);

    (getLogs as MockedFunction<typeof getLogs>).mockResolvedValue({
      logs: [mockLogEntry],
      total: 1,
      hasMore: false,
    });

    (getLogStats as MockedFunction<typeof getLogStats>).mockResolvedValue({
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

    expect(screen.getByLabelText("Log Level")).toBeInTheDocument();
    expect(screen.getByLabelText("Service")).toBeInTheDocument();
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

    const levelFilter = screen.getByLabelText("Log Level");
    await user.selectOptions(levelFilter, "error");

    await waitFor(() => {
      expect(getLogs).toHaveBeenCalledWith({
        level: "error",
        limit: 50,
      });
    });
  });

  it("filters logs by service", async () => {
    const user = userEvent.setup();
    render(<LogsExplorer />);

    const serviceFilter = screen.getByLabelText("Service");
    await user.selectOptions(serviceFilter, "worker-01");

    await waitFor(() => {
      expect(getLogs).toHaveBeenCalledWith({
        service: "worker-01",
        limit: 50,
      });
    });
  });

  it("searches logs with text input", async () => {
    const user = userEvent.setup();
    render(<LogsExplorer />);

    const searchInput = screen.getByPlaceholderText("Search logs...");
    await user.type(searchInput, "database{enter}");

    await waitFor(
      () => {
        expect(searchLogs).toHaveBeenCalledWith({
          query: "database",
          fields: ["message", "service", "traceId"],
          fuzzy: true,
        });
      },
      { timeout: 1000 },
    );
  });

  it("displays statistics dashboard", async () => {
    render(<LogsExplorer />);

    await waitFor(() => {
      expect(screen.getByText("115")).toBeInTheDocument();
      expect(screen.getByText("11.0%")).toBeInTheDocument();
      expect(screen.getByText("worker-01")).toBeInTheDocument();
      expect(screen.getByText("info")).toBeInTheDocument();
    });
  });

  it("exports logs as CSV", async () => {
    const user = userEvent.setup();
    const mockBlob = new Blob(["test"], { type: "text/csv" });

    // Mock URL.createObjectURL
    const originalCreateObjectURL = URL.createObjectURL;
    const originalRevokeObjectURL = URL.revokeObjectURL;
    URL.createObjectURL = vi.fn(() => "blob:test");
    URL.revokeObjectURL = vi.fn();

    // Spy on document.createElement but only mock 'a' tag behavior if needed, 
    // or better, just spy on click.
    // Since the component creates an 'a' tag and clicks it, we can spy on the click method of the created element.
    // However, since we can't easily access the element created inside the component, 
    // we can mock createElement to return a spy object ONLY for 'a' tags.

    const originalCreateElement = document.createElement.bind(document);
    const mockCreateElement = vi
      .spyOn(document, "createElement")
      .mockImplementation((tagName, options) => {
        if (tagName === "a") {
          return {
            click: vi.fn(),
            remove: vi.fn(),
            style: {},
            setAttribute: vi.fn(),
            href: "",
            download: "",
          } as any;
        }
        return originalCreateElement(tagName, options);
      });

    // Mock fetch to return blob (if exportLogs uses fetch internally, but here exportLogs is mocked)
    // exportLogs is mocked at the top.
    (exportLogs as MockedFunction<typeof exportLogs>).mockResolvedValue(mockBlob);

    render(<LogsExplorer />);

    await waitFor(() => {
      expect(screen.getByText("Logs Explorer")).toBeInTheDocument();
    });

    const exportButton = screen.getByText("Export CSV");
    await user.click(exportButton);

    await waitFor(() => {
      expect(exportLogs).toHaveBeenCalledWith(expect.anything(), { format: "csv" });
    });

    mockCreateElement.mockRestore();
    URL.createObjectURL = originalCreateObjectURL;
    URL.revokeObjectURL = originalRevokeObjectURL;
  });

  it("auto-refreshes logs every 30 seconds", async () => {
    vi.useFakeTimers();
    render(<LogsExplorer />);

    // Initial call
    await waitFor(() => {
      expect(getLogs).toHaveBeenCalledTimes(1);
    });

    // Advance time by 30 seconds
    // Note: getLogs is async, so we might need to await promises.
    // But useFakeTimers should handle setTimeout.
    // However, if getLogs is awaited in useEffect, we need to be careful.

    // Reset mock to clear initial call
    (getLogs as MockedFunction<typeof getLogs>).mockClear();

    // Trigger the interval
    await vi.advanceTimersByTimeAsync(30000);

    await waitFor(() => {
      expect(getLogs).toHaveBeenCalledTimes(1);
    });

    vi.useRealTimers();
  });

  it("handles loading state", async () => {
    (getLogs as MockedFunction<typeof getLogs>).mockImplementation(
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

    // We might miss the loading state if it renders too fast or if we don't wrap in act.
    // But testing-library handles act.
    // However, since we are mocking getLogs with a delay, we should see it.
    // Note: The component sets loading=true initially.

    // Use findBy to wait for it if needed, or getBy if it's immediate.
    // Since it's initial state, it should be there.
    // But we need to make sure we don't wait for the promise to resolve before checking.

    // Actually, render() will trigger useEffect -> loadLogs -> getLogs (async).
    // So immediately after render, loading should be true.

    // But wait, the previous test might have interfered if not cleaned up?
    // cleanup() is called.

    // Let's just check for the spinner or text.
    // The component has a loading spinner div, but maybe no text "Loading logs..."?
    // Let's check LogsExplorer.tsx again.
    // It has: {loading && ( ... <div className="animate-spin ..."></div> )}
    // It does NOT have text "Loading logs...".
    // It has "No logs found..." text.

    // So expect(screen.getByText("Loading logs...")).toBeInTheDocument() will FAIL.
    // We should check for the spinner or just wait for logs.
    // Or add aria-label to spinner in component.

    // For now, I'll assume the spinner is there.
    // But the test was failing with appendChild, so I couldn't see the assertion error.

    // I will skip this test assertion for "Loading logs..." text and check for something else or fix the component to have aria-label.
    // Better: check for the spinner class.

    const spinner = document.querySelector(".animate-spin");
    expect(spinner).toBeInTheDocument();

    await waitFor(() => {
      expect(screen.queryByText("Database connection failed")).toBeInTheDocument();
    });
  });

  it("handles empty logs state", async () => {
    (getLogs as MockedFunction<typeof getLogs>).mockResolvedValue({
      logs: [],
      total: 0,
      hasMore: false,
    });

    render(<LogsExplorer />);

    await waitFor(() => {
      expect(
        screen.getByText("No logs found matching your criteria."),
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
    // The message is inside a p tag. The parent div has the badge?
    // No, the badge is separate.
    // <span className="... bg-red-100 ...">ERROR</span>

    const badge = screen.getByText("ERROR");
    expect(badge).toHaveClass("bg-red-100");
    expect(badge).toHaveClass("text-red-800");

    expect(getLogLevelColor).toHaveBeenCalledWith("error");
  });
});
