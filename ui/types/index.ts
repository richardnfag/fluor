export type Language =
    | 'python'
    | 'rust'
    | 'go'
    | 'csharp'
    | 'javascript'
    | 'moonbit';

export const LANGUAGE_OPTIONS: Array<{ value: Language; label: string }> = [
    { value: 'python', label: 'Python' },
    { value: 'rust', label: 'Rust' },
    { value: 'go', label: 'Go' },
    { value: 'csharp', label: 'C#' },
    { value: 'javascript', label: 'JavaScript' },
    { value: 'moonbit', label: 'MoonBit' },
];

export interface Function {
    name: string;
    language: Language;
    executable: string;
    cpu: string;
    memory: string;
    readonly?: boolean;
}

export interface User {
    id: number;
    name: string;
    email: string;
    role: string;
}

export interface Trigger {
    name: string;
    method: string;
    path: string;
    function: string;
    readonly?: boolean;
}
