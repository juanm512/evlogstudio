import { writable } from 'svelte/store';

export const currentUser = writable<{ email: string; role: string } | null>(null);
