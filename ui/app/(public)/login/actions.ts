'use server';

import { cookies } from 'next/headers';
import { redirect } from 'next/navigation';

export async function loginAction(prevState: any, formData: FormData) {
    const email = formData.get('email') as string;
    const password = formData.get('password') as string;

    const API_URL = process.env.API_URL || 'http://api:8080';

    try {
        const res = await fetch(`${API_URL}/login`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ email, password }),
        });

        if (!res.ok) {
            return { error: 'Invalid credentials' };
        }

        const data = await res.json();

        // Set HttpOnly Cookie
        const cookieStore = await cookies();
        cookieStore.set('token', data.token, {
            httpOnly: true,
            secure: process.env.NODE_ENV === 'production',
            sameSite: 'strict',
            maxAge: 60 * 60 * 24, // 1 day
            path: '/',
        });

    } catch (error) {
        console.error('Login error:', error);
        return { error: 'Something went wrong. Please try again.' };
    }

    redirect('/dashboard');
}
