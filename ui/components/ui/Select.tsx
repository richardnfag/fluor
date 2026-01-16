import { SelectHTMLAttributes, forwardRef } from 'react';
import { twMerge } from 'tailwind-merge';
import { clsx } from 'clsx';
import { ChevronDown } from 'lucide-react';

export interface SelectProps extends SelectHTMLAttributes<HTMLSelectElement> {
    label?: string;
    options: { value: string; label: string }[];
}

export const Select = forwardRef<HTMLSelectElement, SelectProps>(
    ({ className, label, id, options, ...props }, ref) => {
        return (
            <div className="flex flex-col gap-1.5 w-full">
                {label && (
                    <label htmlFor={id} className="text-sm font-medium text-[var(--accents-5)]">
                        {label}
                    </label>
                )}
                <div className="relative">
                    <select
                        id={id}
                        ref={ref}
                        className={twMerge(
                            clsx(
                                'flex h-10 w-full appearance-none rounded-md glass-glow bg-white/80 dark:bg-black/30 backdrop-blur-2xl px-3 py-2 text-sm text-[var(--foreground)] dark:text-white placeholder:text-neutral-400 focus:outline-none focus:ring-2 focus:ring-black/20 dark:focus:ring-white/20 focus:border-black dark:focus:border-white disabled:cursor-not-allowed disabled:opacity-50 transition-all duration-200',
                                className
                            )
                        )}
                        {...props}
                    >
                        {options.map((option) => (
                            <option key={option.value} value={option.value} className="bg-black text-white">
                                {option.label}
                            </option>
                        ))}
                    </select>
                    <div className="pointer-events-none absolute inset-y-0 right-0 flex items-center px-2 text-black/50 dark:text-white/50">
                        <ChevronDown size={16} />
                    </div>
                </div>
            </div>
        );
    }
);

Select.displayName = 'Select';
