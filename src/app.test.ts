import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Photo, Snapshot, Plan } from './api';
const bridge = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: bridge.invoke, isTauri: () => true }));
const tick = async () => { for (let n = 0; n < 8; n++) await Promise.resolve(); };
let snapshot: Snapshot;
const p: Photo = { id: 'photo-1', name: '<img src=x onerror=alert(1)>.png', sha256: 'a'.repeat(64), width: 800, height: 600, category: 'EXTERIOR', technical: 'PENDING', review: 'NEEDS_REVIEW', mask_version: 0, render_key: null, error: null, warnings: [], remote_id: null, settings: null, mask_provider: null };
beforeEach(() => {
  vi.useFakeTimers(); vi.resetModules(); bridge.invoke.mockReset();
  document.body.innerHTML = '<div id="app"></div>';
  snapshot = { photos: [p], settings: { limits: { max_file_mib: 32, max_megapixels: 24 }, provider: { app_id: 'app', version_id: 'pinned-v1', function: null } }, submissions: [], library_location: 'local-library', windows: true };
  bridge.invoke.mockImplementation(async (name: string) => {
    if (name === 'snapshot') return snapshot;
    if (name === 'key_status') return true;
    if (name === 'preview') return 'data:image/png;base64,AA==';
    if (name === 'import_photo') return null;
    if (name === 'plan') return { selected: 1, calls: 1, cached: 0, provider: 'inference.sh', model: 'infsh/birefnet', pin: snapshot.settings.provider, cost_notice: 'Coût exact inconnu.' } satisfies Plan;
    return undefined;
  });
  HTMLDialogElement.prototype.showModal = function () { this.open = true; };
  HTMLDialogElement.prototype.close = function () { this.open = false; };
});
afterEach(() => { vi.clearAllTimers(); vi.useRealTimers(); });
describe('Interface de bureau', () => {
  it('échappe les noms importés et permet de naviguer après une opération', async () => {
    const { start } = await import('./main'); await start();
    expect(document.querySelector('img[onerror]')).toBeNull();
    document.querySelector<HTMLButtonElement>('[data-action=import]')!.click(); await tick();
    expect(document.querySelector<HTMLButtonElement>('[data-action=import]')!.disabled).toBe(false);
    document.querySelector<HTMLButtonElement>('[data-photo]')!.click(); await tick();
    expect(document.querySelector('h2')!.textContent).toContain('<img');
    expect(document.querySelector<HTMLButtonElement>('[data-action=approve]')!.disabled).toBe(true);
    expect(bridge.invoke.mock.calls.some(([name]) => name === 'submit_photo')).toBe(false);
  });
  it('ne soumet rien avant une confirmation explicite et transmet la version affichée', async () => {
    const { start } = await import('./main'); await start();
    document.querySelector<HTMLButtonElement>('[data-photo]')!.click(); await tick();
    document.querySelector<HTMLButtonElement>('[data-action=prepare]')!.click(); await tick();
    expect(document.querySelector('dialog')!.open).toBe(true);
    expect(bridge.invoke.mock.calls.filter(([n]) => n === 'submit_photo')).toHaveLength(0);
    document.querySelector<HTMLButtonElement>('#send-confirm')!.click(); await tick();
    expect(bridge.invoke).toHaveBeenCalledWith('submit_photo', { id: 'photo-1', expectedVersion: 'pinned-v1', confirmed: true });
    expect(bridge.invoke.mock.calls.filter(([n]) => n === 'submit_photo')).toHaveLength(1);
  });
  it('ne relance pas de soumission lorsqu’un masque est en cache', async () => {
    const old = bridge.invoke.getMockImplementation()!;
    bridge.invoke.mockImplementation(async (name: string, ...args: unknown[]) => name === 'key_status' ? false : name === 'plan' ? { selected: 1, calls: 0, cached: 1, provider: 'inference.sh', model: 'infsh/birefnet', pin: snapshot.settings.provider, cost_notice: 'Aucun appel.' } : old(name, ...args));
    const { start } = await import('./main'); await start();
    document.querySelector<HTMLButtonElement>('[data-photo]')!.click(); await tick();
    document.querySelector<HTMLButtonElement>('[data-action=prepare]')!.click(); await tick();
    expect(document.querySelector<HTMLButtonElement>('#send-confirm')!.disabled).toBe(false);
    document.querySelector<HTMLButtonElement>('#send-confirm')!.click(); await tick();
    expect(bridge.invoke.mock.calls.filter(([n]) => n === 'submit_photo')).toHaveLength(0);
  });
  it('efface le champ de clé sans afficher la valeur enregistrée', async () => {
    const { start } = await import('./main'); await start();
    document.querySelector<HTMLButtonElement>('[data-view=settings]')!.click();
    const input = document.querySelector<HTMLInputElement>('#api-key')!;
    input.value = 'test-only-not-a-secret';
    document.querySelector<HTMLFormElement>('#key-form')!.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
    expect(input.value).toBe(''); await tick();
    expect(document.body.textContent).not.toContain('test-only-not-a-secret');
    expect(bridge.invoke).toHaveBeenCalledWith('save_key', { key: 'test-only-not-a-secret' });
  });
});
