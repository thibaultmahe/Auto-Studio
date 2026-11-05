# Guide utilisateur

La phase actuelle traite une photo à la fois. Le vrai détourage et l’installateur
doivent encore passer la recette Windows avant un usage commercial.

## Paramètres

Saisir la clé de votre compte inference.sh dans l’application uniquement, jamais
dans le chat ou une capture. **Enregistrer dans Windows** vide le champ et conserve
la clé dans le gestionnaire d’identifiants. Seule sa présence est affichée.
**Supprimer la clé enregistrée** la retire du gestionnaire.

**Vérifier et épingler BiRefNet** consulte le schéma et la version sans détourage.
La version reste fixe jusqu’à un nouveau clic explicite. Vérifier le tarif, les
limites et les conditions commerciales et de conservation chez le fournisseur :
AutoStudio n’annonce ni coût fixe ni rétention nulle.

## Importer

**Importer une photo** ouvre le sélecteur Windows. JPEG, PNG, WebP : le contenu et
les limites configurées sont vérifiés. L’original est copié sans modification et
la copie de travail respecte EXIF. Une photo identique retrouve sa fiche existante.
L’import ne consomme aucun crédit.

Les profils ICC sont refusés dans cette phase : utiliser une copie correctement
convertie en sRGB sans profil intégré, sans simplement supprimer un profil P3/Adobe RGB.

Dans l’atelier, classer manuellement : **Extérieur** permet la préparation d’un
détourage ; **Habitacle**, **Détail** et **À classer** ne peuvent pas être soumis.
Habitacle et détails restent exportables en original. Import multiple, dossier et
glisser-déposer seront ajoutés en phase 2.

## Détourer

**Préparer le détourage** affiche sélection, appels nécessaires, cache, modèle,
version et coût inconnu. **Annuler** ne soumet rien. **Confirmer 1 soumission à l’API**
autorise cet appel précis. L’original est conservé ; le PNG orienté est envoyé.

L’état et l’identifiant sont conservés dans **Traitements**. Le suivi reprend après
redémarrage. Un masque en cache peut être réutilisé sans envoi ni clé API présente.

Si la soumission est incertaine, retrouver son ID dans l’historique inference.sh
avant de renouveler quoi que ce soit. **Consulter / réconcilier** permet de le
rattacher : l’application vérifie source et version. Sans ID récupérable, la nouvelle
soumission reste bloquée pour éviter une double facturation.

Une consultation échouée peut être relancée sans créer de tâche. Une destination,
dimension, transparence ou réponse invalide exige de vérifier le connecteur : aucun
faux détourage local ne le remplace.

## Composer et contrôler

Choisir **Studio clair** ou **Studio foncé**, le format catalogue/réseaux/carré ou
des dimensions personnalisées. Ajuster marges, échelle proportionnelle, position
horizontale, ligne de sol et ombre. **Recomposer** est entièrement local, sans API.
Une coupe déjà présente dans l’original ne peut pas être réparée par le cadrage.

Les vues Original, Détourage, Composition et Avant / après permettent le contrôle.
Le zoom agrandit un aperçu limité à 1600 pixels, pas l’original pleine résolution.
Les barres de défilement permettent de parcourir l’image agrandie.

Vérifier rétroviseurs, antennes, vitres/reflets, roues et bas de caisse. Les alertes
ne sont pas une note de qualité. Les vitres doivent conserver l’intérieur visible.
L’éditeur de masque est prévu en phase 3 : **Refuser** si le masque est incorrect.
**Approuver** seulement après contrôle. Tout nouveau rendu exige une nouvelle approbation.

## Exporter

Choisir WebP, JPEG, PNG transparent, masque PNG ou original inchangé. Qualité
JPEG/WebP : 1 à 100. Changer les dimensions exige de recomposer puis approuver le
format voulu. Masque et RGBA conservent la résolution source ; l’original conserve
ses octets et son format.

**Choisir le dossier et exporter** ouvre un dialogue Windows. Les noms sont assainis
et incrémentés : aucun écrasement silencieux. Un résultat traité non approuvé ou
rejeté ne peut pas être exporté ; l’original reste exportable.

## Sauvegarde et restauration manuelles

L’emplacement exact est dans Paramètres. Fermer complètement l’application puis
copier **tout** `phototheque`, y compris SQLite et ses éventuels `-wal`/`-shm`, vers
un support de sauvegarde. Copier seulement les originaux ou SQLite ne suffit pas.
La clé n’est pas incluse : elle reste dans Windows.

Pour restaurer : fermer AutoStudio, sauvegarder la photothèque actuelle puis la
remplacer entièrement par **une seule sauvegarde cohérente**. Ne pas fusionner
bases/fichiers de dates différentes. Relancer et contrôler les photos avant tout
nouvel envoi. Sur un autre compte Windows, saisir à nouveau la clé. Tester d’abord
la restauration sur une copie ; la sauvegarde intégrée et le déplacement de la
photothèque sont prévus en phase 3.

Aucune commande de nettoyage automatique n’est fournie. Ne pas supprimer les
originaux, masques ou métadonnées comme un simple cache.
