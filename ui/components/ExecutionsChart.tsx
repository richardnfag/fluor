"use client";

import { LineChart, Line, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid } from 'recharts';
import { Card } from '@/components/ui/Card';

interface ExecutionMetric {
    time_bucket: string;
    count: number;
}

interface ExecutionsChartProps {
    data: ExecutionMetric[];
}

export function ExecutionsChart({ data }: ExecutionsChartProps) {
    // Format date for display
    const formattedData = data.map(d => ({
        ...d,
        time: new Date(d.time_bucket).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
    }));

    return (
        <Card className="h-[300px] p-4 flex flex-col">
            <h3 className="text-sm font-medium text-[var(--accents-5)] mb-4">Executions (Last Hour)</h3>
            <div className="flex-1 w-full min-h-0">
                <ResponsiveContainer width="100%" height="100%">
                    <LineChart data={formattedData}>
                        <CartesianGrid strokeDasharray="3 3" vertical={false} stroke="var(--accents-2)" />
                        <XAxis
                            dataKey="time"
                            stroke="var(--accents-5)"
                            fontSize={12}
                            tickLine={false}
                            axisLine={false}
                        />
                        <YAxis
                            stroke="var(--accents-5)"
                            fontSize={12}
                            tickLine={false}
                            axisLine={false}
                            tickFormatter={(value) => `${value}`}
                        />
                        <Tooltip
                            contentStyle={{
                                backgroundColor: 'var(--background)',
                                borderColor: 'var(--accents-2)',
                                borderRadius: '4px',
                                color: 'var(--foreground)'
                            }}
                            itemStyle={{ color: 'var(--foreground)' }}
                            cursor={{ stroke: 'var(--accents-2)' }}
                        />
                        <Line
                            type="monotone"
                            dataKey="count"
                            stroke="var(--foreground)"
                            strokeWidth={2}
                            dot={{ fill: 'var(--foreground)', r: 3 }}
                            activeDot={{ r: 5, fill: 'var(--foreground)' }}
                        />
                    </LineChart>
                </ResponsiveContainer>
            </div>
        </Card>
    );
}
