# AutoStudio Desktop · Pujol Auto

Application Windows 11 / Tauri 2 pour les photographies automobiles : détourage
distant, masque conservé et composition locale. Le développement porte sur la
**phase 1 : une photo de bout en bout**. Aucun modèle d’IA local, serveur HTTP
applicatif, Python ou Docker n’est nécessaire sur le poste utilisateur.

## État réel

Le cœur Rust, le connecteur HTTP, SQLite, l’interface française et la configuration
NSIS sont implémentés. **Aucun véritable détourage ni appel payant n’a été exécuté.**
Les tests HTTP emploient exclusivement un serveur local de test ; ils ne valident
pas le connecteur en production. Résultats et limites : [PROGRESS.md](PROGRESS.md).

La CI Windows est préparée, sans release automatique. Aucun installateur Windows
n’a encore été construit et essayé dans cette session ; aucun binaire n’est signé.
Ne pas présenter ce dépôt comme une version de production déjà validée.

## Utilisation sur Windows

À partir d’un installateur validé : installer AutoStudio Desktop, puis le lancer
depuis le menu Démarrer. L’installateur vise le compte courant et prévoit le
téléchargement officiel du runtime WebView2 s’il manque ; une connexion Internet est
alors nécessaire. Windows 11 dispose souvent de WebView2, sans garantie.
L’utilisateur final n’installe ni Rust, ni Node, ni Python, ni Docker.

Le [guide utilisateur](docs/GUIDE_UTILISATEUR.md) décrit le parcours et la sauvegarde.
Le coût fournisseur reste inconnu ; aucun tarif historique n’est présenté comme
une estimation garantie.

## Développement

Prérequis du **développeur Windows** : Node 24.22.1, Rust 1.99.1 (épinglé dans
`rust-toolchain.toml`), Microsoft C++ Build Tools avec la charge « Développement
Desktop en C++ », SDK Windows et WebView2.

Depuis la racine du checkout existant :

```powershell
npm ci
cargo test -p studio-core --locked
npm test
npm run build
npm run desktop
```

Le cœur et ses tests ne nécessitent ni WebView ni clé API :

```sh
cargo test -p studio-core --locked
cargo clippy -p studio-core --all-targets --locked -- -D warnings
cargo fmt --all -- --check
```

Le développement graphique Linux nécessite GTK 3, WebKitGTK 4.1, librsvg et Ayatana
AppIndicator (paquets de développement). La protection des clés utilise exclusivement
**Windows Credential Manager** dans cette version et refuse explicitement leur
enregistrement sur un autre système. Le frontend web seul désactive l’import et
n’invente ni photothèque ni inférence.

## Construire l’installateur

Sur Windows, depuis la racine :

```powershell
npm ci
npm run tauri build -- --ci --bundles nsis -- --locked
```

Résultat attendu : `target/release/bundle/nsis/*.exe`. Le workflow
`.github/workflows/windows.yml` exécute les vérifications et conserve un artefact de
test, sans créer de release. Aucun certificat de signature n’est configuré.
Tester installation, lancement et désinstallation sur Windows 11 avant distribution.
Ne pas désactiver les protections Windows.

## Organisation

- `crates/studio-core` : logique indépendante de Tauri, migrations et tests.
- `src-tauri` : fenêtre, commandes validées, dialogues backend, tâches hors thread
  d’interface, coffre Windows, CSP et permissions.
- `src` : interface TypeScript française, sans framework.
- [ARCHITECTURE.md](ARCHITECTURE.md), [LICENSES.md](LICENSES.md),
  [contrat fournisseur](docs/FOURNISSEUR.md), [recette](docs/VALIDATION.md).

Dépendances verrouillées : `Cargo.lock` et `package-lock.json`. La CI produit uniquement un artefact de test ; elle ne publie aucune release.
