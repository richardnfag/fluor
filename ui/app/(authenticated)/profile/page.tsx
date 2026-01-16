import { User } from '@/types';
import { Card } from '@/components/ui/Card';
import { PrimaryButton } from '@/components/ui/Button';
import { LogOut } from 'lucide-react';
import { logoutAction } from '../../actions';
import { cookies } from 'next/headers';
import { redirect } from 'next/navigation';
import ProfileForm from './ProfileForm';
import ChangePasswordForm from './ChangePasswordForm';

async function getUser(): Promise<User | null> {
    const cookieStore = await cookies();
    const token = cookieStore.get('token');

    if (!token) return null;

    const apiUrl = process.env.API_URL || 'http://api:8080';

    try {
        const res = await fetch(`${apiUrl}/me`, {
            headers: {
                'Authorization': `Bearer ${token.value}`
            },
            cache: 'no-store'
        });

        if (!res.ok) return null;
        return res.json();
    } catch (e) {
        console.error("Failed to fetch user", e);
        return null;
    }
}

export default async function ProfilePage() {
    const user = await getUser();

    if (!user) {
        redirect('/login');
    }

    return (
        <div className="space-y-6 max-w-2xl mx-auto">
            <div className="flex items-center justify-between">
                <h1 className="text-2xl font-bold tracking-tight">Profile</h1>
                <form action={logoutAction}>
                    <PrimaryButton type="submit" className="bg-red-500/10 text-red-500 hover:bg-red-500/20 border-red-500/20">
                        <LogOut size={16} className="mr-2" />
                        Logout
                    </PrimaryButton>
                </form>
            </div>

            <ProfileForm user={user} />
            <ChangePasswordForm />
        </div>
    );
}


