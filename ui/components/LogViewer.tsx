"use client";

import { Card } from '@/components/ui/Card';

interface LogEntry {
    timestamp: string;
    level: string;
    body: string;
    trace_id: string;
    function_name?: string;
}

interface LogViewerProps {
    logs: LogEntry[];
}

export function LogViewer({ logs, showFunctionName = false }: LogViewerProps & { showFunctionName?: boolean }) {
    return (
        <Card className="h-[400px] flex flex-col p-0 overflow-hidden">
            <div className="p-4 border-b border-white/10 bg-black/5 dark:bg-white/5">
                <h3 className="text-sm font-medium text-[var(--accents-5)]">Recent Logs</h3>
            </div>
            <div className="flex-1 overflow-auto p-4 space-y-2 font-mono text-xs">
                {logs.length === 0 ? (
                    <div className="text-[var(--accents-3)] italic">No logs available</div>
                ) : (
                    logs.map((log, i) => (
                        <div key={i} className="flex gap-2 items-start border-b border-white/5 pb-2 last:border-0 last:pb-0">
                            <span className="text-[var(--accents-4)] shrink-0 min-w-[140px]">
                                {new Date(log.timestamp).toLocaleString()}
                            </span>
                            {showFunctionName && log.function_name && (
                                <span className="text-[var(--accents-6)] shrink-0 min-w-[100px] truncate" title={log.function_name}>
                                    {log.function_name}
                                </span>
                            )}
                            <span className={`shrink-0 uppercase font-bold w-12 ${log.level === 'ERROR' ? 'text-red-500' :
                                log.level === 'WARN' ? 'text-yellow-500' : 'text-blue-400'
                                }`}>
                                {log.level}
                            </span>
                            <span className="break-all whitespace-pre-wrap text-[var(--foreground)]">
                                {log.body}
                            </span>
                        </div>
                    ))
                )}
            </div>
        </Card>
    );
}
