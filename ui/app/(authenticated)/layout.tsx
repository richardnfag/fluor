import { AppShell } from "@/components/AppShell";

export default function AuthenticatedLayout({
    children,
}: {
    children: React.ReactNode;
}) {
    return (
        <AppShell>
            {children}
        </AppShell>
    );
}
