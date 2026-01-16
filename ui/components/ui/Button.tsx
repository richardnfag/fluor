import { ButtonHTMLAttributes, forwardRef } from 'react';
import { twMerge } from 'tailwind-merge';
import { clsx } from 'clsx';

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
    size?: 'sm' | 'md' | 'lg';
}

const baseStyles = 'inline-flex items-center justify-center rounded-md font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50';

const sizeStyles = {
    sm: 'h-8 px-3 text-xs',
    md: 'h-10 px-4 text-sm',
    lg: 'h-12 px-6 text-lg',
};

export const PrimaryButton = forwardRef<HTMLButtonElement, ButtonProps>(
    ({ className, size = 'md', ...props }, ref) => {
        return (
            <button
                ref={ref}
                className={twMerge(
                    clsx(
                        baseStyles,
                        'bg-[var(--button-primary-bg)] text-[var(--button-primary-text)] backdrop-blur-md hover:opacity-80 shadow-lg',
                        sizeStyles[size],
                        className
                    )
                )}
                {...props}
            />
        );
    }
);
PrimaryButton.displayName = 'PrimaryButton';

export const SecondaryButton = forwardRef<HTMLButtonElement, ButtonProps>(
    ({ className, size = 'md', ...props }, ref) => {
        return (
            <button
                ref={ref}
                className={twMerge(
                    clsx(
                        baseStyles,
                        'bg-white/80 dark:bg-black/30 backdrop-blur-2xl glass-glow text-[var(--foreground)] dark:text-white hover:bg-white dark:hover:bg-black/40',
                        sizeStyles[size],
                        className
                    )
                )}
                {...props}
            />
        );
    }
);
SecondaryButton.displayName = 'SecondaryButton';

export const DangerButton = forwardRef<HTMLButtonElement, ButtonProps>(
    ({ className, size = 'md', ...props }, ref) => {
        return (
            <button
                ref={ref}
                className={twMerge(
                    clsx(
                        baseStyles,
                        'bg-red-600/90 backdrop-blur-md text-white hover:bg-red-600',
                        sizeStyles[size],
                        className
                    )
                )}
                {...props}
            />
        );
    }
);
DangerButton.displayName = 'DangerButton';

export const GhostButton = forwardRef<HTMLButtonElement, ButtonProps>(
    ({ className, size = 'md', ...props }, ref) => {
        return (
            <button
                ref={ref}
                className={twMerge(
                    clsx(
                        baseStyles,
                        'text-[var(--accents-5)] hover:text-[var(--foreground)] hover:bg-black/10',
                        sizeStyles[size],
                        className
                    )
                )}
                {...props}
            />
        );
    }
);
GhostButton.displayName = 'GhostButton';
