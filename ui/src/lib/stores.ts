import { writable } from 'svelte/store';

export const selectedSources = writable<string[]>([]);
export const currentUser = writable<{ email: string; role: string } | null>(null);
