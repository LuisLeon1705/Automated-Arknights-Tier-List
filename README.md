# Arknights Analytical Tier List Engine

An empirical, mathematical combat simulator and analytical tier list engine for the mobile game *Arknights*. The project replaces subjective community rankings with deterministic, objective combat simulations over 300-second (5-minute) standard battle encounters. (Up to date to September 11 2026 Global)

---

## Acknowledgements & Credits

- **Database & Reference Models**: This project builds upon the open database structures, formula blueprints, and mechanics compiled by **[myrtle.moe](https://myrtle.moe)**. We express our deepest gratitude to the creators and maintainers of `myrtle.moe` for their invaluable contribution to the Arknights analytical community.
- **Arknights**: All game assets, character illustrations, lore, and core game design belong to **Hypergryph / Studio Montagne / Yostar**.

---

## Core Features & Simulation Logic

### 1. Deterministic Combat Engine
- **Granular 5-Minute (300s) Loop**: Simulates realistic sustained vs. burst damage windows rather than theoretical infinite-time averages.
- **Discrete Configuration State-Space**: Evaluates each operator across every independent skill (`RAW`, `S1`, `S2`, `S3`) and module (`MOD-X`, `MOD-Y`, `MOD-D`, `MOD-RA`, `MOD-IS`, `NO MOD`), simulating over 3,000 unique configurations.
- **Multi-Type Damage Floor**: Respects the Arknights hardcoded 5% damage floor for Physical and Arts attacks after defense and resistance subtractions.

### 2. Specialized Target Profiles
The engine benchmarks configurations against distinct combat scenarios:
- **General**: Composite statistical enemy dataset.
- **Normal Enemies**: Swarm encounters with low defense and high mob count.
- **Elite Enemies**: Mid-to-high DEF / RES targets requiring sustained shredding.
- **Bosses**: High-threat targets (1,000 DEF, 50 RES, 80k HP, high ATK).
- **Reclamation Algorithm (RA)**: Horde control, wide AoE coverage, and frontline durability.
- **Integrated Strategies (IS)**: Roguelike burst damage scaling and crowd control (CC).
- **Contingency Contract (CC)**: The single most demanding permanent mode — a hazard-buffed boss-plus-elite-wave field, modeled tougher than the plain Boss profile across DEF/RES/HP/ATK.
- **DP Generators**: Specialized Vanguard benchmark tracking DP production reliability.

### 3. Comprehensive Metric Modeling
- **Damage Profiles**: Physical DPS, Arts DPS, Elemental DPS, True Damage, and Peak Burst Damage.
- **Survivability & Bulk**: Physical/Arts Effective HP (EHP), Hits-to-Kill (HTK) against category attack values, team Sanctuary mitigations, and block-count enforcement.
- **Exact DP Generation**: Accurate canonical modeling of Flagbearers (Myrtle, Elysium, Saileach), Pioneers (Texas, Siege, Courier), and Agents (Cantabile ammo skills, Ines timed stealing).
- **Team Support Utility**: Defense/Resist shreds, Fragile multipliers, Stun/Silence/Slow durations, and Inspiring ATK/DEF buffs.

### 4. Team Analysis (`/teams`)
Aggregates the per-operator simulation above into a 12-operator TEAM score — no new combat math, just a scoring layer on top of it.
- **Team Builder**: assemble any 12-operator squad (rarity/class/archetype filters, randomizer) and get an instant 6-axis radar — Boss Killing, Lane Holding, Resistance, Utility, Consistency, Reliability — each graded A-E, with rule-based recommendations for closing gaps. Click any team member to lock in a specific skill/module loadout instead of the auto-picked best config, via an inline picker with live skill/module icons fetched from myrtle.moe.
- **Team Tier List**: a genetic search evolves team compositions (drawn from each class's top 5 individually-ranked operators) toward high overall score, surfacing hundreds of viable teams ranked OP-F, cached and auto-regenerated whenever the roster data changes.

---

## Architecture & Tech Stack

```
TierList/
├── CHANGELOG.md                # Version history & patch notes (v1.1)
├── DOCUMENTATION.md            # Comprehensive technical documentation & API reference
├── data/                       # Processed JSON databases
│   ├── Automated_Operators.json
│   ├── Automated_Enemies.json
│   └── classes.json
├── rust_engine/                # High-performance simulation server (Rust)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs             # Axum HTTP routes and tier list ranking endpoints
│       └── core/
│           ├── models.rs       # Operator, Skill, Talent, Buff data structures
│           ├── simulation.rs   # 5-minute combat simulation loop & formulas
│           ├── data_loader.rs  # Blackboard parser & stat normalizer
│           ├── enemy.rs        # Enemy tier calculator & statistical averages
│           └── team.rs         # Team-level scoring (6-axis) & genetic Team Tier List search
├── Scripts/                    # Data extraction scripts (Python), plus a standalone PDF generator
│   ├── extract_operators.py
│   ├── extract_enemies.py
│   ├── generate_tierlist_pdfs.py  # Offline-only; not used by the web app (see Data Export below)
│   └── ...
├── templates/                  # Jinja2 / HTML5 frontend dashboards
│   ├── home.html                # Homepage / roster overview
│   ├── tierlist.html           # Operator Tier List with CDF charts & filters
│   ├── teams.html              # Team Builder + auto-generated Team Tier List
│   ├── enemy_tierlist.html     # Enemy Threat Tier List
│   ├── comparisons.html        # Direct Comparisons (operator-vs-operator)
│   └── editor.html             # Interactive Operator / Buff editor
├── static/                     # Stylesheets, scripts, and portrait assets
│   ├── css/
│   ├── js/                     # operator-icons.js, team-radar.js, main.js
│   └── images/                 # Operator portraits (490+ portraits)
└── ejecutar_programa.bat       # One-click launch script
```

---

## Quick Start

### Prerequisites
- **Rust** (1.75+ recommended): [https://rustup.rs](https://rustup.rs)
- **Python** (3.10+): only needed for the data-extraction pipeline in `Scripts/` (and the optional standalone PDF generator, which needs `pip install reportlab`) — not required to run the web app itself.

### Running the Application

1. **Automatic (Windows)**:
   Double-click `ejecutar_programa.bat` in the project root.

2. **Manual (Console)**:
   ```bash
   cd rust_engine
   cargo run --release
   ```

3. Open your browser and navigate to:
   ```
   http://127.0.0.1:8000/tierlist
   ```

---

## Data Export
Click **"EXPORT CSV"** on the Tier List page to download the current category's ranking as a lightweight CSV. An earlier PDF/ZIP export (116 pre-compiled PDF reports, ~22 MB) was retired in favor of this — CSV covers the same data at a fraction of the size and loads instantly. The standalone `Scripts/generate_tierlist_pdfs.py` generator still exists if you want PDF reports for offline use, but it's no longer wired into the web app.

---

## Versioning & Updates
Current Version: **v1.1 (2026-09-12)**  
See [`CHANGELOG.md`](file:///C:/Users/leonp/OneDrive/Escritorio/TierList/CHANGELOG.md) for the complete version history, release notes, and known considerations for this initial release.

"# Automated-Arknights-Tier-List" 
