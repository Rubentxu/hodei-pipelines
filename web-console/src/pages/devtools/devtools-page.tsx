import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { useState } from "react";

export function DevToolsPage() {
    const [activeTab, setActiveTab] = useState<"api" | "state" | "performance">("api");

    return (
        <div className="p-6 space-y-6">
            <div>
                <h1 className="text-3xl font-bold text-gray-100 mb-2">Developer Tools</h1>
                <p className="text-gray-400">
                    Utilities for debugging and inspecting the application state
                </p>
            </div>

            <div className="flex space-x-4 border-b border-gray-700 pb-4">
                <Button
                    variant={activeTab === "api" ? "default" : "ghost"}
                    onClick={() => setActiveTab("api")}
                >
                    API Inspector
                </Button>
                <Button
                    variant={activeTab === "state" ? "default" : "ghost"}
                    onClick={() => setActiveTab("state")}
                >
                    State Viewer
                </Button>
                <Button
                    variant={activeTab === "performance" ? "default" : "ghost"}
                    onClick={() => setActiveTab("performance")}
                >
                    Performance
                </Button>
            </div>

            <div className="grid gap-6">
                {activeTab === "api" && <ApiInspector />}
                {activeTab === "state" && <StateViewer />}
                {activeTab === "performance" && <PerformanceMonitor />}
            </div>
        </div>
    );
}

function ApiInspector() {
    const [url, setUrl] = useState("/api/health");
    const [method, setMethod] = useState("GET");
    const [response, setResponse] = useState<string | null>(null);

    const handleSend = async () => {
        try {
            const res = await fetch(url, { method });
            const data = await res.json();
            setResponse(JSON.stringify(data, null, 2));
        } catch (error) {
            setResponse(JSON.stringify({ error: "Failed to fetch" }, null, 2));
        }
    };

    return (
        <Card className="p-6 space-y-4">
            <h2 className="text-xl font-semibold text-gray-100">API Inspector</h2>
            <div className="flex gap-4">
                <select
                    className="bg-gray-800 border-gray-700 rounded-md px-3 py-2 text-gray-100"
                    value={method}
                    onChange={(e: React.ChangeEvent<HTMLSelectElement>) => setMethod(e.target.value)}
                >
                    <option>GET</option>
                    <option>POST</option>
                    <option>PUT</option>
                    <option>DELETE</option>
                </select>
                <Input
                    value={url}
                    onChange={(e: React.ChangeEvent<HTMLInputElement>) => setUrl(e.target.value)}
                    placeholder="/api/..."
                    className="flex-1"
                />
                <Button onClick={handleSend}>Send Request</Button>
            </div>
            <div className="bg-gray-900 rounded-md p-4 font-mono text-sm overflow-auto max-h-[400px]">
                <pre className="text-green-400">{response || "// Response will appear here"}</pre>
            </div>
        </Card>
    );
}

function StateViewer() {
    return (
        <Card className="p-6">
            <h2 className="text-xl font-semibold text-gray-100 mb-4">Global State</h2>
            <div className="text-gray-400">
                Local Storage / Session Storage viewer could go here.
            </div>
        </Card>
    );
}

function PerformanceMonitor() {
    return (
        <Card className="p-6">
            <h2 className="text-xl font-semibold text-gray-100 mb-4">Performance Metrics</h2>
            <div className="text-gray-400">
                Web Vitals and custom metrics visualization.
            </div>
        </Card>
    );
}
