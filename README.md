# Arknights Analytical Tier List Engine

An empirical, mathematical combat simulator and analytical tier list engine for the mobile game *Arknights*. The project replaces subjective community rankings with deterministic, objective combat simulations over 300-second (5-minute) standard battle encounters. (Up to date to November 11 2026 Global)

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
- **DP Generators**: Specialized Vanguard benchmark tracking DP production reliability.

### 3. Comprehensive Metric Modeling
- **Damage Profiles**: Physical DPS, Arts DPS, Elemental DPS, True Damage, and Peak Burst Damage.
- **Survivability & Bulk**: Physical/Arts Effective HP (EHP), Hits-to-Kill (HTK) against category attack values, team Sanctuary mitigations, and block-count enforcement.
- **Exact DP Generation**: Accurate canonical modeling of Flagbearers (Myrtle, Elysium, Saileach), Pioneers (Texas, Siege, Courier), and Agents (Cantabile ammo skills, Ines timed stealing).
- **Team Support Utility**: Defense/Resist shreds, Fragile multipliers, Stun/Silence/Slow durations, and Inspiring ATK/DEF buffs.

---

## Architecture & Tech Stack

```
TierList/
├── CHANGELOG.md                # Version history & patch notes (v1.0.0)
├── DOCUMENTATION.md            # Comprehensive technical documentation & API reference
├── data/                       # Processed JSON databases and PDF archives
│   ├── Automated_Operators.json
│   ├── Automated_Enemies.json
│   ├── classes.json
│   └── Arknights_Tier_Lists_PDF.zip
├── rust_engine/                # High-performance simulation server (Rust)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs             # Axum HTTP routes and tier list ranking endpoints
│       └── core/
│           ├── models.rs       # Operator, Skill, Talent, Buff data structures
│           ├── simulation.rs   # 5-minute combat simulation loop & formulas
│           ├── data_loader.rs  # Blackboard parser & stat normalizer
│           └── enemy.rs        # Enemy tier calculator & statistical averages
├── Scripts/                    # Data extraction and PDF generation scripts (Python)
│   ├── extract_operators.py
│   ├── extract_enemies.py
│   ├── generate_tierlist_pdfs.py
│   └── ...
├── templates/                  # Jinja2 / HTML5 frontend dashboards
│   ├── tierlist.html           # Operator Tier List with CDF charts & filters
│   ├── enemy_tierlist.html     # Enemy Threat Tier List
│   └── editor.html             # Interactive Operator / Buff editor
├── static/                     # Stylesheets and portrait assets
│   ├── css/
│   └── images/                 # Operator portraits (490+ portraits)
└── ejecutar_programa.bat       # One-click launch script
```

---

## Quick Start

### Prerequisites
- **Rust** (1.75+ recommended): [https://rustup.rs](https://rustup.rs)
- **Python** (3.10+): with `reportlab` installed for PDF generation:
  ```bash
  pip install reportlab
  ```

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

## PDF Reports Export
Click the **"DESCARGAR PDFS (.ZIP)"** button in the dashboard or run:
```bash
python Scripts/generate_tierlist_pdfs.py
```
This automatically produces detailed PDF ranking documents for every category and metric, including all 3,000+ skill and module variations.

---

## Versioning & Updates
Current Version: **v1.0.2 (Stable Core Release)**  
See [`CHANGELOG.md`](file:///C:/Users/leonp/OneDrive/Escritorio/TierList/CHANGELOG.md) for the complete version history, release notes, and known considerations for this initial release.

"# Automated-Arknights-Tier-List" 
