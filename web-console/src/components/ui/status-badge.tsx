import { cn } from "@/utils/cn";

interface StatusBadgeProps {
  status: "success" | "error" | "warning" | "running" | "pending";
  label?: string;
  pulse?: boolean;
  className?: string;
}

const statusStyles = {
  success: "bg-success/20 text-success",
  error: "bg-error/20 text-error",
  warning: "bg-warning/20 text-warning",
  running: "bg-running/20 text-running",
  pending: "bg-pending/20 text-pending",
};

const statusColors = {
  success: "bg-success",
  error: "bg-error",
  warning: "bg-warning",
  running: "bg-running",
  pending: "bg-pending",
};

export function StatusBadge({
  status,
  label,
  pulse,
  className,
}: StatusBadgeProps) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-xs font-medium",
        statusStyles[status],
        className,
      )}
    >
      <span
        className={cn(
          "h-1.5 w-1.5 rounded-full",
          statusColors[status],
          pulse && status === "running" && "animate-pulse",
        )}
      />
      {label || status}
    </span>
  );
}
