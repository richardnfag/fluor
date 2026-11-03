"use client";

import { useCallback, useEffect, useState } from 'react';
import { useParams, useRouter } from 'next/navigation';
import { Function } from '@/types';
import { PrimaryButton, SecondaryButton } from '@/components/ui/Button';
import { Card } from '@/components/ui/Card';
import { ArrowLeft, RefreshCw } from 'lucide-react';
import { ExecutionsChart, ExecutionMetric } from '@/components/ExecutionsChart';
import { LogViewer, LogEntry } from '@/components/LogViewer';

const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080';

export default function FunctionDetailsPage() {
    const params = useParams();
    const router = useRouter();
    const functionName = params?.name as string;

    const [func, setFunc] = useState<Function | null>(null);
    const [metrics, setMetrics] = useState<ExecutionMetric[]>([]);
    const [logs, setLogs] = useState<LogEntry[]>([]);
    const [loading, setLoading] = useState(true);
    const [refreshing, setRefreshing] = useState(false);

    const fetchData = useCallback(async () => {
        try {
            // Fetch function details
            // We might not have a direct endpoint for single function by name if list is used,
            // but let's assume we can fetch list and find it, or use the list endpoint with filter if supported.
            // Based on previous code, we only saw `GET /functions`.
            // Ideally we need `GET /functions/{name}`.
            // If checking `api/src/infrastructure/http/handlers/functions.rs` reveals it..
            // But let's assume for now we fetch all and find.
            // UPDATE: `handleUpdate` in `page.tsx` used `${API_URL}/functions/${selectedFunction.name}` for PUT.
            // So `GET /functions/${name}` likely exists or should exist.
            // Let's try to fetch it directly.

            const funcRes = await fetch(`${API_URL}/functions/${functionName}`);
            if (funcRes.ok) {
                const funcData = await funcRes.json();
                setFunc(funcData);
            } else {
                // Fallback: fetch all
                const allRes = await fetch(`${API_URL}/functions`);
                const allData = await allRes.json();
                const found = allData.find((f: Function) => f.name === functionName);
                setFunc(found || null);
            }

            // Fetch Metrics
            const metricsRes = await fetch(`${API_URL}/telemetry/functions/${functionName}/metrics`);
            if (metricsRes.ok) {
                setMetrics(await metricsRes.json());
            }

            // Fetch Logs
            const logsRes = await fetch(`${API_URL}/telemetry/functions/${functionName}/logs`);
            if (logsRes.ok) {
                setLogs(await logsRes.json());
            }
        } catch (e) {
            console.error("Failed to fetch data", e);
        } finally {
            setLoading(false);
            setRefreshing(false);
        }
    }, [functionName]);

    useEffect(() => {
        if (!functionName) {
            return;
        }
        // The request resolves before state is updated. The rule flags the call
        // because the loader also clears the pending flag in finally.
        // eslint-disable-next-line react-hooks/set-state-in-effect -- fetch result is applied after await
        void fetchData();
    }, [functionName, fetchData]);

    if (loading) {
        return <div className="p-8 text-[var(--accents-5)]">Loading...</div>;
    }

    if (!func) {
        return <div className="p-8 text-red-500">Function not found</div>;
    }

    return (
        <div className="space-y-6">
            {/* Header */}
            <div className="flex items-center gap-4">
                <button
                    onClick={() => router.back()}
                    className="p-2 hover:bg-white/10 rounded-full transition-colors"
                >
                    <ArrowLeft size={20} />
                </button>
                <div className="flex-1">
                    <h1 className="text-2xl font-bold tracking-tight">{func.name}</h1>
                    <div className="flex items-center gap-2 text-sm text-[var(--accents-5)] mt-1">
                        <span className="px-2 py-0.5 rounded-full bg-blue-500/10 text-blue-500 border border-blue-500/20">
                            {func.language}
                        </span>
                        <span>•</span>
                        <span>{func.cpu}</span>
                        <span>•</span>
                        <span>{func.memory}</span>
                    </div>
                </div>
                <div className="flex gap-2">
                    <SecondaryButton onClick={() => {
                        setRefreshing(true);
                        void fetchData();
                    }} disabled={refreshing}>
                        <RefreshCw size={16} className={`mr-2 ${refreshing ? "animate-spin" : ""}`} />
                        Refresh
                    </SecondaryButton>
                    <PrimaryButton onClick={() => { /* Reuse logic? Or link back? */ }}>
                        Edit
                    </PrimaryButton>
                </div>
            </div>

            {/* Dashboard Grid */}
            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                {/* Metrics Chart */}
                <div className="md:col-span-2">
                    <h2 className="text-lg font-semibold mb-4 text-[var(--foreground)]">Execution Metrics</h2>
                    <ExecutionsChart data={metrics} />
                </div>

                {/* Logs */}
                <div className="md:col-span-2">
                    <h2 className="text-lg font-semibold mb-4 text-[var(--foreground)]">System Logs & Events</h2>
                    <LogViewer logs={logs} />
                </div>

                {/* Source/Config Preview (Read-only) */}
                <Card className="p-0 overflow-hidden md:col-span-2">
                    <div className="p-4 border-b border-white/10 bg-black/5 dark:bg-white/5">
                        <h3 className="text-sm font-medium text-[var(--accents-5)]">Configuration</h3>
                    </div>
                    <div className="p-4 grid grid-cols-2 gap-4 text-sm">
                        <div>
                            <span className="text-[var(--accents-5)] block mb-1">Executable Path</span>
                            <code className="bg-black/10 dark:bg-white/10 px-2 py-1 rounded font-mono block truncate">
                                {func.executable}
                            </code>
                        </div>
                        <div>
                            <span className="text-[var(--accents-5)] block mb-1">Runtime Status</span>
                            <span className="text-green-500">Active</span>
                        </div>
                    </div>
                </Card>
            </div>
        </div>
    );
}
