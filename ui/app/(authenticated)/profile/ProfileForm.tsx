'use client';

import { useState, useActionState } from 'react';
import { User } from '@/types';
import { Card } from '@/components/ui/Card';
import { PrimaryButton } from '@/components/ui/Button';
import { updateProfile } from './actions';
import { Pencil, Save, X } from 'lucide-react';

const initialState = {
    message: '',
    success: false,
};

export default function ProfileForm({ user }: { user: User }) {
    const [isEditing, setIsEditing] = useState(false);
    const [state, formAction] = useActionState(updateProfile, initialState);

    // If success, disable editing mode
    if (state.success && isEditing) {
        setIsEditing(false);
        // Reset state success manually if needed or just rely on re-render?
        // state persists, so we might need effect.
    }

    // Better: use direct submit handler calling action if we want client control, 
    // but useFormState is good for progressive enhancement.
    // Let's stick to simple toggle.

    return (
        <Card className="p-0 overflow-hidden">
            <div className="p-6 space-y-6">
                <div className="flex items-center justify-between">
                    <div className="flex items-center gap-4">
                        <div className="h-20 w-20 rounded-full bg-blue-500/10 flex items-center justify-center text-blue-500 text-2xl font-bold">
                            {user.name?.[0]?.toUpperCase() || 'U'}
                        </div>
                        <div>
                            <h2 className="text-xl font-semibold">{user.name}</h2>
                            <p className="text-[var(--accents-5)]">{user.email}</p>
                        </div>
                    </div>
                    <div>
                        {!isEditing ? (
                            <button
                                onClick={() => setIsEditing(true)}
                                className="flex items-center gap-2 text-sm text-[var(--accents-5)] hover:text-[var(--foreground)]"
                            >
                                <Pencil size={16} />
                                Edit
                            </button>
                        ) : (
                            <button
                                onClick={() => setIsEditing(false)}
                                className="flex items-center gap-2 text-sm text-[var(--accents-5)] hover:text-red-500"
                            >
                                <X size={16} />
                                Cancel
                            </button>
                        )}
                    </div>
                </div>

                {state.message && (
                    <div className={`p-3 rounded text-sm ${state.success ? 'bg-green-500/10 text-green-500' : 'bg-red-500/10 text-red-500'}`}>
                        {state.message}
                    </div>
                )}

                {!isEditing ? (
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-6 pt-4 border-t border-white/10">
                        <div>
                            <span className="text-[var(--accents-5)] text-sm block mb-1">Role</span>
                            <span className="font-medium capitalize">{user.role}</span>
                        </div>
                        <div>
                            <span className="text-[var(--accents-5)] text-sm block mb-1">User ID</span>
                            <span className="font-mono text-sm bg-black/10 dark:bg-white/10 px-2 py-1 rounded">
                                {user.id}
                            </span>
                        </div>
                    </div>
                ) : (
                    <form action={formAction} className="space-y-4 pt-4 border-t border-white/10">
                        <div>
                            <label className="text-[var(--accents-5)] text-sm block mb-1">Name</label>
                            <input
                                name="name"
                                defaultValue={user.name}
                                className="w-full bg-black/5 dark:bg-white/5 border border-white/10 rounded px-3 py-2 outline-none focus:border-blue-500"
                            />
                        </div>
                        <div>
                            <label className="text-[var(--accents-5)] text-sm block mb-1">Email</label>
                            <input
                                name="email"
                                defaultValue={user.email}
                                className="w-full bg-black/5 dark:bg-white/5 border border-white/10 rounded px-3 py-2 outline-none focus:border-blue-500"
                            />
                        </div>
                        <div className="flex justify-end">
                            <PrimaryButton type="submit">
                                <Save size={16} className="mr-2" />
                                Save Changes
                            </PrimaryButton>
                        </div>
                    </form>
                )}
            </div>
        </Card>
    );
}
