# Licences et conditions

## Code AutoStudio

Aucune licence de redistribution du code métier n’a été choisie par son titulaire.
Les manifests utilisent `LicenseRef-Proprietary` ; cela ne concède pas implicitement
une licence MIT au projet. Définir les droits de distribution avant livraison à
des tiers. Les originaux et fonds personnels restent soumis aux droits de leurs
titulaires. Les deux fonds intégrés et l’icône ont été dessinés par du code dans ce
projet, sans photographie ou décor tiers.

## Dépendances

L’[inventaire verrouillé](docs/DEPENDENCY_LICENSES.md) reprend les licences déclarées
de 483 paquets Rust et 201 entrées npm, toutes plateformes et dépendances de
développement incluses. Les verrous et l’inventaire doivent évoluer ensemble.

Composants principaux : Tauri/API/plugins (MIT ou Apache-2.0 selon paquets),
TypeScript (Apache-2.0), Vite/Vitest (MIT), image (MIT ou Apache-2.0), reqwest (MIT ou
Apache-2.0), rusqlite (MIT), keyring (MIT ou Apache-2.0), sha2 (MIT ou Apache-2.0),
webp et ses bindings (MIT selon manifest ; libwebp natif possède ses propres notices).
SQLite natif est dédié au domaine public par son auteur. Les textes de licence et
notices des composants natifs embarqués doivent accompagner la distribution finale ;
l’inventaire SPDX ne remplace pas ces textes. Aucun installateur n’est publié ici.

WebView2 est un runtime Microsoft soumis à ses propres conditions de redistribution
et d’utilisation. L’installateur prévoit son bootstrapper officiel, sans inclure de
licence Microsoft inventée ni garantir sa présence sur tous les Windows 11.

## BiRefNet et fournisseur

La licence du dépôt amont BiRefNet a été consultée le 4 novembre 2026 :
https://github.com/ZhengPeng7/BiRefNet/blob/main/LICENSE — MIT, copyright 2024 ZhengPeng.
Cela ne prouve ni la version des poids effectivement hébergés par inference.sh ni
les droits applicables à un contrat particulier. Aucun poids n’est téléchargé,
installé ou redistribué par AutoStudio.

Les sources officielles de l’API utilisées sont citées dans
[FOURNISSEUR.md](docs/FOURNISSEUR.md). Elles ne confirment pas les conditions
commerciales, les quotas, la politique de rétention/suppression ou la localisation
du service. Les pages web nécessaires ont été bloquées par la politique réseau de
l’environnement. Ces points restent à vérifier officiellement avant envoi de photos
commerciales. Aucun usage commercial gratuit, tarif historique garanti ou conservation
nulle n’est affirmé.
