# Arknights Analytical Tier List Engine

An empirical, mathematical combat simulator and analytical tier list engine for the mobile game *Arknights*. The project replaces subjective community rankings with deterministic, objective combat simulations over 300-second (5-minute) standard battle encounters.

---

## Directory Overview

The project is built on a high-performance **Rust** simulation core serving an interactive web interface with HTML5, CSS3, and Minijinja templates, backed by automated Python data pipelines.

```
TierList/
├── .gitignore                  # Git exclusions (Rust build artifacts, myrtle-main, logs, zips)
├── CHANGELOG.md                # Version history & patch notes (v1.1)
├── GEMINI.md                   # System rules and architectural overview
├── README.md                   # Project presentation and myrtle.moe attribution
├── DOCUMENTATION.md            # Comprehensive technical documentation & API reference
├── ejecutar_programa.bat       # Windows launch script for release engine
├── data/                       # Normalized JSON databases
│   ├── Automated_Operators.json # 426+ operators with skills, modules, and talents
│   ├── Automated_Enemies.json   # 1,698+ enemies with stats, skills, and mechanics
│   └── classes.json             # Archetype definitions and branch traits
├── rust_engine/                # High-performance simulation server in Rust
│   ├── Cargo.toml              # Rust crate manifest (Axum, Tokio, Rayon, Minijinja)
│   ├── Cargo.lock              # Pinned dependencies
│   └── src/
│       ├── main.rs             # Axum HTTP routes, REST endpoints, and ranking algorithms
│       └── core/
│           ├── mod.rs          # Module declarations
│           ├── models.rs       # Operator, Skill, Module, Talent, and Buff data structures
│           ├── simulation.rs   # 300s discrete simulation loop and damage calculations
│           ├── data_loader.rs  # Blackboard parser and JSON normalizer
│           ├── enemy.rs        # Enemy threat model and category baseline averages
│           └── team.rs         # Team-level 6-axis scoring & genetic Team Tier List search
├── Scripts/                    # Data extraction & normalization (Python), plus a standalone PDF generator
│   ├── add_ids.py              # Operator and skill ID assigner
│   ├── add_rarity.py           # Rarity injector
│   ├── extract_enemies.py      # Enemy gamedata extractor
│   ├── extract_operators.py    # Operator gamedata extractor
│   ├── fixup_cn_operators.py   # CN server operator normalizer
│   ├── generate_tierlist_pdfs.py # Offline-only 116-PDF report generator; not used by the web app (see CSV export)
│   ├── recover_images.py       # Portrait recovery and asset linker
│   └── update_skills_talents.py# Blackboard buff parser and updater
├── templates/                  # Minijinja HTML templates
│   ├── base.html               # Shared layout and navigation shell
│   ├── home.html                # Main landing homepage (roster overview)
│   ├── tierlist.html           # Operator Tier List with live CDF charts and metric filters
│   ├── teams.html              # Team Builder + auto-generated Team Tier List
│   ├── comparisons.html        # Direct Comparisons (operator-vs-operator)
│   ├── enemy_tierlist.html     # Enemy Threat Tier List with radar metrics
│   ├── editor.html             # Interactive Operator and Buff simulator editor
│   └── enemy_editor.html       # Enemy stat and threat level editor
└── static/                     # Static assets
    ├── css/style.css           # Glassmorphism dark-theme styling
    ├── js/main.js              # Client-side utility functions
    ├── js/operator-icons.js    # Shared class/archetype icon + name helpers
    ├── js/team-radar.js        # 6-axis hexagonal radar chart renderer (Team pages)
    └── images/                 # Operator portraits, class icons, archetype icons
```

---

## Architectural Highlights

### 1. Deterministic Combat Engine (`rust_engine`)
- **Granular 300-Second Simulation**: Simulates exact rotation timings, attack intervals, SP charge curves, burst windows, and downtime instead of infinite-time approximations.
- **State-Space Evaluation**: Evaluates each operator across every skill (`RAW`, `S1`, `S2`, `S3`) and module (`MOD-X`, `MOD-Y`, `MOD-D`, `MOD-RA`, `MOD-IS`, `NO MOD`), generating over 3,000 unique configurations.
- **Rayon Parallelism**: Benchmarks all 426+ operators across all configurations in under 150 milliseconds using multi-threaded work stealing.
- **Damage Floor Rule**: Enforces the canonical Arknights 5% hardcoded damage floor for Physical and Arts damage after DEF/RES calculations.
- **Fast-Redeploy & Defeat Mechanics**: Models retreat and redeployment cycles for Executors (Texas Alter, Yato Alter, etc.) and penalizes fragile operators that suffer defeats against high enemy DPS.

### 2. Specialized Target Profiles
Evaluates combat efficacy across 7 distinct encounter profiles:
- **General**: Composite average enemy dataset.
- **Normal**: High-density swarm targets with low defense.
- **Elite**: Mid-to-high DEF / RES targets requiring sustained shredding.
- **Boss**: High-threat targets (1,000 DEF, 50 RES, 80,000 HP, high ATK).
- **Reclamation Algorithm (RA)**: Large horde suppression and defensive chokeholds.
- **Integrated Strategies (IS)**: Roguelike burst execution and crowd control.
- **DP Generators**: Specialized Vanguard reliability and generation rate ranking.

### 3. Team Analysis (`core/team.rs`, `/teams`)
Aggregates the per-operator simulation into a 12-operator TEAM score — a scoring layer on top of the existing engine, not new combat math.
- **6 axes**: Boss Killing, Lane Holding, Resistance, Utility (deploy speed + buffs/debuffs/CC), Consistency, Reliability (penalizes over-specialization), each graded A-E off a rendered hexagonal radar (`static/js/team-radar.js`).
- **Team Builder**: manual 12-operator assembly (`POST /api/team_score`, computed live) with rule-based improvement recommendations.
- **Team Tier List**: a genetic search (population/generations/elitism/crossover/mutation + random immigrants to preserve diversity) over each class's top 5 individually-ranked operators, cached (`GET /api/team_tierlist`) and auto-invalidated when the roster data changes.

### 4. Technology Stack
- **Server Framework**: Axum 0.8 with Tokio async runtime
- **Template Engine**: Minijinja 2.24 (Jinja2 compatible)
- **Parallel Computing**: Rayon 1.12
- **Data Serialization**: Serde / Serde JSON
- **Randomness**: Rand 0.8 (Team Tier List's genetic search)
- **Asset Serving**: Tower-HTTP
- **Skill/Module Icons**: fetched live client-side from myrtle.moe's public API (`/api/operators/{char_id}`, `/api/skill-icon/{id}`, `/api/module-icon/{id}`) for the Team Builder's loadout picker
- **Database & Reference**: [myrtle.moe](https://myrtle.moe) open database
