# Arknights Analytical Tier List Engine

An empirical, mathematical combat simulator and analytical tier list engine for the mobile game *Arknights*. The project replaces subjective community rankings with deterministic, objective combat simulations over 300-second (5-minute) standard battle encounters.

---

## Directory Overview

The project is built on a high-performance **Rust** simulation core serving an interactive web interface with HTML5, CSS3, and Minijinja templates, backed by automated Python data pipelines.

```
TierList/
├── .gitignore                  # Git exclusions (Rust build artifacts, myrtle-main, logs, zips)
├── CHANGELOG.md                # Version history & patch notes (v1.0.0)
├── GEMINI.md                   # System rules and architectural overview
├── README.md                   # Project presentation and myrtle.moe attribution
├── DOCUMENTATION.md            # Comprehensive technical documentation & API reference
├── ejecutar_programa.bat       # Windows launch script for release engine
├── data/                       # Normalized JSON databases
│   ├── Automated_Operators.json # 426+ operators with skills, modules, and talents
│   ├── Automated_Enemies.json   # 1,698+ enemies with stats, skills, and mechanics
│   ├── classes.json             # Archetype definitions and branch traits
│   └── Arknights_Tier_Lists_PDF.zip # Compiled 116-PDF tier list bundle
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
│           └── enemy.rs        # Enemy threat model and category baseline averages
├── Scripts/                    # Data extraction, normalization, and PDF generation (Python)
│   ├── add_ids.py              # Operator and skill ID assigner
│   ├── add_rarity.py           # Rarity injector
│   ├── extract_enemies.py      # Enemy gamedata extractor
│   ├── extract_operators.py    # Operator gamedata extractor
│   ├── fixup_cn_operators.py   # CN server operator normalizer
│   ├── generate_tierlist_pdfs.py # 116 analytical PDF report generator (ReportLab)
│   ├── recover_images.py       # Portrait recovery and asset linker
│   └── update_skills_talents.py# Blackboard buff parser and updater
├── templates/                  # Minijinja HTML templates
│   ├── base.html               # Shared layout and navigation shell
│   ├── dashboard.html          # Main landing dashboard
│   ├── tierlist.html           # Operator Tier List with live CDF charts and metric filters
│   ├── enemy_tierlist.html     # Enemy Threat Tier List with radar metrics
│   ├── editor.html             # Interactive Operator and Buff simulator editor
│   └── enemy_editor.html       # Enemy stat and threat level editor
└── static/                     # Static assets
    ├── css/style.css           # Glassmorphism dark-theme styling
    ├── js/main.js              # Client-side utility functions
    ├── avatars/                # Operator portrait images
    └── enemy_avatars/          # Enemy portrait images
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

### 3. Technology Stack
- **Server Framework**: Axum 0.8 with Tokio async runtime
- **Template Engine**: Minijinja 2.24 (Jinja2 compatible)
- **Parallel Computing**: Rayon 1.12
- **Data Serialization**: Serde / Serde JSON
- **Asset Serving**: Tower-HTTP
- **PDF Generation**: Python ReportLab
- **Database & Reference**: [myrtle.moe](https://myrtle.moe) open database
