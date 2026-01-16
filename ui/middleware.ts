import { NextResponse } from 'next/server';
import type { NextRequest } from 'next/server';

export function middleware(request: NextRequest) {
    const token = request.cookies.get('token');

    // Protect dashboard routes
    if (request.nextUrl.pathname.startsWith('/dashboard') ||
        request.nextUrl.pathname.startsWith('/functions') ||
        request.nextUrl.pathname.startsWith('/triggers') ||
        request.nextUrl.pathname.startsWith('/settings')) {

        if (!token) {
            return NextResponse.redirect(new URL('/login', request.url));
        }
    }

    // Redirect authenticated users away from login
    if (request.nextUrl.pathname === '/login' || request.nextUrl.pathname === '/register') {
        if (token) {
            return NextResponse.redirect(new URL('/dashboard', request.url));
        }
    }

    return NextResponse.next();
}

export const config = {
    matcher: ['/dashboard/:path*', '/functions/:path*', '/triggers/:path*', '/settings/:path*', '/login', '/register'],
};
