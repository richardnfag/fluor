"use client";

import { Card } from '@/components/ui/Card';
import { ExecutionsChart } from '@/components/ExecutionsChart';
import { LogViewer } from '@/components/LogViewer';
import { useEffect, useState } from 'react';

const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080';


export default function DashboardPage() {
    const [logs, setLogs] = useState([]);
    const [loading, setLoading] = useState(true);
    const [stats, setStats] = useState({ functions: 0, triggers: 0 });
    const [systemStatus, setSystemStatus] = useState<'Operational' | 'Degraded' | 'Down'>('Down');
    const [executionMetrics, setExecutionMetrics] = useState([]);

    useEffect(() => {
        const fetchData = async () => {
            try {
                const [logsRes, functionsRes, triggersRes, healthRes, metricsRes] = await Promise.all([
                    fetch(`${API_URL}/telemetry/logs`),
                    fetch(`${API_URL}/functions`),
                    fetch(`${API_URL}/triggers`),
                    fetch(`${API_URL}/function/healthz`).catch(() => null),
                    fetch(`${API_URL}/telemetry/metrics/overall`)
                ]);

                if (logsRes.ok) {
                    const data = await logsRes.json();
                    setLogs(data);
                }

                if (functionsRes.ok) {
                    const data = await functionsRes.json();
                    setStats(prev => ({ ...prev, functions: data.length }));
                }

                if (triggersRes.ok) {
                    const data = await triggersRes.json();
                    setStats(prev => ({ ...prev, triggers: data.length }));
                }

                if (metricsRes.ok) {
                    const data = await metricsRes.json();
                    setExecutionMetrics(data);
                }

                if (healthRes && healthRes.ok) {
                    try {
                        const health = await healthRes.json();
                        setSystemStatus(health.status === 'ok' ? 'Operational' : 'Degraded');
                    } catch {
                        setSystemStatus('Degraded');
                    }
                } else {
                    setSystemStatus('Down');
                }

            } catch (error) {
                console.error('Failed to fetch dashboard data', error);
                setSystemStatus('Down');
            } finally {
                setLoading(false);
            }
        };

        fetchData();
        const interval = setInterval(fetchData, 5000); // Poll every 5 seconds
        return () => clearInterval(interval);
    }, []);

    const getStatusColor = (status: string) => {
        switch (status) {
            case 'Operational': return 'bg-green-500';
            case 'Degraded': return 'bg-yellow-500';
            case 'Down': return 'bg-red-500';
            default: return 'bg-gray-500';
        }
    };

    return (
        <div className="space-y-6">
            <div className="flex flex-col gap-2">
                <h1 className="text-3xl font-bold tracking-tight">Dashboard</h1>
                <p className="text-[var(--accents-5)]">Welcome to the Fluor Runtime Platform.</p>
            </div>

            <div className="grid grid-cols-1 gap-6 md:grid-cols-3">
                {/* Stats cards */}
                <Card className="p-6">
                    <h3 className="text-lg font-medium">System Status</h3>
                    <div className="mt-2 flex items-center gap-2">
                        <div className={`h-3 w-3 rounded-full ${getStatusColor(systemStatus)}`} />
                        <span className="font-medium">{systemStatus}</span>
                    </div>
                </Card>
                <Card className="p-6">
                    <h3 className="text-lg font-medium">Functions</h3>
                    <p className="mt-2 text-3xl font-bold">{stats.functions}</p>
                </Card>
                <Card className="p-6">
                    <h3 className="text-lg font-medium">Triggers</h3>
                    <p className="mt-2 text-3xl font-bold">{stats.triggers}</p>
                </Card>
            </div>

            <div className="grid grid-cols-1">
                <ExecutionsChart data={executionMetrics} />
            </div>

            <div className="grid grid-cols-1">
                <h2 className="text-xl font-semibold mb-4">Live Event Log</h2>
                {loading ? (
                    <div className="text-[var(--accents-5)] text-sm animate-pulse">Loading logs...</div>
                ) : (
                    <LogViewer logs={logs} showFunctionName={true} />
                )}
            </div>
        </div>
    );
}
