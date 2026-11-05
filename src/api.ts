import { invoke, isTauri } from '@tauri-apps/api/core';
export type Category = 'UNKNOWN' | 'EXTERIOR' | 'INTERIOR' | 'DETAIL';
export type Asset = 'original' | 'cutout' | 'mask' | 'render';
export interface Pin { app_id: string; version_id: string; function: string | null }
export interface Limits { max_file_mib: number; max_megapixels: number }
export interface Composition {
  width: number; height: number; background: string; margin: number; scale: number;
  horizontal: number; ground: number; shadow: boolean; shadow_opacity: number;
  shadow_blur: number; shadow_offset: number;
}
export interface Photo {
  id: string; name: string; sha256: string; width: number; height: number; category: Category;
  technical: string; review: string; mask_version: number; render_key: string | null;
  error: string | null; warnings: string[]; remote_id: string | null;
  settings: Composition | null; mask_provider: Pin | null;
}
export interface Settings { limits: Limits; provider: Pin | null }
export interface Submission { id: string; photo_id: string; version_id: string; remote_id: string | null; state: string; created_at: string }
export interface Snapshot { photos: Photo[]; settings: Settings; submissions: Submission[]; library_location: string; windows: boolean }
export interface Plan { selected: number; calls: number; cached: number; provider: string; model: string; pin: Pin | null; cost_notice: string }
export const defaults = (): Composition => ({ width: 1600, height: 1200, background: 'STUDIO_CLAIR', margin: 0.08, scale: 1, horizontal: 0.5, ground: 0.88, shadow: true, shadow_opacity: 0.16, shadow_blur: 0.035, shadow_offset: 0 });
export const desktop = isTauri();
export async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!desktop) throw new Error('Lancez la version de bureau pour accéder à votre photothèque.');
  return invoke<T>(command, args);
}
export const escapeHtml = (s: unknown): string => String(s ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]!));
export const categoryLabel: Record<Category, string> = { UNKNOWN: 'À classer', EXTERIOR: 'Extérieur', INTERIOR: 'Habitacle', DETAIL: 'Détail' };
const states: Record<string, string> = { PENDING: 'Importée', UPLOADING: 'Envoi du fichier', SUBMITTING: 'Soumission en cours', AMBIGUOUS: 'À réconcilier', REMOTE_RUNNING: 'Détourage en cours', MASK_READY: 'Masque disponible', COMPOSITING: 'Composition en cours', COMPLETED: 'Terminée', FAILED: 'À vérifier', CANCELLED: 'Annulée', REJECTED: 'Refusée', NEEDS_REVIEW: 'À valider', APPROVED: 'Approuvée' };
export const stateLabel = (s: string): string => states[s] ?? s;
