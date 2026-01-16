export interface Function {
    name: string;
    language: 'python' | 'rust' | 'go';
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
