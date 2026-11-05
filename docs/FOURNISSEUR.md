# Contrat inference.sh / BiRefNet

## Sources officielles consultées le 4 novembre 2026

- https://github.com/inference-sh/sdk-js, révision
  `2a303bf342ab66f622f8833953d859161874302f` : `src/http/client.ts`,
  `src/api/apps.ts`, `files.ts`, `tasks.ts`, `src/types.ts`.
- Documentation du modèle :
  https://github.com/inference-sh/skills/blob/0cbf91c26ada28702c79962406286f0c14292150/tools/image/background-removal/SKILL.md
  Elle documente `infsh/birefnet`, l’entrée `image` et un PNG transparent. Les
  suggestions de génération par prompt du même document ne sont pas utilisées.

Les pages https://inference.sh/apps/infsh/birefnet et `/docs` ont été bloquées par le
proxy de l’environnement (CONNECT 403). `inference.sh` et `api.inference.sh` ont été
enregistrés au **brouillon réseau**, sans application ni publication. Une éventuelle
publication de l’environnement relève de l’utilisateur ; aucune n’a été exécutée.

## Contrat implémenté

Origine `https://api.inference.sh`, Bearer côté Rust, enveloppe `data` v3 selon SDK.

| Opération | Contrat officiel | Contrôle |
| --- | --- | --- |
| Modèle | `GET /apps/infsh/birefnet` | namespace/nom, app ID, version ID égale à `version.id`, schéma `image:string`, autres paramètres obligatoires refusés |
| Fichier | `POST /files`, `files:[{uri:"",filename,content_type,size}]` | exactement un fichier, URI et URL d’upload présentes |
| Upload | `PUT upload_url`, corps PNG, type `image/png` | hôte/HTTPS/port autorisés, sans clé API de l’application |
| Soumission | `POST /apps/run`, `app_id`, `version_id`, `input:{image:uri}`, `wait:false`, fonction si déclarée | état durable préalable, pas de retry, ID conservé immédiatement |
| Suivi | `GET /tasks/{id}` | ID, statut, application/version/URI source concordants |
| Fichier résultat | `GET /tasks/{id}/files?role=output` | exactement un fichier `image/png` |
| Téléchargement | `GET` de son URI HTTPS | liste d’hôtes fermée, taille bornée, pas de redirection ni clé API |

La version réelle est résolue lors du clic « Vérifier et épingler », puis enregistrée
et transmise explicitement. Aucun ID inventé ou `latest` silencieux. Le schéma de la
fonction par défaut est utilisé si défini. Un changement incompatible bloque l’envoi.
La structure métier de `output` n’est pas supposée : la liste officielle des fichiers
attachés est utilisée. Les URI non HTTPS ou d’un hôte autre que `cloud.inference.sh`
sont refusées jusqu’à vérification officielle d’un éventuel autre stockage présigné.

Statuts du `types.ts` cité : 1 reçu, 2 file, 3 dispatch, 4 préparation, 5 service,
6 setup, 7 exécution, 8 annulation, 9 upload, **10 terminé**, 11 échec, 12 annulé,
13 refusé. D’anciens exemples README emploient encore 9 comme terminé ; le connecteur
suit les constantes typées de la révision consultée.

## Inconnues explicites

- Version effective et schémas d’un compte réel : résolution implémentée, non exécutée ici.
- Formats, poids/résolution maximum et quotas du **service** : non vérifiés. Les limites
  locales ne garantissent pas l’acceptation ; la source n’est pas réduite en secret.
- Tarif, coût réel et latence : aucun montant fixe, budget bloquant ou délai garanti.
  Aucun champ de coût fiable confirmé, donc aucun coût réel inventé en base.
- Hôtes réellement utilisés par les URL présignées et forme des URI de résultat.
- Conditions commerciales, rétention, suppression, localisation et confidentialité.
  Aucune licence commerciale incluse ni rétention nulle n’est promise.
- Idempotence distante : aucun en-tête ni endpoint de recherche supposé. L’état local
  bloque les reprises ambiguës et permet le rattachement d’un ID retrouvé par l’utilisateur.

Ces points doivent être vérifiés officiellement avant production. Les tests locaux
ne les valident pas.

## Tests sans coût

`cargo test -p studio-core --locked` vérifie le contrat avec un serveur HTTP loopback
réservé aux tests : résolution, upload, une soumission simulée, suivi, récupération,
puis vrai pipeline local de masque/composition/approbation/export. Les erreurs
401/402/429/5xx, réponses invalides, doublons, reprise et destinations inattendues sont
testées. Les clés sont des chaînes de test sans valeur réelle. Aucun simulateur
n’est accessible dans l’application et aucun appel payant n’a été exécuté.

## Test réel volontaire, désactivé par défaut

Aucun test automatisé ne lit de clé d’environnement ou ne soumet au fournisseur.
La recette passe uniquement par l’application Windows :

1. Vérifier auprès du fournisseur les inconnues ci-dessus, notamment tarif et
   confidentialité. Utiliser une photo autorisée et limiter l’essai à une photo.
2. Saisir la clé dans l’application, jamais dans le terminal ou le chat.
3. Épingler le modèle et contrôler sa version dans le récapitulatif.
4. Lire le coût inconnu et le tarif vérifié auprès du fournisseur ; ne confirmer que
   si cette dépense convient. Le clic autorise cet appel potentiellement payant.
5. Noter ID, nombre de soumissions dans le compte, durée, dimensions et qualité.
6. Changer le fond : vérifier zéro soumission supplémentaire.
7. Fermer pendant le suivi puis relancer : vérifier la reprise du même ID, sans
   multiplier volontairement des appels payants pour provoquer des erreurs.

Ne considérer le connecteur validé en production qu’après cette recette et la levée
des inconnues. Aucune autorisation d’appel payant n’a été donnée dans cette session.
