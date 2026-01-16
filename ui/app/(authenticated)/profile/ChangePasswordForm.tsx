'use client';

import { useActionState } from 'react';
import { changePassword } from './actions';
import { PrimaryButton } from '@/components/ui/Button';
import { Card } from '@/components/ui/Card';
import { Lock } from 'lucide-react';

const initialState = {
    message: '',
    success: false,
};

export default function ChangePasswordForm() {
    const [state, formAction] = useActionState(changePassword, initialState);

    return (
        <Card className="p-0 overflow-hidden">
            <div className="p-6 space-y-6">
                <div className="flex items-center gap-3 mb-4">
                    <Lock className="text-[var(--accents-5)]" size={20} />
                    <h3 className="text-lg font-semibold">Change Password</h3>
                </div>

                {state.message && (
                    <div className={`p-3 rounded text-sm ${state.success ? 'bg-green-500/10 text-green-500' : 'bg-red-500/10 text-red-500'}`}>
                        {state.message}
                    </div>
                )}

                <form action={formAction} className="space-y-4">
                    <div>
                        <label className="text-[var(--accents-5)] text-sm block mb-1">Current Password</label>
                        <input
                            type="password"
                            name="current_password"
                            required
                            className="w-full bg-black/5 dark:bg-white/5 border border-white/10 rounded px-3 py-2 outline-none focus:border-blue-500"
                        />
                    </div>
                    <div>
                        <label className="text-[var(--accents-5)] text-sm block mb-1">New Password</label>
                        <input
                            type="password"
                            name="new_password"
                            required
                            className="w-full bg-black/5 dark:bg-white/5 border border-white/10 rounded px-3 py-2 outline-none focus:border-blue-500"
                        />
                    </div>
                    <div className="flex justify-end pt-2">
                        <PrimaryButton type="submit">
                            Update Password
                        </PrimaryButton>
                    </div>
                </form>
            </div>
        </Card>
    );
}
