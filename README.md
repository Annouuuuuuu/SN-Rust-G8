# FileSentinel

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey.svg)]()

> Système de surveillance et synchronisation de répertoires en temps réel écrit en Rust

**FileSentinel** est un outil de surveillance de fichiers qui détecte les changements (création, modification, suppression) et synchronise automatiquement vers un répertoire de destination. Inspiré par `rsync` et `inotifywait`, il offre des fonctionnalités avancées de versionnement, compression, et notifications desktop.

## Table de matière

- [Fonctionnalités](#-fonctionnalités)
- [Installation](#-installation)
- [Démarrage rapide](#-démarrage-rapide)
- [Commandes](#-commandes)
- [Dashboard TUI](#-dashboard-tui)
- [Configuration](#-configuration)
- [Workflows](#-workflows)
- [Architecture](#-architecture)
- [Développement](#-développement)
- [Dépannage & FAQ](#-dépannage--faq)
- [Cas d'utilisation](#-cas-dutilisation)
- [Roadmap](#-roadmap)
- [Licence](#-licence)

## Fonctionnalités

### Surveillance
- **Polling** : Analyse récursive des répertoires avec comparaison d'état (hash MD5)
- **Inotify** (Linux) : Détection native et instantanée des changements
- **Cross-platform** : Fonctionne sur Windows, Linux et macOS via `notify`

### Synchronisation
- Synchronisation incrémentielle (différentielle)
- Gestion des créations, modifications et suppressions
- Détection des conflits
- Support de compression pour les transferts

### Versionnement
- Sauvegarde automatique des versions de fichiers
- Nombre de versions configurable
- Restauration à une version spécifique
- Nettoyage automatique des anciennes versions

### Filtrage
- Patterns glob d'exclusion/inclusion
- Filtrage par taille de fichier
- Filtrage par extension
- Règles configurables

### Notifications
- Notifications desktop natives
- Regroupement par lots
- Filtrage des fichiers critiques
- Intervalle de notification configurable

### Dashboard TUI interactif
- Interface en terminal (TUI) ultra-lisible avec **ratatui**
- **6 onglets** : Dashboard, Surveillance, Synchronisation, Versions, Configuration, Règles
- Surveillance et synchronisation contrôlables depuis l'interface sans quitter le terminal
- Gestion complète des versions : recherche, restauration, suppression, nettoyage
- Flux d'événements en temps réel avec horodatage
- Ajout/suppression de dossiers surveillés à chaud
- Modification de la destination de sync sans redémarrage
- Annulation de la synchronisation en cours

### Réseau
- Synchronisation via SSH/rsync
- Support des clés SSH
- Options rsync personnalisables
- Synchronisation bidirectionnelle

### Compression
- Compression GZIP à la volée
- Niveau de compression configurable
- Seuil de taille minimum

## Installation

### Option 1 — Binaire précompilé (recommandé, aucun prérequis)

Téléchargez la dernière version depuis la [page Releases](https://github.com/Annouuuuuuu/SN-Rust-G8/releases), extrayez l'archive et lancez le script d'installation correspondant à votre système.

**Windows**

```
filesentinel-windows.zip
├── filesentinel.exe
├── install.ps1
└── uninstall.ps1
```

Clic droit sur `install.ps1` → **Exécuter avec PowerShell** (en tant qu'administrateur).

> Si Windows bloque le script, ouvrez PowerShell en administrateur et exécutez d'abord :
> ```powershell
> Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned
> ```

**Linux**

```
filesentinel-linux.tar.gz
├── filesentinel
├── install.sh
└── uninstall.sh
```

```bash
chmod +x install.sh
./install.sh
```

---

### Option 2 — Depuis les sources (nécessite Rust 1.70+)

```bash
# Cloner le dépôt
git clone https://github.com/Annouuuuuuu/SN-Rust-G8
cd SN-Rust-G8

# Compiler et installer globalement
cargo install --path .
```

> **Note Linux :** avant de compiler, activez le watcher natif inotify en décommentant les fonctions Linux dans `src/config/settings.rs` et en commentant les fonctions Windows correspondantes (instructions détaillées dans le fichier).

---

### Dépendances optionnelles

- **rsync** et **SSH** : uniquement nécessaires pour la synchronisation réseau distante

---

### Vérification de l'installation

```bash
filesentinel --version
filesentinel --help
```

## Démarrage rapide

Après l'installation, voici les étapes pour démarrer :

```bash
# 1. Générer la configuration par défaut
filesentinel init

# 2. Ouvrir le dashboard interactif (recommandé)
filesentinel dashboard
# → tout se gère depuis l'interface : surveillance, sync, versions…

# ─── Ou en ligne de commande ──────────────────────────────────────

# 3. Première synchronisation
mkdir mon_projet
echo "Hello FileSentinel" > mon_projet/test.txt
filesentinel sync --source ./mon_projet --dest ./backup

# 4. Démarrer la surveillance en temps réel
filesentinel watch --directories ./mon_projet
```

Pour plus de détails sur chaque commande, voir la section [Commandes](#-commandes) ci-dessous.  
Pour l'interface graphique en terminal, voir la section [Dashboard TUI](#-dashboard-tui).

## Commandes

### `dashboard` - Interface TUI interactive

Ouvre le tableau de bord interactif en terminal. **C'est le mode recommandé** : toutes les opérations sont accessibles depuis une seule interface sans avoir à taper de commandes.

```bash
filesentinel dashboard
```

> Toutes les autres commandes CLI restent disponibles et fonctionnent indépendamment du dashboard.

---

### `init` - Initialisation

Génère un fichier de configuration par défaut.

```bash
# Configuration par défaut
filesentinel init

# Avec un nom personnalisé
filesentinel --config production.toml init
```

### `watch` - Surveillance

Démarre la surveillance des répertoires en temps réel. (Synchronisation automatique)

```bash
# Utiliser les répertoires de la configuration
filesentinel watch

# Surveiller un répertoire spécifique
filesentinel watch -d ./mon_dossier

# Surveiller plusieurs répertoires
filesentinel watch -d ./projet1 -d ./projet2

# Mode verbeux avec plus de détails
filesentinel --verbose watch -d ./src

# Avec fichier de configuration personnalisé
filesentinel --config prod.toml watch
```

### `sync` - Synchronisation manuelle

Effectue une synchronisation complète entre deux répertoires.

```bash
# Sync avec les paramètres par défaut
filesentinel sync

# Sync avec source et destination spécifiques
filesentinel sync --source ./source --dest ./backup

# Version courte
filesentinel sync -s ./src -d ./dst

# Avec logs détaillés
filesentinel -v sync -s ./projet -d ./sauvegarde
```

### `version-history` - Historique des versions

Affiche l'historique des versions sauvegardées d'un fichier.

```bash
# Voir l'historique
filesentinel version-history ./mon_fichier.txt

# Fichier dans un sous-dossier
filesentinel version-history ./docs/rapport.pdf
```

### `restore` - Restauration de version

Restaure une version spécifique d'un fichier.

```bash
# Restaurer la version 1
filesentinel restore ./mon_fichier.txt --version 1

# Restaurer une version spécifique
filesentinel restore ./config.json -v 3
```

### `show-config` - Affichage de la configuration

Affiche la configuration actuelle complète.

```bash
# Configuration par défaut
filesentinel show-config

# Configuration personnalisée
filesentinel --config custom.toml show-config
```

### `stats` - Statistiques

Affiche les statistiques de versionnement.

```bash
# Statistiques par défaut (24h)
filesentinel stats

# Période personnalisée
filesentinel stats --period 1h
filesentinel stats --period 7d
filesentinel stats -p 30m
```

### `rules` - Règles de filtrage

Liste les règles de filtrage actives.

```bash
filesentinel rules
```

### `network-sync` - Synchronisation réseau

Synchronise via SSH/rsync vers ou depuis un serveur distant.

```bash
# Envoyer vers le serveur
filesentinel network-sync to-remote

# Télécharger depuis le serveur
filesentinel network-sync from-remote
```

### Options globales

| Option | Raccourci | Description | Défaut |
|--------|-----------|-------------|--------|
| `--config` | `-c` | Fichier de configuration | `config.toml` |
| `--verbose` | `-v` | Mode verbeux (logs détaillés) | Désactivé |
| `--help` | `-h` | Affiche l'aide | - |
| `--version` | `-V` | Affiche la version | - |

## Dashboard TUI

Lancé avec `filesentinel dashboard`, le dashboard offre une interface en terminal complète construite avec [ratatui](https://github.com/ratatui/ratatui).

### Navigation entre onglets

| Touche | Action |
|--------|--------|
| `Tab` / `Shift+Tab` | Onglet suivant / précédent |
| `1` à `6` | Aller directement à un onglet |
| `Q` ou `Ctrl+C` | Quitter |

### Onglet 1 — Dashboard

Vue d'ensemble de l'état du système : statut de la surveillance, dossiers configurés, statistiques de la dernière synchronisation et flux des derniers événements détectés.

| Touche | Action |
|--------|--------|
| `W` | Démarrer / arrêter la surveillance |
| `S` | Lancer une synchronisation |
| `↑` / `↓` ou `k` / `j` | Naviguer dans la liste d'événements |

### Onglet 2 — Surveillance

Gestion des dossiers surveillés et flux d'événements en temps réel.

| Touche | Action |
|--------|--------|
| `W` | Démarrer / arrêter la surveillance |
| `A` | Ajouter un dossier à surveiller |
| `D` ou `Suppr` | Retirer le dossier sélectionné |
| `S` | Lancer une synchronisation |
| `↑` / `↓` | Sélectionner un dossier dans la liste |

> Le flux d'événements en bas de l'écran se met à jour en temps réel : chaque fichier créé (`+`), modifié (`~`) ou supprimé (`✗`) apparaît avec son horodatage.

### Onglet 3 — Synchronisation

Contrôle de la synchronisation et affichage des résultats.

| Touche | Action |
|--------|--------|
| `S` | Lancer la synchronisation complète |
| `X` | Annuler la synchronisation en cours |
| `E` | Modifier la destination de synchronisation |

> Le résultat de la dernière sync (fichiers copiés, créés, supprimés, durée, erreurs) s'affiche à droite.

### Onglet 4 — Versions

Gestion de l'historique des versions de fichiers.

| Touche | Action |
|--------|--------|
| `E` ou `Entrée` | Chercher les versions d'un fichier (saisir le chemin) |
| `↑` / `↓` | Sélectionner une version dans la liste |
| `R` | Restaurer la version sélectionnée à son emplacement d'origine |
| `F` | Restaurer la version dans un dossier au choix |
| `D` ou `Suppr` | Supprimer la version sélectionnée |
| `C` | Nettoyer : conserver seulement N versions (saisir N) |

### Onglet 5 — Configuration

Affichage en lecture seule de la configuration active (issue de `config.toml`). Les modifications de dossiers et de destination effectuées dans les autres onglets sont sauvegardées automatiquement dans `config.toml`.

### Onglet 6 — Règles

Affichage des patterns d'exclusion actifs, de la taille maximum de fichier, des extensions incluses et des patterns critiques pour les notifications.

### Saisie de texte (overlay)

Quand une action nécessite une saisie (ajouter un dossier, modifier la destination, chercher un fichier…), un panneau de saisie apparaît par-dessus l'interface :

| Touche | Action |
|--------|--------|
| Caractères | Saisir le texte |
| `⌫` Backspace | Effacer le dernier caractère |
| `Entrée` | Confirmer |
| `Échap` | Annuler |

---

## Configuration

### Fichier `config.toml`

Généré automatiquement par `filesentinel init` avec des valeurs par défaut prêtes à l'emploi. Modifiez les champs selon vos besoins — en particulier la section `[network]` si vous souhaitez utiliser la synchronisation distante.

```toml
[watch]
# Répertoires à surveiller
directories = ["./test_source"]
# Intervalle de polling (ms)
polling_interval_ms = 1000

[sync]
# Répertoire de destination
destination = "./sync_dest"
# Sauvegardes avant sync
create_backups = false
# Opérations simultanées
max_concurrent_operations = 4

[filters]
# Patterns d'exclusion
exclude_patterns = [
    "**/.git/**",
    "**/node_modules/**",
    "**/target/**",
    "**/*.tmp",
    "**/*.swp"
]
# Taille max en MB
max_file_size_mb = 100
# Extensions à surveiller (vide = toutes)
include_extensions = []

[reporting]
# Afficher la progression
show_progress = true
# Fichier de logs
# log_file = "filesentinel.log"

[versioning]
# Activer le versionnement
enabled = true
# Versions max par fichier
max_versions = 5
# Dossier de stockage
versions_dir = ".versions"
# Versionnement automatique
auto_version_on_change = true

[compression]
# Activer la compression
enabled = true
# Niveau (1-9)
level = 6
# Taille minimum pour compression (octets)
min_file_size_for_compression = 1024

[notifications]
# Activer les notifications
enabled = true
# Résumé par lots
show_batch_summary = true
# Intervalle minimum (secondes)
min_interval_seconds = 5
# Patterns de fichiers critiques
critical_patterns = [
    "*.conf",
    "*.env",
    "*.toml",
    "*.lock"
]

# Configuration réseau (décommentez et adaptez pour activer)
# La section [network] est générée par défaut avec des valeurs placeholder.
# Modifiez les champs ci-dessous avec vos informations de connexion.
[network]
# Adresse du serveur distant
host = "mon-serveur.example.com"
# Port SSH (22 par défaut)
port = 22
# Nom d'utilisateur SSH
username = "votre_utilisateur"
# Chemin vers votre clé SSH privée (optionnel, commentez si non utilisé)
key_path = "~/.ssh/id_rsa"
# Chemin de destination sur le serveur distant
remote_path = "/home/user/sauvegarde"
# Options rsync
rsync_options = ["-avz", "--progress", "--partial"]
# Synchronisation automatique toutes les X minutes (optionnel)
auto_sync_interval_minutes = 30
```

> **Note Linux :** si vous compilez depuis les sources, pensez à activer les fonctions Linux dans `src/config/settings.rs` avant de compiler (voir section [Installation](#-installation)).

## Workflows

### Workflow 0 : Démarrage avec le dashboard (recommandé)

```bash
# 1. Initialiser la configuration
filesentinel init

# 2. Ouvrir le dashboard
filesentinel dashboard

# Dans le dashboard :
# → Appuyer sur [2] pour aller dans l'onglet Surveillance
# → Appuyer sur [A] pour ajouter un dossier à surveiller
# → Appuyer sur [W] pour démarrer la surveillance
# → Appuyer sur [3] pour aller dans l'onglet Sync
# → Appuyer sur [E] pour définir la destination
# → Appuyer sur [S] pour lancer la première synchronisation
```

### Workflow 1 : Premier démarrage

```bash
# 1. Initialiser
filesentinel init

# 2. Éditer la configuration
vim config.toml  # ou notepad config.toml sur Windows

# 3. Créer un dossier source
mkdir mon_projet
echo "Hello World" > mon_projet/readme.txt

# 4. Première synchronisation
filesentinel sync -s ./mon_projet -d ./backup

# 5. Démarrer la surveillance
filesentinel watch -d ./mon_projet
```

### Workflow 2 : Développement avec surveillance

```bash
# Terminal 1 : Démarrer la surveillance
filesentinel -v watch -d ./src

# Terminal 2 : Travailler normalement
echo "fn main() {}" > src/main.rs
mkdir src/lib
echo "pub fn add(a: i32, b: i32) -> i32 { a + b }" > src/lib/math.rs

# Les changements sont automatiquement détectés et synchronisés
```

### Workflow 3 : Gestion de versions

```bash
# Activer le versionnement dans config.toml d'abord

# Créer plusieurs versions
echo "Version 1.0" > projet/CHANGELOG.md
filesentinel sync -s ./projet -d ./backup

echo "Version 2.0" > projet/CHANGELOG.md
filesentinel sync -s ./projet -d ./backup

echo "Version 3.0" > projet/CHANGELOG.md
filesentinel sync -s ./projet -d ./backup

# Voir l'historique
filesentinel version-history projet/CHANGELOG.md

# Restaurer la version 2
filesentinel restore projet/CHANGELOG.md -v 2
```

### Workflow 4 : Backup distant

```bash
# 1. Configurer l'accès SSH dans config.toml
vim config.toml

# 2. Tester la connexion (via network-sync qui teste automatiquement)
filesentinel network-sync to-remote

# 3. Synchronisation régulière
filesentinel network-sync to-remote
```

## Architecture

```
filesentinel/
├── src/
│   ├── main.rs              # Point d'entrée
│   ├── errors.rs            # Gestion d'erreurs
│   ├── watcher/
│   │   ├── mod.rs           # Trait Watcher
│   │   ├── types.rs         # Types communs
│   │   └── polling.rs       # Watcher par polling (MD5)
│   ├── synchro/
│   │   ├── mod.rs
│   │   └── engine.rs        # Moteur de synchronisation
│   ├── filters/
│   │   ├── mod.rs
│   │   └── rules.rs         # Règles de filtrage (glob, taille, ext)
│   ├── versioning/
│   │   └── mod.rs           # Gestion des versions
│   ├── compression/
│   │   └── mod.rs           # Compression de fichiers
│   ├── network/
│   │   └── mod.rs           # Synchronisation réseau SSH
│   ├── notifications/
│   │   └── mod.rs           # Notifications desktop
│   ├── config/
│   │   ├── mod.rs
│   │   └── settings.rs      # Configuration TOML
│   ├── cli/
│   │   ├── mod.rs
│   │   └── commands.rs      # Interface CLI (clap)
│   └── tui/
│       ├── mod.rs           # Point d'entrée du dashboard (boucle principale)
│       ├── app.rs           # État de l'application, threads background
│       ├── events.rs        # Gestion des touches clavier
│       └── ui.rs            # Rendu ratatui (6 onglets, overlay saisie)
├── Cargo.toml
├── config.toml
└── README.md
```

### Modules principaux

| Module | Description | Fonctionnalités clés |
|--------|-------------|---------------------|
| `watcher` | Détection de changements | Polling, hash MD5, comparaison d'états |
| `synchro` | Moteur de synchronisation | Copie différentielle, gestion conflits |
| `filters` | Système de filtrage | Patterns glob, taille, extensions |
| `versioning` | Gestion de versions | Sauvegarde, restauration, nettoyage |
| `compression` | Compression GZIP | Compression/décompression à la volée |
| `network` | Synchronisation SSH | Rsync over SSH, test connexion |
| `notifications` | Notifications desktop | Lots, priorités, fichiers critiques |
| `config` | Configuration | TOML, sérialisation, valeurs par défaut |
| `cli` | Interface en ligne de commande | Sous-commandes, arguments, aide |
| `tui` | Dashboard interactif | 6 onglets, threads background, overlay saisie |

## Développement

### Compilation

```bash
# Mode debug
cargo build

# Mode release (optimisé)
cargo build --release

# Avec des features spécifiques
cargo build --features "network"
```

### Tests

```bash
# Lancer tous les tests
cargo test

# Tests avec logs
cargo test -- --nocapture

# Tests spécifiques
cargo test test_sync_engine
```

### Qualité de code

```bash
# Vérification rapide
cargo check

# Linting
cargo clippy

# Formatage
cargo fmt

# Audit de sécurité
cargo audit
```

### Débogage

```bash
# Mode verbeux
RUST_LOG=debug filesentinel watch

# Avec fichier de logs
RUST_LOG=info filesentinel watch 2> filesentinel.log

# Logs encore plus détaillés
RUST_LOG=trace filesentinel -v watch
```

## Dépannage & FAQ

**Q : Les notifications ne fonctionnent pas sur Linux**
```bash
# Installer le serveur de notifications
sudo apt install libnotify-bin  # Ubuntu/Debian
sudo dnf install libnotify      # Fedora
```

**Q : Erreur de permission sur les fichiers**
```bash
# Vérifier les droits
ls -la

# Lancer avec les droits appropriés
sudo filesentinel watch -d /var/log
```

**Q : Comment configurer la synchronisation réseau ?**

Après `filesentinel init`, la section `[network]` est déjà présente dans `config.toml` avec des valeurs placeholder. Modifiez-les :
```toml
[network]
host = "mon-serveur.com"      # Adresse de votre serveur
port = 22                      # Port SSH
username = "alice"             # Votre identifiant
key_path = "~/.ssh/id_rsa"    # Votre clé privée SSH
remote_path = "/home/alice/backup"
rsync_options = ["-avz", "--progress", "--partial"]
auto_sync_interval_minutes = 30
```
Puis testez avec `filesentinel network-sync to-remote`.

**Q : La synchronisation réseau échoue**
```bash
# Vérifier rsync
rsync --version

# Vérifier SSH (remplacer user@host par votre vrai host)
ssh user@host "echo test"

# Tester avec filesentinel
filesentinel network-sync to-remote
```

### Configuration

**Q : Comment exclure les fichiers cachés ?**
```toml
[filters]
exclude_patterns = ["**/.*", "**/.*/**"]
```

**Q : Comment surveiller uniquement les fichiers Rust ?**
```toml
[filters]
include_extensions = ["rs", "toml", "lock"]
```

**Q : Comment augmenter le nombre de versions ?**
```toml
[versioning]
max_versions = 20
```

## Cas d'utilisation

### Développeurs
- Sauvegarde automatique du code source
- Synchronisation entre machines de développement
- Versionnement des fichiers de configuration

### Designers
- Backup automatique des assets
- Historique des versions de créations
- Synchronisation avec un NAS

### Administrateurs système
- Surveillance des logs
- Backup de configuration serveur
- Réplication de données

### Étudiants & Enseignants
- Sauvegarde de travaux académiques
- Synchronisation avec le cloud
- Protection contre la perte de données



## Roadmap

### Version 0.2.0 ✅ (actuelle)
- [x] Dashboard TUI interactif avec ratatui (6 onglets)
- [x] Gestion des versions depuis l'interface (restauration, suppression, nettoyage)
- [x] Ajout/suppression de dossiers surveillés à chaud
- [x] Annulation de synchronisation
- [x] Flux d'événements en temps réel

### Version 0.3.0
- [ ] Support WebSocket pour interface web
- [ ] Compression différentielle

### Version 0.4.0
- [ ] Chiffrement des sauvegardes
- [ ] Support S3/Cloud storage
- [ ] Planification de synchronisation

### Version 1.0.0
- [ ] GUI avec egui/tauri
- [ ] Tests de performance
- [ ] Documentation complète

## Licence

MIT License - voir le fichier [LICENSE](LICENSE) pour plus de détails.

## Remerciements

Merci aux créateurs de ces excellentes crates Rust :

- [notify](https://github.com/notify-rs/notify) - Surveillance de fichiers cross-platform
- [clap](https://github.com/clap-rs/clap) - Interface en ligne de commande robuste
- [ratatui](https://github.com/ratatui/ratatui) - Framework TUI pour le dashboard interactif
- [crossterm](https://github.com/crossterm-rs/crossterm) - Backend terminal cross-platform
- [serde](https://github.com/serde-rs/serde) - Sérialisation/désérialisation
- [chrono](https://github.com/chronotope/chrono) - Gestion du temps et dates
- [flate2](https://github.com/rust-lang/flate2-rs) - Compression GZIP
- [toml](https://github.com/toml-rs/toml) - Parsing TOML
- [anyhow](https://github.com/dtolnay/anyhow) - Gestion d'erreurs ergonomique

## Contribution

Les contributions sont les bienvenues ! Merci de :

1. Fork le projet
2. Créer une branche pour votre fonctionnalité (`git checkout -b feature/AmazingFeature`)
3. Commiter vos changements (`git commit -m 'Add AmazingFeature'`)
4. Pusher la branche (`git push origin feature/AmazingFeature`)
5. Ouvrir une Pull Request

### Guidelines de contribution

- Respecter le style de code existant
- Ajouter des tests pour les nouvelles fonctionnalités
- Mettre à jour la documentation
- Utiliser `cargo fmt` et `cargo clippy` avant de soumettre

## Support et contact

- **Issues GitHub** : Signaler des bugs ou demander des fonctionnalités
- **Discussions** : Poser des questions sur l'utilisation
- **GitHub Wiki** : Consulter la documentation étendue

## Version actuelle

**FileSentinel v0.2.0** - Mai 2026 — Dashboard TUI interactif
