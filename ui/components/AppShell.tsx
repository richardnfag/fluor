'use client';

import {
    LayoutDashboard,
    Code2,
    Zap,
    Settings,
    User,
    Workflow,
    LogOut,
    FunctionSquare
} from 'lucide-react';
import { logoutAction } from '../app/actions';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

const links = [
    { href: '/dashboard', label: 'Dashboard', icon: LayoutDashboard },
    { href: '/functions', label: 'Functions', icon: Code2 },
    { href: '/triggers', label: 'Triggers', icon: Zap },
    // { href: '/workflows', label: 'Workflows', icon: Workflow },
];

function cn(...inputs: (string | undefined | null | false)[]) {
    return twMerge(clsx(inputs));
}

export function AppShell({ children }: { children: React.ReactNode }) {
    const pathname = usePathname();

    return (
        <div className="flex h-screen">
            <aside className="fixed inset-y-0 left-0 z-50 w-64 glass-glow bg-white/30 dark:bg-black/30">
                <div className="flex h-16 items-center border-b border-white/10 px-6">
                    <Link href="/dashboard" className="flex items-center gap-2 font-semibold">
                        <FunctionSquare size={24} />
                        <span>Fluor</span>
                    </Link>
                </div>

                <nav className="flex-1 space-y-1 p-4">
                    {links.map((link) => {
                        const Icon = link.icon;
                        const isActive = pathname === link.href;
                        return (
                            <Link
                                key={link.href}
                                href={link.href}
                                className={cn(
                                    "flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors",
                                    isActive
                                        ? "glass-glow bg-black/5 dark:bg-white/5 text-[var(--foreground)]"
                                        : "border border-transparent text-[var(--accents-5)] hover:glass-glow hover:bg-black/5 dark:hover:bg-white/5 hover:text-[var(--foreground)]"
                                )}
                            >
                                <Icon size={18} />
                                {link.label}
                            </Link>
                        );
                    })}
                </nav>

                <div className="border-t border-white/20 dark:border-white/10 p-4 space-y-2">
                    <Link href="/profile" className="flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium border border-transparent text-[var(--accents-5)] hover:glass-glow hover:bg-black/5 dark:hover:bg-white/5 hover:text-[var(--foreground)] cursor-pointer transition-colors">
                        <User size={18} />
                        <span>User Profile</span>
                    </Link>
                    <form action={logoutAction}>
                        <button className="w-full flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium border border-transparent text-[var(--accents-5)] hover:glass-glow hover:bg-red-500/10 hover:text-red-500 cursor-pointer transition-colors">
                            <LogOut size={18} />
                            <span>Log Out</span>
                        </button>
                    </form>
                </div>
            </aside>

            <main className="pl-64 w-full">
                <div className="h-full overflow-auto p-8">
                    <div className="mx-auto max-w-6xl">
                        {children}
                    </div>
                </div>
            </main>
        </div>
    );
}

