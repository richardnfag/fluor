import { PrimaryButton } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Card } from '@/components/ui/Card';
import Link from 'next/link';

export default function RegisterPage() {
    return (
        <div className="flex min-h-screen flex-col items-center justify-center p-6">
            <div className="w-full max-w-[400px] space-y-6">
                <div className="space-y-2 text-center">
                    <div className="mx-auto h-8 w-8 rounded-full bg-[var(--foreground)]" />
                    <h1 className="text-2xl font-bold tracking-tight">Create an account</h1>
                    <p className="text-[var(--accents-5)]">Enter your details to get started.</p>
                </div>

                <Card className="p-6">
                    <form className="space-y-4">
                        <Input
                            id="name"
                            label="Name"
                            type="text"
                            placeholder="John Doe"
                        />
                        <Input
                            id="email"
                            label="Email"
                            type="email"
                            placeholder="user@example.com"
                        />
                        <Input
                            id="password"
                            label="Password"
                            type="password"
                        />
                        <PrimaryButton className="w-full">Sign Up</PrimaryButton>
                    </form>
                </Card>

                <div className="text-center text-sm text-[var(--accents-5)]">
                    Already have an account?{' '}
                    <Link href="/login" className="font-medium text-[var(--foreground)] hover:underline">
                        Log in
                    </Link>
                </div>
            </div>
        </div>
    );
}

