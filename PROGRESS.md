# Progression · 4 novembre 2026

## Phase 1 implémentée

- Workspace Rust, cœur indépendant, fenêtre/IPC Tauri 2 et interface TypeScript française.
- Import d’une photo, contenu JPEG/PNG/WebP, limites, SHA256/déduplication, octets
  d’origine conservés, EXIF sur copie de travail, miniature et photothèque SQLite.
- Catégories manuelles, extérieur explicitement requis ; intérieur/détail inchangés.
- Clé dans Windows Credential Manager côté Rust, présence seulement vers l’UI,
  buffers applicatifs effacés, aucun secret en base/log/export.
- Connecteur réel fondé sur les sources officielles citées, version résolue et
  épinglée, schéma vérifié avant soumission, upload/suivi/récupération documentés.
- Confirmation avec 0/1 appel, cache, fournisseur/modèle/version et coût inconnu.
  Historique persistant, ID immédiat, refus des doubles soumissions, reprise et
  réconciliation des envois incertains sans resoumission aveugle.
- PNG gris 8 bits et PNG RGBA, alpha validé, RGB de la source conservés, alertes
  simples sans score inventé. Deux fonds locaux versionnés et empreintes conservées.
- Cadrage proportionnel borné, ombre déterministe facultative, réglages persistés,
  cache local de rendu, changement de fond sans appel API.
- Original/détourage/composition/comparateur/zoom ; approbation humaine distincte,
  invalidation après changement, export commercial refusé sans approbation.
- Exports JPEG/WebP qualité réglable, PNG alpha/masque et original exact, dialogue
  natif de destination, noms sûrs et aucun écrasement silencieux.
- Migrations, fichiers immuables, verrou OS, CSP et permissions restreintes.
- Configuration NSIS/WebView2 et workflow Windows sans release automatique.
- README, architecture, guide français, contrat fournisseur, inventaire de licences,
  protocole de recette, benchmark reproductible.

## Exécuté et vérifié dans cette session

- `cargo test -p studio-core --locked` : **15 tests Rust**, sans appel payant.
  Parcours HTTP complet sur serveur de test puis composition/approbation/export,
  contenu/RGB/RGBA/EXIF, masques, géométrie/alpha/ombre, cache/version, exports,
  migrations/verrou/chemins, interruption/réconciliation, 401/402/429/5xx,
  JSON invalide, redirections et timeout sans resoumission.
- `npm test` : **4 tests d’interface jsdom**, avec pont IPC simulé explicitement :
  échappement des noms, confirmation obligatoire, cache sans soumission même sans clé,
  champ de clé effacé et aucun réaffichage.
- `npm run build` : TypeScript et bundle Vite ; pas d’erreur.
- `cargo check -p autostudio-desktop --locked` : couche Tauri complète vérifiée sur
  Linux après préparation d’un sysroot de paquets Debian signés. Aucun prérequis
  graphique système n’a été remplacé ; une première tentative a échoué faute de
  GLib/GTK/WebKit de développement, puis la vérification a réussi avec le sysroot.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` et format Rust.
- Frontend ouvert dans Chromium : accueil/paramètres, aucune erreur JavaScript,
  pas de débordement horizontal à 1440 px, import désactivé hors Tauri.
- Mesure du cœur release : image synthétique 24 MP, import 506 ms, 30 compositions
  séquentielles 6 787 ms, pic RSS **196,3 Mio**. Détails et protocole dans
  [VALIDATION.md](docs/VALIDATION.md). Ce n’est ni un lot réel ni une mesure Windows.

Le test de masque vide a initialement révélé un débordement ; il a été corrigé.
Les résultats finaux ne présentent pas cette tentative échouée comme une réussite.

## Non validé : phase 1 pas encore livrable en production

- **Aucun appel réel à inference.sh**, aucune clé fournie, aucune dépense autorisée
  ou exécutée. Les tests simulés ne prouvent ni détourage ni qualité de BiRefNet.
- **Aucun installateur Windows construit/exécuté**, aucune CI distante lancée,
  aucun essai réel du coffre Windows et aucun binaire signé. Le check Linux n’en
  constitue pas une preuve. La fenêtre native n’a pas été lancée dans cette session.
- Tarif actuel, limites réelles du service, hôtes présignés, version effective et
  modalités commerciales/rétention restent à vérifier officiellement. Pages web
  bloquées par le proxy ; connecteur fermé en cas de schéma/destination inattendus.
- **Compatibilité 8 Go non démontrée** : pas de mesure de la fenêtre/WebView2 au
  repos, de vraies photos sur Windows, ni de latence/coût/transfert du fournisseur.
- Profils ICC refusés explicitement ; pas de gestion colorimétrique complète.
  Images animées hors protocole validé. Pas de photos automobiles réelles fournies
  pour évaluer vitres, antennes, rétroviseurs et reflets.
- Sauvegarde/restauration manuelle documentée, non recettée sur Windows ; pas de
  sauvegarde intégrée, déplacement de photothèque ou nettoyage en un clic.

## Étapes suivantes

1. Lever les inconnues du fournisseur et tester une photo réelle depuis l’application,
   après vérification du tarif et confirmation explicite de la dépense par l’utilisateur.
2. Exécuter la CI Windows et la recette installateur/coffre/WebView2/8 Go/qualité.
   Le workflow est enregistré mais n’a pas été déclenché ni publié.
3. Phase 2 : import multiple/dossier/glisser-déposer, véhicules, classement multiple,
   file locale multi-photo, annulation des tâches non commencées et reprise par image.
4. Phase 3 : éditeur de masque avec annuler/rétablir, fonds personnels, bibliothèque
   déplaçable, sauvegarde/restauration intégrée et exports par lots approuvés.

Au 4 novembre, les fichiers étaient uniquement locaux, sans commit/push de cette
phase, sans release ni publication d’environnement. Seul l’ajout réseau
`inference.sh`/`api.inference.sh` a été sauvegardé au brouillon, sans activation.

## Publication du code demandée le 5 novembre 2026

L’utilisateur a autorisé le commit et le push sur le dépôt existant. Le workflow
Windows est configuré pour démarrer au push sur `main` ; son résultat reste à
vérifier dans GitHub Actions. Cette opération ne publie ni release ni environnement
cloud et ne lance aucun détourage payant. Les limites de validation ci-dessus
restent applicables.
