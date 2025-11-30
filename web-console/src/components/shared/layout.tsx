import { useState } from "react";
import { Link, useLocation } from "react-router-dom";
import { cn } from "@/utils/cn";

interface LayoutProps {
  children: React.ReactNode;
}

const navigationItems = [
  { name: "Dashboard", href: "/", icon: "📊" },
  { name: "Pipelines", href: "/pipelines", icon: "🚀" },
  { name: "Workers", href: "/workers", icon: "📦" },
  { name: "Resource Pools", href: "/resource-pools", icon: "🏊" },
  { name: "Logs Explorer", href: "/logs", icon: "📋" },
  { name: "Alert Management", href: "/alerts", icon: "🚨" },
  { name: "FinOps", href: "/finops", icon: "💰" },
  { name: "Security", href: "/security", icon: "🔒" },
  { name: "Observability", href: "/observability", icon: "📈" },
  { name: "DevTools", href: "/devtools", icon: "🛠️" },
];

export function Layout({ children }: LayoutProps) {
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const location = useLocation();

  return (
    <div className="min-h-screen bg-background">
      {/* Sidebar */}
      <aside
        className={cn(
          "fixed inset-y-0 left-0 z-50 w-64 bg-card border-r border-border transition-transform lg:translate-x-0",
          sidebarOpen ? "translate-x-0" : "-translate-x-full",
        )}
      >
        <div className="flex h-16 items-center border-b border-border px-6">
          <h1 className="text-xl font-bold text-primary">HODEI</h1>
          <span className="ml-2 text-xs text-muted-foreground">v1.0</span>
        </div>

        <nav className="flex-1 space-y-1 px-3 py-4">
          {navigationItems.map((item) => {
            const isActive = location.pathname === item.href;
            return (
              <Link
                key={item.name}
                to={item.href}
                className={cn(
                  "flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors",
                  isActive
                    ? "bg-primary text-primary-foreground"
                    : "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
                )}
              >
                <span>{item.icon}</span>
                {item.name}
              </Link>
            );
          })}
        </nav>

        <div className="border-t border-border p-4">
          <div className="flex items-center gap-3">
            <div className="h-8 w-8 rounded-full bg-primary/20 flex items-center justify-center text-sm font-medium">
              AD
            </div>
            <div className="flex-1">
              <p className="text-sm font-medium">Admin User</p>
              <p className="text-xs text-muted-foreground">admin@company.com</p>
            </div>
          </div>
        </div>
      </aside>

      {/* Main content */}
      <div className="lg:pl-64">
        {/* Top bar */}
        <header className="sticky top-0 z-40 flex h-16 items-center gap-4 border-b border-border bg-background/95 px-4 backdrop-blur supports-[backdrop-filter]:bg-background/60 sm:gap-6 sm:px-6">
          <button
            type="button"
            className="lg:hidden"
            onClick={() => setSidebarOpen(!sidebarOpen)}
          >
            <span className="sr-only">Toggle sidebar</span>☰
          </button>

          <div className="flex flex-1 items-center justify-end gap-4">
            <div className="hidden sm:flex">
              <span className="text-sm text-muted-foreground">
                Cluster:{" "}
                <span className="text-foreground font-medium">
                  prod-us-east-1
                </span>
              </span>
            </div>

            <div className="flex items-center gap-2">
              <span className="inline-block h-2 w-2 rounded-full bg-success"></span>
              <span className="text-sm text-muted-foreground">Connected</span>
            </div>
          </div>
        </header>

        {/* Page content */}
        <main className="p-4 sm:p-6 lg:p-8">{children}</main>
      </div>
    </div>
  );
}
