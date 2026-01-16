'use client';

import { PrimaryButton } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Card } from '@/components/ui/Card';
import Link from 'next/link';

import { useActionState } from 'react';
import { loginAction } from './actions';

export default function LoginPage() {
    const [state, action, isPending] = useActionState(loginAction, null);

    return (
        <div className="flex min-h-screen flex-col items-center justify-center p-6">
            <div className="w-full max-w-[400px] space-y-6">
                <div className="space-y-2 text-center">
                    <div className="mx-auto h-8 w-8 rounded-full bg-[var(--foreground)]" />
                    <h1 className="text-2xl font-bold tracking-tight">Log in to Fluor</h1>
                    <p className="text-[var(--accents-5)]">Enter your email and password to continue.</p>
                </div>

                <Card className="p-6">
                    <form action={action} className="space-y-4">
                        {state?.error && (
                            <div className="text-red-500 text-sm text-center">{state.error}</div>
                        )}
                        <Input
                            id="email"
                            name="email"
                            label="Email"
                            type="email"
                            placeholder="user@example.com"
                            required
                        />
                        <Input
                            id="password"
                            name="password"
                            label="Password"
                            type="password"
                            required
                        />
                        <PrimaryButton className="w-full" disabled={isPending}>
                            {isPending ? 'Logging in...' : 'Log In'}
                        </PrimaryButton>
                    </form>
                </Card>

                <div className="text-center text-sm text-[var(--accents-5)]">
                    Don&apos;t have an account?{' '}
                    <Link href="/register" className="font-medium text-[var(--foreground)] hover:underline">
                        Sign up
                    </Link>
                </div>
            </div>
        </div>
    );
}

