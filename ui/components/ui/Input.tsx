import { InputHTMLAttributes, forwardRef } from 'react';
import { twMerge } from 'tailwind-merge';
import { clsx } from 'clsx';

export interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
    label?: string;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(
    ({ className, label, id, ...props }, ref) => {
        return (
            <div className="flex flex-col gap-1.5 w-full">
                {label && (
                    <label htmlFor={id} className="text-sm font-medium text-[var(--accents-5)]">
                        {label}
                    </label>
                )}
                <input
                    id={id}
                    ref={ref}
                    className={twMerge(
                        clsx(
                            'flex h-10 w-full rounded-md glass-glow bg-white/80 dark:bg-black/30 backdrop-blur-2xl px-3 py-2 text-sm text-[var(--foreground)] dark:text-white placeholder:text-neutral-400 focus:outline-none focus:ring-2 focus:ring-black/20 dark:focus:ring-white/20 focus:border-black dark:focus:border-white disabled:cursor-not-allowed disabled:opacity-50 transition-all duration-200',
                            className
                        )
                    )}
                    {...props}
                />
            </div>
        );
    }
);

Input.displayName = 'Input';
