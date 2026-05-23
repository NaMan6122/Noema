import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

export type ThemeChoice = 'system' | 'light' | 'dark';
export type ResolvedTheme = 'light' | 'dark';

export const themeChoice = writable<ThemeChoice>('system');

export const systemPreference = writable<ResolvedTheme>(
  window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
);

export const resolvedTheme = derived(
  [themeChoice, systemPreference],
  ([$choice, $sys]) => ($choice === 'system' ? $sys : $choice) as ResolvedTheme
);

const mql = window.matchMedia('(prefers-color-scheme: dark)');
mql.addEventListener('change', (e) => {
  systemPreference.set(e.matches ? 'dark' : 'light');
});

export async function initTheme() {
  try {
    const saved = await invoke<string>('get_theme');
    if (saved === 'light' || saved === 'dark' || saved === 'system') {
      themeChoice.set(saved);
      localStorage.setItem('noema-theme', saved);
    }
  } catch { /* first run */ }
}

export async function setTheme(choice: ThemeChoice) {
  themeChoice.set(choice);
  localStorage.setItem('noema-theme', choice);
  await invoke('set_theme', { theme: choice }).catch(() => {});
}
