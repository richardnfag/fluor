import { HTMLAttributes, forwardRef } from 'react';
import { twMerge } from 'tailwind-merge';
import { clsx } from 'clsx';

interface CardProps extends HTMLAttributes<HTMLDivElement> {
    variant?: 'default' | 'danger';
}

export const Card = forwardRef<HTMLDivElement, CardProps>(
    ({ className, variant = 'default', ...props }, ref) => {
        return (
            <div
                ref={ref}
                className={twMerge(
                    clsx(
                        'rounded-lg shadow-sm backdrop-blur-2xl transition-all',
                        {
                            'bg-white/30 dark:bg-black/30 glass-glow': variant === 'default',
                            'bg-red-500/5 dark:bg-red-900/10 border border-red-500/20 dark:border-red-500/10': variant === 'danger',
                        },
                        className
                    )
                )}
                {...props}
            />
        );
    }
);

Card.displayName = 'Card';
