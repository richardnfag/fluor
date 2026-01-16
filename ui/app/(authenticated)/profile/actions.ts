'use server';

import { cookies } from 'next/headers';
import { revalidatePath } from 'next/cache';

const API_URL = process.env.API_URL || 'http://api:8080';

export async function updateProfile(prevState: any, formData: FormData) {
    const cookieStore = await cookies();
    const token = cookieStore.get('token');

    if (!token) {
        return { message: 'Not authenticated' };
    }

    const name = formData.get('name') as string;
    const email = formData.get('email') as string;

    try {
        const res = await fetch(`${API_URL}/me`, {
            method: 'PUT',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': `Bearer ${token.value}`
            },
            body: JSON.stringify({ name, email }),
        });

        if (!res.ok) {
            const text = await res.text();
            return { message: text || 'Failed to update profile' };
        }

        revalidatePath('/profile');
        return { message: 'Profile updated successfully', success: true };
    } catch (e) {
        return { message: 'Failed to update profile' };
    }
}

export async function changePassword(prevState: any, formData: FormData) {
    const cookieStore = await cookies();
    const token = cookieStore.get('token');

    if (!token) {
        return { message: 'Not authenticated' };
    }

    const current_password = formData.get('current_password') as string;
    const new_password = formData.get('new_password') as string;

    try {
        const res = await fetch(`${API_URL}/me/password`, {
            method: 'PUT',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': `Bearer ${token.value}`
            },
            body: JSON.stringify({ current_password, new_password }),
        });

        if (!res.ok) {
            const text = await res.text();
            return { message: text || 'Failed to change password' };
        }

        return { message: 'Password changed successfully', success: true };
    } catch (e) {
        return { message: 'Failed to change password' };
    }
}
