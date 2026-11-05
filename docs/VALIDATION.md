# Protocole de validation

## Vérifications automatisées

Depuis la racine :

```sh
npm ci
npm test
npm run build
cargo fmt --all -- --check
cargo test -p studio-core --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo check -p autostudio-desktop --locked
```

Les deux dernières commandes nécessitent les prérequis graphiques de l’OS. Sur le
cloud Linux de cette session, ils ont été téléchargés depuis les dépôts Debian
signés puis extraits dans `/workspace/.tools/autostudio-sysroot`, sans installation
système. Les variables utilisées pour cette vérification seulement :

```sh
export RUSTUP_HOME=/workspace/.tools/rustup
export CARGO_HOME=/workspace/.tools/cargo
export PATH="$CARGO_HOME/bin:$PATH"
export PKG_CONFIG_SYSROOT_DIR=/workspace/.tools/autostudio-sysroot
export PKG_CONFIG_PATH="$PKG_CONFIG_SYSROOT_DIR/usr/lib/x86_64-linux-gnu/pkgconfig:$PKG_CONFIG_SYSROOT_DIR/usr/share/pkgconfig"
```

Le check compile et valide la configuration/les permissions ; il ne lance pas une
fenêtre et ne valide pas Windows Credential Manager. Le serveur Vite a été ouvert
dans Chromium pour vérifier l’interface web seule, avec import désactivé ; ce n’est
pas une exécution de l’application native. Les 4 tests jsdom injectent un pont IPC
de test ; ils ne constituent pas une preuve du fonctionnement des dialogues Windows.

## Mesure reproductible du cœur

```sh
cargo build -p studio-core --example measure --release --locked
target/release/examples/measure prepare /tmp/autostudio-measure
# Utiliser un dossier library neuf pour mesurer l’import, pas un hit de cache.
target/release/examples/measure run /tmp/autostudio-measure
```

Fixture synthétique RGBA 6000 × 4000 : forme rectangulaire et aplats, sans photo ni
modèle. Un import, puis 30 compositions 1600 × 1200 séquentielles avec ombre et
encodage PNG, pas un traitement d’un lot réel de 30 véhicules. Le masque est
synthétique et n’est jamais présenté comme un détourage.

Mesure du 4 novembre 2026, Linux x86_64, Rust release, hôte disposant d’environ
33 Gio de RAM : import avec initialisation 506 ms ; 30 compositions 6 787 ms ;
durée totale du processus 7,539 s ; maximum RSS `201020 KiB`, soit **196,3 Mio**,
mesuré par `resource.getrusage(RUSAGE_CHILDREN)` dans un processus parent Python
réservé à cette mesure. Le générateur de fixture a été exécuté séparément.
SHA256 du dernier rendu :
`1e8b0d5d5141d9997324fdde23e6f47f8c598803907805a8202b20f516a38bdb`.

Ce résultat ne mesure ni WebView2, ni RAM au repos de l’application, ni vraies photos
haute entropie, ni réseau/API, ni Windows 11. Il ne démontre pas une compatibilité
avec 8 Go. Un seul original est chargé, jamais 30 à la fois. Les transferts réels,
coûts, latence et qualité doivent être mesurés dans une recette autorisée.

## Recette Windows 11 obligatoire avant livraison

1. Construire la CI Windows ou localement ; récupérer l’artefact NSIS de test.
   Vérifier le hash du fichier distribué, son origine et le statut de signature
   réel. Aucun certificat n’est configuré. Ne pas désactiver les protections Windows.
2. Sur un Windows 11 de test avec 8 Go, installer pour l’utilisateur courant,
   vérifier le menu Démarrer, le lancement, puis la désinstallation.
3. Sur un compte/VM de test avec WebView2 absent, vérifier son téléchargement
   officiel et le message utile si Internet est indisponible.
4. Vérifier enregistrement/presence/suppression de clé dans Windows Credential
   Manager, aucune réapparition dans le frontend, SQLite, exports ou logs.
5. Importer des JPEG/PNG/WebP autorisés, EXIF orientations 1–8, RGB/RGBA ; vérifier
   originaux SHA256, profils ICC refusés explicitement et limites de taille/résolution.
6. Exécuter la [recette API volontaire](FOURNISSEUR.md) après vérification du tarif et
   des conditions. Relever ID, requêtes réellement facturables, octets et durées.
7. Fermer pendant le suivi, reprendre le même ID. Vérifier qu’une soumission ambiguë
   bloque un nouvel envoi. Tester les erreurs avec le serveur de test, sans multiplier
   volontairement les appels payants.
8. Recomposer avec deux fonds et plusieurs formats ; vérifier qu’il n’y a aucun
   appel API. Rejeter puis vérifier le refus d’export commercial ; approuver ensuite.
9. Vérifier qualité/dimensions des JPEG/WebP, alpha du PNG, masque gris 8 bits,
   original inchangé et absence d’écrasement. Modifier le cadre : approbation invalidée.
10. Application fermée, sauvegarder toute la photothèque, restaurer une copie cohérente
    puis contrôler fichiers, métadonnées et reprise. Tester une interruption brutale.

## Mesures sur le poste cible

Avec Gestionnaire des tâches ou Process Explorer, mesurer ensemble le processus
AutoStudio et les processus WebView2 enfants : mémoire privée/working set au repos
après 60 secondes, pic durant import d’une vraie photo 24 MP, puis compositions
séquentielles. Noter CPU, mémoire système disponible, version Windows/WebView2,
dimensions et taille de chaque entrée. Mesurer la durée et la réactivité de la
navigation pendant le traitement. Ne pas généraliser une seule image synthétique.

Les lots de 30 photos et leur annulation ne sont pas implémentés dans la phase 1.
À la phase 2, tester 30 originaux distincts sans chargement simultané, erreurs par
image, ordre, reprise, annulation des tâches non commencées et plafond de soumissions.

## Qualité automobile sur photos légales

Utiliser des photos fournies par Pujol Auto ou autorisées, avec plusieurs véhicules,
teintes et éclairages. Inclure vues 3/4, profil, face, arrière, véhicule sombre sur
fond sombre, ciel clair, jantes complexes, ombres et reflets. Conserver les originaux.

Pour chaque résultat, noter séparément : rétroviseurs entiers, antennes conservées,
roues/jantes inchangées, bas de caisse sans coupe, reflets et habitacle conservés
dans les vitres, plaques/accessoires/défauts inchangés, absence de frange, de parasites,
d’étirement et de perspective artificielle. Comparer l’alpha et le rendu avec l’original.
Décision humaine acceptée/refusée avec commentaire ; aucun pourcentage de qualité
inventé. Les photos de cette recette n’ont pas été fournies dans cette session.
