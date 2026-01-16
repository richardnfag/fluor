'use client';

import { useEffect, useState } from 'react';
import { Trigger, Function } from '@/types';
import { PrimaryButton, GhostButton, SecondaryButton } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Select } from '@/components/ui/Select';
import { Modal } from '@/components/ui/Modal';
import { Card } from '@/components/ui/Card';
import { Pencil, Trash2, Plus } from 'lucide-react';

const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080';

export default function TriggersPage() {
    const [triggers, setTriggers] = useState<Trigger[]>([]);
    const [functions, setFunctions] = useState<Function[]>([]);
    const [selectedTrigger, setSelectedTrigger] = useState<Trigger | null>(null);
    const [isModalOpen, setIsModalOpen] = useState(false);
    const [isCreateMode, setIsCreateMode] = useState(false);

    useEffect(() => {
        fetchTriggers();
        fetchFunctions();
    }, []);

    const fetchTriggers = () => {
        fetch(`${API_URL}/triggers`)
            .then((res) => res.json())
            .then((data) => setTriggers(data));
    };

    const fetchFunctions = () => {
        fetch(`${API_URL}/functions`)
            .then((res) => res.json())
            .then((data) => setFunctions(data));
    };

    const handleEdit = (trigger: Trigger) => {
        setSelectedTrigger({ ...trigger });
        setIsCreateMode(false);
        setIsModalOpen(true);
    };

    const handleCreate = () => {
        setSelectedTrigger({
            name: '',
            method: 'GET',
            path: '/',
            function: ''
        });
        setIsCreateMode(true);
        setIsModalOpen(true);
    };

    const handleUpdate = async () => {
        if (!selectedTrigger) return;

        const url = isCreateMode
            ? `${API_URL}/triggers`
            : `${API_URL}/triggers/${selectedTrigger.name}`;

        const method = isCreateMode ? 'POST' : 'PUT';

        await fetch(url, {
            method: method,
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(selectedTrigger),
        });
        setIsModalOpen(false);
        fetchTriggers();
    };

    const handleDelete = async (name: string) => {
        if (confirm(`Are you sure you want to delete ${name}?`)) {
            await fetch(`${API_URL}/triggers/${name}`, {
                method: 'DELETE',
            });
            fetchTriggers();
        }
    };



    return (
        <div className="space-y-6">
            <div className="flex justify-between items-center">
                <div className="flex flex-col gap-2">
                    <h1 className="text-3xl font-bold tracking-tight">Triggers</h1>
                    <p className="text-[var(--accents-5)]">Manage event triggers for your functions.</p>
                </div>
                <PrimaryButton onClick={handleCreate}>
                    <Plus size={16} className="mr-2" />
                    New Trigger
                </PrimaryButton>
            </div>

            <Card className="overflow-hidden">
                <table className="w-full text-left text-sm">
                    <thead className="border-b border-white/20 dark:border-white/10 bg-black/5 dark:bg-white/5">
                        <tr>
                            <th className="px-6 py-3 font-medium text-[var(--accents-5)]">Name</th>
                            <th className="px-6 py-3 font-medium text-[var(--accents-5)]">Method</th>
                            <th className="px-6 py-3 font-medium text-[var(--accents-5)]">Path</th>
                            <th className="px-6 py-3 font-medium text-[var(--accents-5)]">Function</th>
                            <th className="px-6 py-3 font-medium text-[var(--accents-5)]">Actions</th>
                        </tr>
                    </thead>
                    <tbody className="divide-y divide-white/20 dark:divide-white/10">
                        {triggers.map((trigger) => (
                            <tr key={trigger.name} className="hover:bg-black/5 dark:hover:bg-white/5 transition-colors">
                                <td className="px-6 py-4 font-medium">{trigger.name}</td>
                                <td className="px-6 py-4">
                                    <span className="inline-flex items-center rounded-md bg-white/20 dark:bg-white/10 border border-white/20 dark:border-white/10 px-2.5 py-0.5 text-xs font-medium text-[var(--foreground)]">
                                        {trigger.method}
                                    </span>
                                </td>
                                <td className="px-6 py-4 font-mono text-[var(--accents-5)]">{trigger.path}</td>
                                <td className="px-6 py-4 text-[var(--accents-5)]">{trigger.function}</td>
                                <td className="px-6 py-4">
                                    <div className="flex gap-2">
                                        <GhostButton
                                            size="sm"
                                            onClick={() => handleEdit(trigger)}
                                            className={`h-8 w-8 p-0 ${trigger.readonly ? 'opacity-50 cursor-not-allowed text-[var(--accents-3)]' : ''}`}
                                            disabled={trigger.readonly}
                                            title={trigger.readonly ? "Read-only" : "Edit"}
                                        >
                                            <Pencil size={16} />
                                        </GhostButton>
                                        <GhostButton
                                            size="sm"
                                            onClick={() => handleDelete(trigger.name)}
                                            disabled={trigger.readonly}
                                            className={`h-8 w-8 p-0 ${trigger.readonly ? 'opacity-50 cursor-not-allowed text-[var(--accents-3)]' : 'text-red-600 hover:bg-red-50 hover:text-red-700'}`}
                                            title={trigger.readonly ? "Read-only" : "Delete"}
                                        >
                                            <Trash2 size={16} />
                                        </GhostButton>
                                    </div>
                                </td>
                            </tr>
                        ))}
                    </tbody>
                </table>
            </Card>

            {selectedTrigger && (
                <Modal
                    isOpen={isModalOpen}
                    title={isCreateMode ? "Create Trigger" : "Trigger Details"}
                    onClose={() => setIsModalOpen(false)}
                >
                    <div className="space-y-4">
                        <Input
                            id="name"
                            label="Name"
                            value={selectedTrigger.name}
                            readOnly={!isCreateMode}
                            onChange={(e) => setSelectedTrigger({ ...selectedTrigger, name: e.target.value })}
                        />
                        <Input
                            id="method"
                            label="Method"
                            value={selectedTrigger.method}
                            onChange={(e) => setSelectedTrigger({ ...selectedTrigger, method: e.target.value })}
                        />
                        <Input
                            id="path"
                            label="Path"
                            value={selectedTrigger.path}
                            onChange={(e) => setSelectedTrigger({ ...selectedTrigger, path: e.target.value })}
                        />
                        <Select
                            id="function"
                            label="Target Function"
                            value={selectedTrigger.function}
                            onChange={(e) => setSelectedTrigger({ ...selectedTrigger, function: e.target.value })}
                            options={functions.map(f => ({ value: f.name, label: f.name }))}
                        />
                        <div className="flex justify-end gap-2 mt-6">
                            <SecondaryButton onClick={() => setIsModalOpen(false)}>
                                Cancel
                            </SecondaryButton>
                            <PrimaryButton onClick={handleUpdate}>
                                {isCreateMode ? "Create" : "Update"}
                            </PrimaryButton>
                        </div>
                    </div>
                </Modal>
            )}
        </div>
    );
}

