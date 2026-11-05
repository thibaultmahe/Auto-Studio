import './style.css';
import { call, desktop, defaults, escapeHtml as h, stateLabel, categoryLabel } from './api';
import type { Snapshot, Photo, Composition, Asset, Category, Plan } from './api';

type View = 'home' | 'library' | 'workshop' | 'jobs' | 'backgrounds' | 'settings';
let current: Snapshot = { photos: [], settings: { limits: { max_file_mib: 32, max_megapixels: 24 }, provider: null }, submissions: [], library_location: '', windows: false };
let view: View = 'home', selected: string | null = null, busy = false, keyPresent = false;
let draft = defaults(), dirty = false, asset: Asset | 'compare' = 'original', zoom = 1;
let original = '', cutout = '', rendered = '', requestVersion = 0;
let timer: ReturnType<typeof setInterval> | undefined;
const root = () => document.querySelector<HTMLDivElement>('#app')!;
const photo = (): Photo | undefined => current.photos.find(p => p.id === selected);
const icons: Record<string, string> = {
  home: '<path d="m3 10 9-7 9 7v10H3zM9 20v-7h6v7"/>',
  library: '<rect x="3" y="3" width="18" height="18" rx="3"/><circle cx="8" cy="8" r="1.5"/><path d="m3 17 6-5 4 3 3-5 5 6"/>',
  workshop: '<path d="m4 20 6-6m4-4 6-6M4 4l16 16M8 3l-5 5m18 8-5 5"/>',
  jobs: '<path d="M8 6h13M8 12h13M8 18h13M3 6h1M3 12h1M3 18h1"/>',
  backgrounds: '<rect x="3" y="3" width="18" height="18" rx="3"/><path d="M3 15h18M9 3v12"/>',
  settings: '<path d="M4 6h16M4 12h16M4 18h16M8 3v6M16 9v6M10 15v6"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  arrow: '<path d="M4 12h16m-6-6 6 6-6 6"/>',
};
const icon = (name: string) => `<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${icons[name] ?? icons.library}</svg>`;
const disabled = (condition: boolean, reason = '') => condition ? `disabled title="${h(reason)}"` : '';
function button(label: string, action: string, secondary = false, unavailable = false, reason = ''): string {
  return `<button class="${secondary ? 'secondary' : 'primary'}" data-action="${action}" ${disabled(unavailable, reason)}>${h(label)}</button>`;
}
const badge = (label: string, good = false) => `<span class="badge ${good ? 'good' : ''}">${h(label)}</span>`;
const empty = (title: string, text: string) => `<div class="empty">${icon('library')}<h2>${h(title)}</h2><p>${h(text)}</p>${button('Importer une photo', 'import', false, !desktop, 'Disponible dans l’application de bureau.')}</div>`;
function shell(content: string): void {
  const titles: Record<View, string> = { home: 'Votre studio, au quotidien', library: 'Photothèque', workshop: 'Atelier photo', jobs: 'Historique des traitements', backgrounds: 'Les fonds du studio', settings: 'Paramètres' };
  const nav: [View, string][] = [['home', 'Accueil'], ['library', 'Photothèque'], ['workshop', 'Atelier photo'], ['jobs', 'Traitements'], ['backgrounds', 'Fonds'], ['settings', 'Paramètres']];
  root().innerHTML = `<aside class="sidebar"><a class="brand" href="#" data-view="home"><span class="mark">A</span><span>AutoStudio<small>DESKTOP</small></span></a><div class="dealership">PUJOL AUTO <span>ATELIER PHOTO</span></div><nav aria-label="Navigation principale">${nav.map(([key, label]) => `<button data-view="${key}" class="nav ${view === key ? 'active' : ''}">${icon(key)}${label}${key === 'library' ? `<span class="count">${current.photos.length}</span>` : ''}</button>`).join('')}</nav><div class="sidebar-bottom"><span class="dot"></span> Photothèque locale<small>Vos originaux restent conservés.</small><span class="version">Version 0.1 · Phase 1</span></div></aside><main><header><div><p class="eyebrow">PUJOL AUTO / ${view === 'workshop' ? 'CRÉATION' : 'VOTRE ESPACE'}</p><h1>${titles[view]}</h1></div>${button('＋ Importer une photo', 'import', false, !desktop, 'Ouvrez l’application de bureau.')}</header>${!desktop ? '<div class="notice">Interface web seule : l’import et les traitements nécessitent l’application Tauri. Aucune photo de démonstration ni inférence simulée.</div>' : ''}<div id="operation" class="notice" role="status" ${busy ? '' : 'hidden'}>${busy ? 'Opération en cours. Vous pouvez continuer à parcourir l’interface.' : ''}</div><section class="content">${content}</section><footer>AutoStudio Desktop <span>Original conservé · Détourage distant · Composition locale</span></footer></main><div id="toast" role="status" aria-live="polite"></div><dialog id="confirmation" aria-labelledby="confirm-title"></dialog>`;
  bind();
  if (busy) working(true);
}
function home(): string {
  const done = current.photos.filter(p => p.review === 'APPROVED').length;
  return `<section class="hero"><div><p class="eyebrow">LA PHOTO AUTOMOBILE, SIMPLEMENT</p><h2>Le même véhicule.<br>Un cadre plus soigné.</h2><p>Préparez vos photos extérieures dans un studio cohérent,<br>en conservant les détails qui font leur authenticité.</p>${button('Ouvrir la photothèque →', 'library', true)}</div><div class="hero-art" aria-hidden="true"><div class="studio-lines"></div><div class="art-card"><span>01 / VOTRE PHOTO</span><div class="art-frame">${icon('library')}</div><p>Un fond fixe.<br>Vos vrais pixels.</p></div></div></section><div class="stats"><article><span>Photos importées</span><strong>${current.photos.length.toString().padStart(2, '0')}</strong></article><article><span>À valider</span><strong>${current.photos.filter(p => p.render_key && p.review === 'NEEDS_REVIEW').length.toString().padStart(2, '0')}</strong></article><article><span>Photos approuvées</span><strong>${done.toString().padStart(2, '0')}</strong></article></div><div class="section-head"><h2>Dernières photos</h2><button class="text-button" data-view="library">Tout voir ${icon('arrow')}</button></div>${current.photos.length ? cards(current.photos.slice(-4).reverse()) : empty('Votre première photo commence ici', 'Importez une photo, classez-la et préparez son détourage. Aucun envoi automatique.')}<section class="workflow"><h3>Du fichier original au catalogue</h3><ol><li><b>01</b> Importer et classer</li><li><b>02</b> Confirmer le détourage</li><li><b>03</b> Composer et valider</li><li><b>04</b> Exporter</li></ol></section>`;
}
function cards(photos: Photo[]): string {
  return `<div class="photo-grid">${photos.map(p => `<button class="photo-card" data-photo="${h(p.id)}"><div class="thumb"><img data-thumb="${h(p.id)}" alt="${h(p.name)}" loading="lazy"></div><div class="photo-info"><strong>${h(p.name)}</strong><span>${h(categoryLabel[p.category])} · ${p.width} × ${p.height}</span>${badge(stateLabel(p.review === 'APPROVED' ? p.review : p.technical), p.review === 'APPROVED')}</div></button>`).join('')}</div>`;
}
function workshop(): string {
  const p = photo();
  if (!p) return empty('Une photo, un atelier', 'Sélectionnez une photo dans la photothèque pour commencer.');
  const canExport = p.review === 'APPROVED' && p.technical === 'COMPLETED' && !dirty;
  const setting = (name: keyof Composition, label: string, min: number, max: number, step: number) => `<label>${label}<input name="${name}" type="range" min="${min}" max="${max}" step="${step}" value="${draft[name]}"><output>${draft[name]}</output></label>`;
  return `<div class="photo-heading"><div><h2>${h(p.name)}</h2><p>${p.width} × ${p.height} px · Original conservé${p.mask_version ? ` · Masque v${p.mask_version}` : ''}</p>${p.mask_provider ? `<p>Masque BiRefNet · version <code>${h(p.mask_provider.version_id)}</code></p>` : ''}</div>${badge(stateLabel(p.technical))}${badge(stateLabel(p.review), p.review === 'APPROVED')}</div>${p.error ? `<div class="error">${h(p.error)}</div>` : ''}<div class="atelier"><section class="viewer"><div class="viewer-tabs" role="group" aria-label="Vue photo">${(['original', 'cutout', 'render', 'compare'] as const).map(k => `<button data-asset="${k}" class="${asset === k ? 'chosen' : ''}" ${disabled((k === 'cutout' && !p.mask_version) || ((k === 'render' || k === 'compare') && !p.render_key))}>${({ original: 'Original', cutout: 'Détourage', render: 'Composition', compare: 'Avant / après' })[k]}</button>`).join('')}</div><div class="viewport"><div class="scene" style="width:${zoom * 100}%"><img id="base-image" alt="${asset === 'compare' ? 'Original' : 'Vue sélectionnée'}"><img id="after-image" alt="Composition après traitement" ${asset === 'compare' ? '' : 'hidden'}><span class="image-label">${asset === 'compare' ? 'AVANT / APRÈS' : h(({ original: 'ORIGINAL', cutout: 'DÉTOURAGE', render: 'COMPOSITION', mask: 'MASQUE' })[asset])}</span></div></div><div class="viewer-controls"><label>Zoom <input id="zoom" type="range" min="0.5" max="2.5" step="0.1" value="${zoom}"></label><span>Aperçu limité à 1600 px</span></div>${asset === 'compare' ? '<label class="compare-control">Comparer <input id="compare" type="range" min="0" max="100" value="50" aria-label="Position du comparateur avant après"></label>' : ''}<div id="dirty-note" class="notice" ${dirty ? '' : 'hidden'}>Réglages modifiés : cliquez sur Recomposer pour voir et valider le nouveau rendu.</div>${p.warnings.length ? `<div class="quality"><h3>Points à contrôler</h3><ul>${p.warnings.map(w => `<li>${h(w)}</li>`).join('')}</ul><p>Ces indicateurs ne constituent pas une note de qualité.</p></div>` : ''}<div class="review-actions">${button('Approuver', 'approve', false, !p.render_key || dirty || p.technical !== 'COMPLETED', 'Composez un rendu avec les réglages actuels.')} ${button('Refuser', 'reject', true, !p.render_key)}</div></section><aside class="tools"><section><p class="eyebrow">01 / CLASSER</p><label>Type de photo<select id="category">${Object.entries(categoryLabel).map(([key, value]) => `<option value="${key}" ${p.category === key ? 'selected' : ''}>${value}</option>`).join('')}</select></label><p class="help">Seuls les extérieurs explicitement classés peuvent être envoyés. Habitacle et détails restent inchangés.</p>${button('Préparer le détourage', 'prepare', false, p.category !== 'EXTERIOR', 'Classez cette photo en Extérieur.')}${p.remote_id || p.technical === 'SUBMITTING' || p.error ? button('Consulter / réconcilier', 'sync', true) : ''}</section><section><p class="eyebrow">02 / COMPOSER LOCALEMENT</p><form id="composition"><label>Fond<select name="background"><option value="STUDIO_CLAIR" ${draft.background === 'STUDIO_CLAIR' ? 'selected' : ''}>Studio clair · v1</option><option value="STUDIO_FONCE" ${draft.background === 'STUDIO_FONCE' ? 'selected' : ''}>Studio foncé · v1</option></select></label><label>Format<select id="preset"><option value="custom">Dimensions personnalisées</option><option value="catalogue" ${draft.width === 1600 && draft.height === 1200 ? 'selected' : ''}>Catalogue · 1600 × 1200</option><option value="social" ${draft.width === 1080 && draft.height === 1350 ? 'selected' : ''}>Réseaux · 1080 × 1350</option><option value="square" ${draft.width === 1080 && draft.height === 1080 ? 'selected' : ''}>Carré · 1080 × 1080</option></select></label><div class="two-fields"><label>Largeur<input name="width" type="number" min="256" max="4096" value="${draft.width}" required></label><label>Hauteur<input name="height" type="number" min="256" max="4096" value="${draft.height}" required></label></div>${setting('margin', 'Marges', .02, .25, .01)}${setting('scale', 'Échelle proportionnelle', .1, 1, .01)}${setting('horizontal', 'Position horizontale', 0, 1, .01)}${setting('ground', 'Ligne de sol', .55, .95, .01)}<label class="checkbox"><input name="shadow" type="checkbox" ${draft.shadow ? 'checked' : ''}> Ombre de contact</label>${setting('shadow_opacity', 'Opacité de l’ombre', 0, .4, .01)}${setting('shadow_blur', 'Diffusion de l’ombre', .005, .12, .005)}${setting('shadow_offset', 'Décalage de l’ombre', -.1, .1, .005)}<button class="primary" type="submit" ${disabled(!p.mask_version, 'Un masque est nécessaire.')}>Recomposer</button><p class="help">Aucun appel API. Un nouveau rendu devra être approuvé.</p></form></section><section><p class="eyebrow">03 / EXPORTER</p><form id="export"><label>Fichier<select id="export-format"><option value="webp">Composition WebP</option><option value="jpeg">Composition JPEG</option><option value="cutout" ${disabled(!p.mask_version)}>Détourage PNG transparent</option><option value="mask" ${disabled(!p.mask_version)}>Masque PNG 8 bits</option><option value="original">Original inchangé</option></select></label><label>Qualité JPEG / WebP<input id="export-quality" type="number" min="1" max="100" value="90" required></label><button id="export-button" class="primary" type="submit" ${disabled(!canExport, 'Approuvez le rendu courant ou choisissez Original inchangé.')}>Choisir le dossier et exporter</button><p class="help">${p.render_key ? `${draft.width} × ${draft.height} px. ` : ''}Aucun fichier existant n’est écrasé.</p></form></section></aside></div>`;
}
function settings(): string {
  const pin = current.settings.provider;
  return `<div class="settings-grid"><section class="panel"><p class="eyebrow">FOURNISSEUR DE DÉTOURAGE</p><h2>inference.sh / BiRefNet</h2><p>Les appels partent du backend Rust. Le fond, le cadrage et l’export restent locaux.</p><div class="notice">Tarif actuel, limites du service et conditions de conservation à vérifier auprès du fournisseur avant usage commercial. Aucun coût fixe garanti.</div><h3>Clé API ${badge(keyPresent ? 'Enregistrée dans Windows' : 'Absente', keyPresent)}</h3><form id="key-form"><label>Nouvelle clé<input id="api-key" type="password" autocomplete="off" spellcheck="false" placeholder="Saisie locale uniquement" required ${disabled(!desktop || !current.windows)}></label><button type="submit" class="primary" ${disabled(!desktop || !current.windows)}>Enregistrer dans Windows</button></form>${button('Supprimer la clé enregistrée', 'delete-key', true, !keyPresent)}<p class="help">La clé ne sera jamais réaffichée. Elle n’est stockée ni dans SQLite, ni dans les exports.</p>${!current.windows ? '<p class="help">Le stockage sécurisé de cette version est réservé à Windows.</p>' : ''}<hr><h3>Version du modèle</h3><p>${pin ? `Version épinglée : <code>${h(pin.version_id)}</code>` : 'Aucune version épinglée.'}</p>${button('Vérifier et épingler BiRefNet', 'pin', false, !keyPresent, 'Enregistrez d’abord votre clé API.')}<p class="help">Consulte le schéma et la version, sans soumettre de photo. Une nouvelle version ne réutilise pas silencieusement les masques d’une autre.</p></section><div><section class="panel"><p class="eyebrow">PHOTOTHÈQUE LOCALE</p><h2>Vos originaux sont conservés</h2><p class="path">${h(current.library_location || 'Emplacement Windows standard, disponible dans l’application.')}</p><p class="help">La sélection d’un autre emplacement et la sauvegarde intégrée sont prévues en phase 3. Le guide décrit la sauvegarde manuelle à application fermée.</p></section><section class="panel"><h2>Limites d’import</h2><form id="limits"><label>Taille maximale par fichier (Mio)<input name="max_file_mib" type="number" min="1" max="64" value="${current.settings.limits.max_file_mib}" required></label><label>Résolution maximale (mégapixels)<input name="max_megapixels" type="number" min="1" max="40" value="${current.settings.limits.max_megapixels}" required></label><button class="primary" type="submit" ${disabled(!desktop)}>Enregistrer les limites</button></form><p class="help">Une photo traitée à la fois. Aucune IA locale, aucune télémétrie.</p></section></div></div>`;
}
function jobs(): string {
  return `<p class="intro">Les soumissions sont conservées localement. Un envoi incertain n’est jamais renouvelé automatiquement.</p>${current.submissions.length ? `<div class="table-wrap"><table><thead><tr><th>Photo</th><th>État</th><th>Identifiant distant</th><th>Version</th><th>Date</th></tr></thead><tbody>${current.submissions.map(s => `<tr><td><button class="text-button" data-photo="${h(s.photo_id)}">${h(current.photos.find(p => p.id === s.photo_id)?.name ?? 'Photo')}</button></td><td>${badge(stateLabel(s.state))}</td><td><code>${h(s.remote_id ?? 'Non reçu')}</code></td><td><code>${h(s.version_id)}</code></td><td>${h(new Date(s.created_at).toLocaleString('fr-FR'))}</td></tr>`).join('')}</tbody></table></div>` : '<section class="panel"><h2>Aucun détourage soumis</h2><p>L’import et le classement ne déclenchent aucun appel facturable.</p></section>'}`;
}
function backgrounds(): string {
  return `<p class="intro">Deux fonds déterministes, créés localement à partir de dégradés. Aucun décor généré par IA, aucune image tierce.</p><div class="background-grid">${[['STUDIO_CLAIR', 'Studio clair', 'light'], ['STUDIO_FONCE', 'Studio foncé', 'dark']].map(([id, label, css]) => `<article class="background-card"><div class="background-swatch ${css}"><span>PUJOL AUTO / ${label.toUpperCase()}</span></div><div><h2>${label}</h2>${badge('Version immuable 1')}<p>Référence 1600 × 1200 · ligne de sol 88 %.<br>Zone utile : marges 8 %. Ombre facultative.</p><button class="primary" data-background="${id}" ${disabled(!photo()?.mask_version, 'Sélectionnez d’abord une photo avec un masque.')}>Utiliser pour la photo sélectionnée</button></div></article>`).join('')}</div><p class="help">Les fonds personnalisés et l’éditeur de masque seront ajoutés en phase 3.</p>`;
}
function render(): void {
  const content = view === 'home' ? home() : view === 'library' ? `<div class="section-head"><p>${current.photos.length} photo(s) · JPEG, PNG, WebP</p><span class="muted">Classement manuel · Aucun envoi automatique</span></div>${current.photos.length ? cards(current.photos) : empty('Une photothèque encore vierge', 'Vos fichiers originaux sont copiés sans modification et identifiés par SHA256.')}` : view === 'workshop' ? workshop() : view === 'jobs' ? jobs() : view === 'backgrounds' ? backgrounds() : settings();
  shell(content);
  if (view === 'workshop' && photo()) paintImages();
  for (const img of root().querySelectorAll<HTMLImageElement>('[data-thumb]')) {
    void call<string>('preview', { id: img.dataset.thumb, kind: 'original', thumbnail: true }).then(src => { if (img.isConnected) img.src = src; }).catch(() => { if (img.isConnected) img.alt = 'Aperçu indisponible'; });
  }
}
function notify(message: string, error = false): void {
  const el = document.querySelector<HTMLDivElement>('#toast')!;
  el.textContent = message; el.className = error ? 'visible error-toast' : 'visible';
  setTimeout(() => el.classList.remove('visible'), 6500);
}
async function refresh(): Promise<void> { current = await call<Snapshot>('snapshot'); }
function working(on: boolean): void {
  if (on) {
    for (const button of root().querySelectorAll<HTMLButtonElement>('button:not([data-view]):not([data-photo]):not([data-asset])')) {
      if (!button.disabled) { button.dataset.working = 'true'; button.disabled = true; }
    }
  } else {
    for (const button of root().querySelectorAll<HTMLButtonElement>('[data-working]')) { button.disabled = false; delete button.dataset.working; }
  }
}
async function act(action: () => Promise<void>): Promise<void> {
  if (busy) return;
  busy = true; working(true); document.querySelector('#operation')?.removeAttribute('hidden');
  try { await action(); } catch (e) { notify(String(e instanceof Error ? e.message : e), true); }
  finally { busy = false; working(false); document.querySelector('#operation')?.setAttribute('hidden', ''); if (view === 'workshop') { updateExport(); const approve = document.querySelector<HTMLButtonElement>('[data-action=approve]'); if (approve) approve.disabled = dirty || !photo()?.render_key || photo()?.technical !== 'COMPLETED'; } }
}
async function selectPhoto(id: string): Promise<void> {
  selected = id; draft = { ...(photo()?.settings ?? defaults()) }; dirty = false;
  asset = photo()?.render_key ? 'compare' : photo()?.mask_version ? 'cutout' : 'original'; zoom = 1;
  view = 'workshop'; original = ''; cutout = ''; rendered = ''; render(); await loadImages();
}
async function loadImages(): Promise<void> {
  const p = photo(); if (!p) return;
  const version = ++requestVersion;
  try {
    const before = await call<string>('preview', { id: p.id, kind: 'original', thumbnail: false });
    const transparent = p.mask_version ? await call<string>('preview', { id: p.id, kind: 'cutout', thumbnail: false }) : '';
    const after = p.render_key ? await call<string>('preview', { id: p.id, kind: 'render', thumbnail: false }) : '';
    if (version !== requestVersion || selected !== p.id) return;
    original = before; cutout = transparent; rendered = after; paintImages();
  } catch (e) { notify(String(e), true); }
}
function paintImages(): void {
  const base = document.querySelector<HTMLImageElement>('#base-image');
  const after = document.querySelector<HTMLImageElement>('#after-image');
  if (base) { const src = asset === 'cutout' ? cutout : asset === 'render' ? rendered : original; if (src) base.src = src; }
  if (after && asset === 'compare' && rendered) { after.src = rendered; after.style.clipPath = 'inset(0 0 0 50%)'; }
}
function readDraft(): void {
  const form = document.querySelector<HTMLFormElement>('#composition')!;
  const data = new FormData(form);
  for (const key of ['width', 'height', 'margin', 'scale', 'horizontal', 'ground', 'shadow_opacity', 'shadow_blur', 'shadow_offset'] as const) draft[key] = Number(data.get(key));
  draft.background = String(data.get('background')); draft.shadow = data.has('shadow');
  dirty = JSON.stringify(draft) !== JSON.stringify(photo()?.settings);
  document.querySelector('#dirty-note')?.toggleAttribute('hidden', !dirty);
  const approve = document.querySelector<HTMLButtonElement>('[data-action=approve]');
  if (approve) approve.disabled = dirty || !photo()?.render_key || photo()?.technical !== 'COMPLETED';
  updateExport();
}
function updateExport(): void {
  const select = document.querySelector<HTMLSelectElement>('#export-format');
  const btn = document.querySelector<HTMLButtonElement>('#export-button');
  if (btn) btn.disabled = (select?.value !== 'original' && (photo()?.review !== 'APPROVED' || photo()?.technical !== 'COMPLETED' || dirty));
}
async function prepare(): Promise<void> {
  const p = photo(); if (!p) return;
  const plan = await call<Plan>('plan', { id: p.id });
  const dialog = document.querySelector<HTMLDialogElement>('#confirmation')!;
  dialog.innerHTML = `<p class="eyebrow">CONFIRMATION AVANT ENVOI</p><h2 id="confirm-title">Détourage de cette photo</h2><dl><dt>Photos sélectionnées</dt><dd>${plan.selected}</dd><dt>Appels nécessaires</dt><dd>${plan.calls}</dd><dt>Masques réutilisables</dt><dd>${plan.cached}</dd><dt>Fournisseur / modèle</dt><dd>${h(plan.provider)} / ${h(plan.model)}</dd><dt>Version épinglée</dt><dd><code>${h(plan.pin?.version_id ?? 'Non configurée')}</code></dd></dl><p class="notice">${h(plan.cost_notice)}</p>${plan.calls > 0 && (!plan.pin || !keyPresent) ? '<p class="error">Enregistrez une clé API et épinglez le modèle dans Paramètres avant tout envoi.</p>' : ''}<p>L’original restera conservé. Seul le PNG orienté sera envoyé au fournisseur. Vérifiez ses conditions de confidentialité avant l’envoi.</p><div class="dialog-actions"><button id="cancel-confirm" class="secondary">Annuler</button><button id="send-confirm" class="primary" ${disabled(plan.calls > 0 && (!plan.pin || !keyPresent))}>${plan.calls ? 'Confirmer 1 soumission à l’API' : 'Réutiliser le masque, sans appel API'}</button></div>`;
  dialog.querySelector('#cancel-confirm')!.addEventListener('click', () => dialog.close());
  dialog.querySelector('#send-confirm')!.addEventListener('click', () => {
    dialog.close();
    void act(async () => {
      if (plan.calls) await call('submit_photo', { id: p.id, expectedVersion: plan.pin!.version_id, confirmed: true });
      await refresh(); render(); notify(plan.calls ? 'Soumission enregistrée. Le traitement distant peut prendre du temps.' : 'Masque disponible : composez avec le fond de votre choix.');
    });
  });
  dialog.showModal();
}
async function synchronize(): Promise<void> {
  const p = photo(); if (!p) return;
  const remoteId = p.remote_id ? null : window.prompt('Identifiant du traitement retrouvé dans votre compte inference.sh. Aucun nouvel envoi ne sera effectué.');
  if (!p.remote_id && !remoteId) return;
  await call('sync_photo', { id: p.id, remoteId }); await refresh();
  if (photo()?.mask_version && asset === 'original') asset = 'cutout';
  render(); await loadImages(); notify('État du traitement actualisé.');
}
function bind(): void {
  for (const el of root().querySelectorAll<HTMLElement>('[data-view]')) el.addEventListener('click', e => { e.preventDefault(); view = el.dataset.view as View; render(); });
  for (const el of root().querySelectorAll<HTMLElement>('[data-photo]')) el.addEventListener('click', () => { void selectPhoto(el.dataset.photo!); });
  for (const el of root().querySelectorAll<HTMLElement>('[data-asset]')) el.addEventListener('click', () => { asset = el.dataset.asset as Asset | 'compare'; render(); });
  for (const el of root().querySelectorAll<HTMLElement>('[data-background]')) el.addEventListener('click', () => { draft.background = el.dataset.background!; dirty = true; view = 'workshop'; render(); });
  for (const el of root().querySelectorAll<HTMLElement>('[data-action]')) el.addEventListener('click', () => { void act(async () => {
    switch (el.dataset.action) {
      case 'import': { const p = await call<Photo | null>('import_photo'); if (p) { await refresh(); await selectPhoto(p.id); } break; }
      case 'library': view = 'library'; render(); break;
      case 'prepare': await prepare(); break;
      case 'sync': await synchronize(); break;
      case 'approve': case 'reject': await call('review_photo', { id: selected, approved: el.dataset.action === 'approve' }); await refresh(); render(); break;
      case 'delete-key': await call('delete_key'); keyPresent = false; render(); notify('Clé supprimée du gestionnaire Windows.'); break;
      case 'pin': await call('pin_provider'); await refresh(); render(); notify('Version du modèle vérifiée et épinglée.'); break;
    }
  }); });
  document.querySelector('#category')?.addEventListener('change', e => { void act(async () => { await call('classify', { id: selected, category: (e.target as HTMLSelectElement).value as Category }); await refresh(); render(); }); });
  document.querySelector('#composition')?.addEventListener('input', e => {
    const input = e.target as HTMLInputElement;
    if (input.type === 'range') input.nextElementSibling!.textContent = input.value;
    readDraft();
  });
  document.querySelector('#composition')?.addEventListener('submit', e => { e.preventDefault(); readDraft(); const id = selected; const requested = { ...draft }; void act(async () => { await call('compose_photo', { id, settings: requested }); await refresh(); if (selected === id) { draft = { ...photo()!.settings! }; dirty = false; asset = 'compare'; } render(); if (view === 'workshop') await loadImages(); notify('Composition locale terminée, sans appel API. Vérifiez puis approuvez.'); }); });
  document.querySelector('#preset')?.addEventListener('change', e => {
    const dimensions: Record<string, [number, number]> = { catalogue: [1600, 1200], social: [1080, 1350], square: [1080, 1080] };
    const d = dimensions[(e.target as HTMLSelectElement).value]; if (!d) return;
    document.querySelector<HTMLInputElement>('[name=width]')!.value = String(d[0]); document.querySelector<HTMLInputElement>('[name=height]')!.value = String(d[1]); readDraft();
  });
  document.querySelector('#compare')?.addEventListener('input', e => { document.querySelector<HTMLImageElement>('#after-image')!.style.clipPath = `inset(0 0 0 ${(e.target as HTMLInputElement).value}%)`; });
  document.querySelector('#zoom')?.addEventListener('input', e => { zoom = Number((e.target as HTMLInputElement).value); document.querySelector<HTMLElement>('.scene')!.style.width = `${zoom * 100}%`; });
  document.querySelector('#export-format')?.addEventListener('change', updateExport);
  document.querySelector('#export')?.addEventListener('submit', e => { e.preventDefault(); const format = document.querySelector<HTMLSelectElement>('#export-format')!.value; const quality = Number(document.querySelector<HTMLInputElement>('#export-quality')!.value); void act(async () => { const path = await call<string | null>('export_photo', { id: selected, format, quality }); if (path) notify(`Fichier exporté : ${path}`); }); });
  document.querySelector('#key-form')?.addEventListener('submit', e => { e.preventDefault(); const input = document.querySelector<HTMLInputElement>('#api-key')!; const key = input.value; input.value = ''; void act(async () => { await call('save_key', { key }); keyPresent = await call<boolean>('key_status'); render(); notify('Clé protégée dans le gestionnaire d’identifiants Windows.'); }); });
  document.querySelector('#limits')?.addEventListener('submit', e => { e.preventDefault(); const data = new FormData(e.target as HTMLFormElement); void act(async () => { await call('save_limits', { limits: { max_file_mib: Number(data.get('max_file_mib')), max_megapixels: Number(data.get('max_megapixels')) } }); await refresh(); render(); notify('Limites enregistrées.'); }); });
}
export async function start(): Promise<void> {
  if (desktop) { try { await refresh(); keyPresent = await call<boolean>('key_status'); } catch (e) { render(); notify(String(e), true); return; } }
  render();
  if (timer) clearInterval(timer);
  timer = setInterval(() => {
    if (!desktop || busy || !current.photos.some(p => p.technical === 'REMOTE_RUNNING')) return;
    void act(async () => {
      const pending = current.photos.find(p => p.technical === 'REMOTE_RUNNING')!;
      await call('sync_photo', { id: pending.id, remoteId: null }); await refresh();
      if (view === 'jobs' || view === 'library' || (view === 'workshop' && selected === pending.id && !dirty)) { render(); if (view === 'workshop') await loadImages(); }
    });
  }, 5000);
}
if (import.meta.env.MODE !== 'test') void start();
