# Arknights Analytical Tier List Engine - Technical Documentation & API Reference

An empirical, mathematical combat simulator and analytical tier list engine for *Arknights*. This document details the system architecture, mathematical formulations, REST API specifications, file catalogs, and simulation logic.

---

## 1. System Architecture Overview

The system is structured as a high-performance, deterministic combat simulator running on a **Rust** backend with an asynchronous **Axum** web framework, accompanied by a reactive frontend rendered using **Minijinja** (Jinja2-compatible) templates and vanilla JavaScript. Data extraction, enrichment, and batch PDF generation are handled by a modular **Python** pipeline.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                             Web Client (Browser)                            │
│  - Operator Tier List (CDF Chart, Metric Filters, Search, Badges)           │
│  - Enemy Threat Tier List (Threat Scores, Radar Metrics, Filters)           │
│  - Operator & Buff Sandbox Editor (Live Rotation & Burst Testing)           │
│  - Enemy Sandbox Editor (Stat & Mechanics Customizer)                       │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │ HTTP / REST / Server-Rendered HTML
┌──────────────────────────────────────▼──────────────────────────────────────┐
│                    Rust Engine (Axum + Tokio Runtime)                       │
│  - Multi-threaded Simulation (Rayon work-stealing parallelism)              │
│  - 300s Discrete Rotation Simulator (core::simulation::SimulationEnvironment)│
│  - Stat Calculator & Stacking Logic (core::models::Operator)                │
│  - Gamedata & Blackboard Parser (core::data_loader::DataLoader)             │
│  - Enemy Threat & Category Profiler (core::enemy)                           │
│  - Template Renderer (Minijinja) & Static Asset Handler (Tower-HTTP)        │
└──────────────────┬───────────────────────────────────────┬──────────────────┘
                   │ Reads / Writes                        │ Invokes on-demand
┌──────────────────▼──────────────────┐ ┌──────────────────▼──────────────────┐
│          JSON Databases             │ │       Python Script Pipeline        │
│  - data/Automated_Operators.json    │ │  - Scripts/generate_tierlist_pdfs.py │
│  - data/Automated_Enemies.json      │ │  - Scripts/extract_operators.py     │
│  - data/classes.json                │ │  - Scripts/extract_enemies.py       │
│  - data/Arknights_Tier_Lists_PDF.zip│ │  - Scripts/update_skills_talents.py │
└─────────────────────────────────────┘ └─────────────────────────────────────┘
```

---

## 2. Comprehensive REST API Reference

All endpoints are hosted by default on `http://127.0.0.1:8000` (configurable via the `PORT` environment variable).

---

### 2.1 View Routes (HTML Delivery)

#### `GET /`
- **Handler**: `read_root` (`rust_engine/src/main.rs`)
- **Description**: Renders the main landing dashboard displaying project statistics, operator counts, and navigation shortcuts.
- **Template**: `templates/dashboard.html`
- **Response**: `200 OK` (Content-Type: `text/html; charset=utf-8`)

#### `GET /tierlist`
- **Handler**: `tierlist_view` (`rust_engine/src/main.rs`)
- **Description**: Renders the primary Operator Analytical Tier List dashboard featuring cumulative distribution function (CDF) curve graphs, category selectors, ranking metric toggles, search filters, and PDF/CSV export buttons.
- **Template**: `templates/tierlist.html`
- **Response**: `200 OK` (Content-Type: `text/html; charset=utf-8`)

#### `GET /enemy_tierlist`
- **Handler**: `enemy_tierlist_view` (`rust_engine/src/main.rs`)
- **Description**: Renders the Enemy Threat Tier List dashboard displaying threat radar metrics, survivability parameters, DPS outputs, special mechanic flags, and category filters.
- **Template**: `templates/enemy_tierlist.html`
- **Response**: `200 OK` (Content-Type: `text/html; charset=utf-8`)

#### `GET /editor`
- **Handler**: `operator_editor` (`rust_engine/src/main.rs`)
- **Description**: Renders the Operator and Buff Sandbox Editor. Allows interactive parameter editing, buff injection, skill adjustments, and immediate 300-second rotation simulation with interactive charts.
- **Query Parameters**:
  - `op_id` *(optional, integer)*: Operator ID to pre-load into the editor.
- **Template**: `templates/editor.html`
- **Response**: `200 OK` (Content-Type: `text/html; charset=utf-8`)

#### `GET /enemy_editor`
- **Handler**: `enemy_editor_view` (`rust_engine/src/main.rs`)
- **Description**: Renders the Enemy Sandbox Editor, allowing users to modify HP, DEF, RES, ATK, attack intervals, revive phases, dodge chances, and threat tiers.
- **Template**: `templates/enemy_editor.html`
- **Response**: `200 OK` (Content-Type: `text/html; charset=utf-8`)

---

### 2.2 Analytical & Simulation APIs

#### `GET /api/tierlist_data`
- **Handler**: `get_tierlist_data` (`rust_engine/src/main.rs`)
- **Description**: Computes the complete deterministic analytical tier list ranking for all 426+ operators across all valid skill and module configurations against the selected category's enemy profile. Parallelized via Rayon.
- **Query Parameters**:
  - `category` *(optional, string)*: Target benchmark scenario. Options:
    - `"general"` (default): Statistical composite enemy dataset.
    - `"normal"`: Swarm encounter (low DEF, high target count).
    - `"elite"`: Armored targets requiring sustained shredding.
    - `"boss"`: High-threat boss encounter (1,000 DEF, 50 RES, 80,000 HP, high ATK).
    - `"ra"`: Reclamation Algorithm profile (wide AoE and endurance).
    - `"is"`: Integrated Strategies profile (burst damage and CC scaling).
    - `"dp"`: DP Generator ranking (filtered to Vanguards and DP producers).
  - `apply_decay` *(optional, boolean)*: Whether to apply the combat defeat uptime penalty. Defaults to `true`.
- **Response Payload**: `200 OK` (Content-Type: `application/json`)
  ```json
  {
    "category": "boss",
    "enemy_stats": {
      "hp": 80000.0,
      "atk": 1500.0,
      "def": 1000.0,
      "res": 50.0,
      "attack_interval": 3.0,
      "weight": 5.0,
      "dodge_phys": 0.0,
      "dodge_arts": 0.0
    },
    "general": [
      {
        "operator_name": "Wis'adel",
        "class_name": "SNIPER",
        "subclass_name": "Flinger",
        "rarity": 6,
        "skill_name": "S3: 'Whispers of the Revenant'",
        "module_name": "MOD-X (Level 3)",
        "score": 3840.25,
        "tier": "OP",
        "phys_dmg": 482000.0,
        "arts_dmg": 0.0,
        "true_dmg": 0.0,
        "elemental_dmg": 0.0,
        "total_dmg": 482000.0,
        "dps": 1606.67,
        "max_burst": 220000.0,
        "wave_ttc": 6.8,
        "boss_ttc": 14.2,
        "surv": 5820.0,
        "phys_surv": 6200.0,
        "arts_surv": 5400.0,
        "hits_to_kill": 12.4,
        "dp": 0.0,
        "heal": 0.0,
        "support": 145.0,
        "niche": 28.0,
        "photo_path": "/static/avatars/char_1033_wisdel.png"
      }
    ],
    "detailed": [ /* Contains all 3,000+ individual skill/module variations */ ]
  }
  ```

#### `GET /api/enemy_tierlist_data`
- **Handler**: `get_enemy_tierlist_data` (`rust_engine/src/main.rs`)
- **Description**: Retrieves and enriches the complete enemy threat ranking dataset. Calculates physical/arts effective HP, raw and ability DPS, multi-target threat, and composite Threat Scores.
- **Query Parameters**:
  - `category` *(optional, string)*: Filter by enemy tier. Options: `"all"` (default), `"normal"`, `"elite"`, `"boss"`.
- **Response Payload**: `200 OK` (Content-Type: `application/json`)
  ```json
  [
    {
      "id": "enemy_1001_patrot",
      "name": "Patriot",
      "tier_type": "BOSS",
      "hp": 45000.0,
      "def": 2100.0,
      "res": 90.0,
      "atk": 2500.0,
      "attack_interval": 4.0,
      "weight": 7.0,
      "threat_score": 92.45,
      "tier": "S",
      "ehp_phys": 900000.0,
      "ehp_arts": 450000.0,
      "dps": 625.0,
      "special_mechanics": ["Invulnerable Phase Transition", "Taunt", "Global March"]
    }
  ]
  ```

#### `POST /api/simulate`
- **Handler**: `run_simulation` (`rust_engine/src/main.rs`)
- **Description**: Runs an isolated, high-precision 300-second combat simulation for a single operator configuration against custom target parameters.
- **Request Body**: `application/json`
  ```json
  {
    "operator_name": "SilverAsh",
    "skill_index": 2,
    "module_index": 0,
    "target_def": 800.0,
    "target_res": 30.0,
    "target_atk": 1200.0,
    "target_interval": 2.5,
    "target_weight": 3.0,
    "is_boss": false
  }
  ```
- **Response Payload**: `200 OK` (Content-Type: `application/json`)
  Returns sampled cumulative curves for damage, healing, and DP, discrete skill activation events, and final recalculated stats.

#### `POST /api/simulate_batch`
- **Handler**: `run_simulation_batch` (`rust_engine/src/main.rs`)
- **Description**: Executes parallel simulations for an arbitrary batch of operator configurations with globally overridden enemy armor, resistance, and attack parameters.
- **Request Body**: `application/json`
  ```json
  {
    "configs": [
      { "operator_name": "Młynar", "skill_index": 2, "module_index": 0 },
      { "operator_name": "Surtr", "skill_index": 2, "module_index": 0 }
    ],
    "global_target_def": 1200.0,
    "global_target_res": 50.0
  }
  ```
- **Response Payload**: `200 OK` (Content-Type: `application/json`)
  Returns an array of simulation summaries, final stats, and sampled yield curves for each requested configuration.

---

### 2.3 Operator Management APIs

#### `GET /api/operators`
- **Handler**: `get_operators` (`rust_engine/src/main.rs`)
- **Description**: Returns the raw array of all registered operators, including skills, modules, talents, base stats, and image paths.
- **Response**: `200 OK` (`application/json`)

#### `POST /api/operators/save`
- **Handler**: `save_operator` (`rust_engine/src/main.rs`)
- **Description**: Creates or updates an operator record in `data/Automated_Operators.json`. Supports uploading portrait image files.
- **Content-Type**: `multipart/form-data`
  - `operator_data` *(text/json)*: Complete serialized Operator JSON object.
  - `photo` *(optional, binary file)*: Avatar image file (`.png`, `.jpg`, `.webp`). Saved into `static/avatars/`.
- **Response**: `200 OK` with `{ "status": "success", "operator_id": 1050 }`

#### `DELETE /api/operators/{op_id}`
- **Handler**: `delete_operator` (`rust_engine/src/main.rs`)
- **Description**: Permanently deletes an operator by their numeric ID from `data/Automated_Operators.json`.
- **URL Parameter**: `op_id` *(integer)*
- **Response**: `200 OK` with `{ "status": "deleted", "operator_id": 1050 }`

---

### 2.4 Enemy Management APIs

#### `GET /api/enemies`
- **Handler**: `get_enemies` (`rust_engine/src/main.rs`)
- **Description**: Returns all enemy records stored in `data/Automated_Enemies.json`.
- **Response**: `200 OK` (`application/json`)

#### `POST /api/enemies/save`
- **Handler**: `save_enemy` (`rust_engine/src/main.rs`)
- **Description**: Creates or updates an enemy record in `data/Automated_Enemies.json`.
- **Request Body**: `application/json` (`EnemyData` schema)
- **Response**: `200 OK` with `{ "status": "success", "id": "enemy_1001_patrot" }`

#### `DELETE /api/enemies/{id}`
- **Handler**: `delete_enemy` (`rust_engine/src/main.rs`)
- **Description**: Permanently deletes an enemy record by string ID.
- **URL Parameter**: `id` *(string)*
- **Response**: `200 OK` with `{ "status": "deleted", "id": "enemy_1001_patrot" }`

---

### 2.5 Export & Download APIs

#### `GET /api/tierlist/export`
- **Handler**: `export_tierlist_csv` (`rust_engine/src/main.rs`)
- **Description**: Generates and streams a high-density CSV spreadsheet containing ranks, tiers, scores, damage types, survivability, and utility metrics for the requested category.
- **Query Parameters**: `category` (e.g. `general`, `boss`, `elite`).
- **Response**: `200 OK` (Content-Type: `text/csv`, Content-Disposition: `attachment; filename="arknights_tier_list.csv"`)

#### `GET /api/tierlist/export_pdfs_zip`
- **Handler**: `download_tierlists_pdf_zip` (`rust_engine/src/main.rs`)
- **Description**: Serves the pre-compiled ZIP archive containing all 116 analytical PDF reports. If the ZIP does not exist on disk, the endpoint automatically invokes `Scripts/generate_tierlist_pdfs.py` to compile it on-demand.
- **Response**: `200 OK` (Content-Type: `application/zip`, Content-Disposition: `attachment; filename="Arknights_Tier_Lists_PDF.zip"`)

---

## 3. Complete File-by-File Catalog

### 3.1 Project Root

| File Path | Description & Role | Key Dependencies / Tools |
|---|---|---|
| [`ejecutar_programa.bat`] | Windows batch script that builds the Rust engine in `--release` mode and launches the web server on `http://127.0.0.1:8000`. | Windows CMD, Cargo |
| [`README.md`] | Project presentation document providing feature overviews, architecture highlights, quick-start guides, and explicit attribution to **[myrtle.moe](https://myrtle.moe)**. | Markdown |
| [`CHANGELOG.md`]| Official version history, patch notes, and release considerations for v1.0.2. | Markdown |
| [`GEMINI.md`] | Master AI context guide and system specification detailing directories, rules, and core mathematical principles. | Markdown |
| [`DOCUMENTATION.md`] | Exhaustive technical documentation and API reference manual. | Markdown |
| [`.gitignore`] | Git exclusion patterns covering Rust target builds, Python caches, temporary logs, `.zip` archives, and `myrtle-main/`. | Git |

---

### 3.2 Data Directory (`data/`)

| File Path | Description & Content | Schema / Records |
|---|---|---|
| [`data/Automated_Operators.json`] | Canonical operator database containing 426+ operators. Each record includes base stats, skills (sp cost, duration, blackboard buffs), modules (X, Y, D, RA, IS), talents, and sub-archetype IDs. | JSON (`{ "operators": [ Operator ] }`) |
| [`data/Automated_Enemies.json`] | Canonical enemy database containing 1,698+ enemies. Each entry includes HP, DEF, RES, ATK, attack interval, weight, movement speed, dodge ratios, skill counts, revive flags, and threat classifications. | JSON (`[ EnemyData ]`) |
| [`data/classes.json`] | Subclass/Branch archetype blueprint defining standard base attack intervals, default target counts, attack ranges, and branch combat traits. | JSON (`{ "classes": [ ... ] }`) |
| [`data/Arknights_Tier_Lists_PDF.zip`] | Pre-compiled archive (22.20 MB) containing all 116 high-density analytical PDF tier lists (84 operator matrices + 32 enemy matrices). | ZIP Archive |

---

### 3.3 High-Performance Rust Engine (`rust_engine/`)

| File Path | Purpose | Key Structs / Functions / Crates |
|---|---|---|
| [`Cargo.toml`] | Crate specification file. Configures package metadata and pinned dependencies. | `axum` (0.8), `tokio` (1.53), `rayon` (1.12), `minijinja` (2.24), `serde` (1.0), `tower-http` (0.7) |
| [`src/main.rs`] | Server entry point. Registers all 18 Axum routes, binds the TCP listener, evaluates batch simulations, renders templates via Minijinja, executes Rayon multi-threaded tier list evaluations, and handles file uploads and exports. | `main`, `read_root`, `get_tierlist_data`, `get_enemy_tierlist_data`, `evaluate_single_operator`, `export_tierlist_csv`, `download_tierlists_pdf_zip` |
| [`src/core/mod.rs`] | Module registry declaring `data_loader`, `models`, `simulation`, and `enemy`. | Rust module exports |
| [`src/core/models.rs`] | Defines core domain data structures and mathematical stat calculators. Implements buff stacking (additive ratios, flat additions, true multipliers), ASPD/interval formulas, Physical/Arts EHP, and Hits-to-Kill. | `Operator`, `Skill`, `Module`, `Talent`, `Buff`, `final_atk()`, `final_def()`, `final_interval()`, `calculate_stat()`, `calculate_ehp_phys()`, `calculate_ehp_arts()`, `calculate_hits_to_kill()` |
| [`src/core/simulation.rs`] | Implements the 300-second discrete simulation loop. Models rotation timings, attack intervals, branch traits, instant skills, ammo mechanics, fast-redeploy retreat cycles, defeat penalties, and damage floors. | `SimulationEnvironment`, `state_rates()`, `run_5_minute_sim()`, `run_wave_sim()`, `run_boss_sim()` |
| [`src/core/data_loader.rs`]( | JSON ingestion and normalization layer. Parses raw operator files and blackboards into strongly-typed Rust structs. Normalizes module levels, SP charging types, and talent aura buffs. | `DataLoader`, `load_operators()`, `parse_blackboard_buffs()`, `get_operator()` |
| [`src/core/enemy.rs`] | Enemy data loading, category statistical aggregation, and Threat Score calculations. Provides baseline average enemy stats for `normal`, `elite`, and `boss` categories. | `EnemyData`, `AverageEnemy`, `calculate_average_enemy()`, `get_enemy_by_category()`, `calculate_threat_score()` |

---

### 3.4 Data Pipeline & Automation Scripts (`Scripts/`)

| File Path | Language & Libraries | Purpose & Execution |
|---|---|---|
| [`Scripts/generate_tierlist_pdfs.py`] | Python 3, `reportlab`, `urllib` | Generates 116 analytical PDF reports (84 operator matrices across 6 target categories × 14 ranking metrics, plus 32 enemy matrices across 4 categories × 8 metrics). Employs response caching and packages results into `Arknights_Tier_Lists_PDF.zip`. |
| [`Scripts/extract_operators.py`] | Python 3, `json` | Extracts raw operator data, attributes, skills, and branches from the `myrtle-main` repository into standardized intermediate structures. |
| [`Scripts/extract_enemies.py`] | Python 3, `json` | Extracts enemy statistics, combat levels, skill counts, and phases from raw gamedata into `Automated_Enemies.json`. |
| [`Scripts/update_skills_talents.py`] | Python 3, `json`, `urllib` | Fetches up-to-date talent descriptions, module stat increments, and blackboard buff parameters, updating `Automated_Operators.json`. |
| [`Scripts/fixup_cn_operators.py`] | Python 3, `json` | Enriches CN server exclusive operators with localized names, missing branch tags, and module attributes. |
| [`Scripts/add_ids.py`] | Python 3, `json` | Injects canonical sequential numeric IDs to operators and skills lacking unique integer identifiers. |
| [`Scripts/add_rarity.py`] | Python 3, `json` | Injects 1-star through 6-star rarity attributes into operator records. |
| [`Scripts/recover_images.py`] | Python 3, `os` | Audits portrait files in `static/avatars/` and re-links missing image paths in the operator database. |

---

### 3.5 Web Templates (`templates/`)

| File Path | Template Engine | Role & Contents |
|---|---|---|
| [`templates/base.html`] | Minijinja / HTML5 | Master layout shell providing standard navigation bar, header badges, responsive meta viewport, and favicon declarations. |
| [`templates/dashboard.html`] | Minijinja / HTML5 | Main landing view showing engine summary cards, database stats, and direct links to the Tier Lists and Editors. |
| [`templates/tierlist.html`] | Minijinja / HTML5 / JS | Interactive Operator Tier List dashboard. Features a dynamic SVG Cumulative Distribution Function (CDF) curve, ranking metric selectors, category buttons, profession filters, card views, and PDF/CSV download triggers. |
| [`templates/enemy_tierlist.html`] | Minijinja / HTML5 / JS | Interactive Enemy Threat Tier List dashboard. Renders enemy portrait cards with threat badges, DPS ratings, DEF/RES meters, and special mechanic indicators. |
| [`templates/editor.html`] | Minijinja / HTML5 / JS | Operator Sandbox Editor. Allows modifying base stats, adding/editing custom buffs, selecting skills and modules, and executing real-time 300s simulations with SVG rotation graphs. |
| [`templates/enemy_editor.html`] | Minijinja / HTML5 / JS | Enemy Sandbox Editor. Form-based interface to create, modify, or delete enemy entries. |

---

### 3.6 Static Assets (`static/`)

| Directory / File Path | Type | Role |
|---|---|---|
| [`static/css/style.css`] | CSS3 | Dark-mode glassmorphism design system. Defines responsive flex/grid layouts, animated progress bars, tier badge color hierarchies (`OP`, `S`, `A`, `B`, `C`, `D`), and modals. |
| [`static/js/main.js`] | JavaScript | Core client-side utility script for asynchronous form submission and dynamic DOM updates. |
| `static/avatars/` | PNG Images | High-resolution portrait illustrations for 490+ operators. |
| `static/enemy_avatars/` | PNG Images | Portrait illustrations for 1,600+ enemies. |

---

## 4. Mathematical Modeling & Combat Formulas

The engine replaces approximations with strict, deterministic implementations of canonical *Arknights* combat formulas.

### 4.1 Attack Power ($\text{ATK}$) & Stat Stacking

Operator stats follow a strict three-tier calculation hierarchy:

$$\text{Final ATK} = \left( \text{Base ATK} \times (1 + \sum \text{Ratio Buffs}) + \sum \text{Flat Buffs} + \text{Inspiration} \right) \times \prod \text{Multipliers}$$

Where:
1. **Ratio Buffs**: Standard percentage modifiers from talents, skills, and modules (e.g. `+50% ATK`).
2. **Flat Buffs & Inspiration**: Direct additive increments that bypass ratio scaling (e.g. Bard Inspiration from Skadi Alter).
3. **True Multipliers**: Multiplicative bonuses that scale the entire accumulated value (e.g. `atk_scale` modifiers).

---

### 4.2 Attack Interval & Attack Speed ($\text{ASPD}$)

The actual attack interval between consecutive strikes is governed by:

$$\text{Final Interval} = \max\left(0.20, \frac{\text{Base Interval} \times 100}{100 + \text{ASPD}}\right)$$

- Base interval is determined by the operator's branch archetype in [`classes.json`] and modified by module traits.
- Minimum interval is clamped to **0.20s** (equivalent to the 6-frame game tick limit).

---

### 4.3 Damage Types & The 5% Hardcoded Floor

#### Physical Damage
Physical damage subtracts enemy Defense ($\text{DEF}$) directly from the attack power, subject to the hardcoded 5% floor:

$$\text{Physical Damage} = \max\left(\text{Final ATK} \times 0.05, \, \text{Final ATK} - \text{Enemy DEF}\right) \times (1 - \text{Phys Dodge})$$

#### Arts Damage
Arts damage is mitigated by enemy Arts Resistance ($\text{RES}$, expressed as a percentage):

$$\text{Arts Damage} = \max\left(\text{Final ATK} \times 0.05, \, \text{Final ATK} \times \left(1 - \frac{\text{Enemy RES}}{100}\right)\right) \times (1 - \text{Arts Dodge})$$

#### True Damage
True damage completely bypasses Defense, Resistance, and Dodge:

$$\text{True Damage} = \text{Final ATK}$$

#### Elemental Damage
Elemental damage builds an independent damage meter on the target, triggering Prismatic Elemental Fallout bursts (e.g., Necrosis, Burn, Corrosion) once accumulated damage reaches archetype threshold values.

---

### 4.4 Survivability & Bulk Metrics

#### Effective Health Pool ($\text{EHP}$)
$$\text{EHP}_{\text{Phys}} = \frac{\text{HP}}{1 - \min\left(0.95, \frac{\text{DEF}}{\text{DEF} + 1000}\right)}$$

$$\text{EHP}_{\text{Arts}} = \frac{\text{HP}}{1 - \min\left(0.95, \frac{\text{RES}}{100}\right)}$$

$$\text{Overall Survivability} = \left(\frac{\text{EHP}_{\text{Phys}} + \text{EHP}_{\text{Arts}}}{2}\right) \times (1 + \text{Healing Received Bonus})$$

#### Hits-to-Kill ($\text{HTK}$)
$$\text{HTK} = \frac{\text{HP}}{\max\left(\text{Incoming Damage per Hit} - \text{Regen per Interval}, \, \text{Incoming Damage per Hit} \times 0.05\right)}$$

Where:
- $\text{Incoming Damage per Hit} = \text{Enemy ATK} \times (1 - \text{Sanctuary Mitigation}) - \text{Operator DEF}$
- $\text{Regen per Interval} = \text{Regen per Second} \times \text{Enemy Attack Interval}$

---

### 4.5 300-Second Discrete Simulation Loop

Rather than averaging skill uptime mathematically ($\text{DPS} = \text{DPS}_{\text{skill}} \times u + \text{DPS}_{\text{base}} \times (1-u)$), [`SimulationEnvironment::run_5_minute_sim()`] advances time discretely over 300 seconds (5 minutes):

1. **SP Charging**:
   - **Automatic (Time)**: Accumulates $+1.0 \text{ SP/s}$ (augmented by SP recovery buffs like Ptilopsis or Mostima).
   - **Offensive (On-Hit)**: Grants $+1.0 \text{ SP}$ per completed basic attack strike.
   - **Defensive (On-Damaged)**: Grants $+1.0 \text{ SP}$ each time the operator takes damage.
2. **Instant Skills vs Toggle Skills**:
   - Instant offensive skills (`duration == -1.0` or single discharge) trigger for the duration of a single attack interval ($b_{\text{int}}$) before entering the recharge cycle.
   - Permanent toggle skills (e.g. Thorns S3, Mountain S2) remain in the active skill state indefinitely once fully charged.
3. **Ammo-Based Skills**:
   - Skills with finite ammunition counts (e.g. Ela S3, Ash S2, Pozëmka S3) fire their allocated ammo count at accelerated rates and transition to recharge immediately upon ammo exhaustion.
4. **Fast-Redeploy Retreat & Redeployment Cycles**:
   - Executors (Texas the Omertosa, Kirin X Yato, Phantom, Projekt Red) remain on the field for their active skill window ($3.5\text{s} - 10\text{s}$), deliver their burst, and immediately retreat to the bench for their redeployment cooldown ($14\text{s} - 18\text{s}$) before repeating the cycle.
5. **Combat Defeat Penalties**:
   - Operators taking more DPS than their HP and self-healing can sustain suffer combat defeats, incurring 15-second redeployment delays that reduce their `combat_uptime_factor` and penalize total yield.
6. **Duelist Defender Blocking, Weight Gating & Module Upgrades**:
   - Duelist Defenders (Eunectes, Aurora, Cement) possess the archetype trait: *"Only restores SP when blocking an enemy"*.
   - Canonical mechanics dictate that an operator cannot block an enemy whose weight exceeds their block count ($\text{Weight} > \text{Block}$).
   - Because Duelists have a base block count of $1$, when facing heavy targets (Elites with weight $\ge 3$ or Bosses with weight $\ge 5$), they cannot block them during the charging phase.
   - Without a module or with **MOD-Y** (which keeps `sp_recover_ratio = -0.999` and rewards +15% ATK/DEF while blocking), SP recovery drops to $0.0\text{ SP/s}$, preventing them from charging high-burst skills against unblockable heavier enemies and correctly evaluating them in their unbuffed state.
   - **MOD-X (DUA-X / HES-X) Trait Upgrade**: Upgrades the branch trait so that when not blocking an enemy, SP recovers at **20% of the normal rate** (`sp_recover_ratio = -0.8`, granting $0.20 \times (1 + \text{sp\_recovery\_per\_sec})$). This allows operators equipped with MOD-X to charge skills against heavy bosses even without blocking them, unlocking S3 activation (e.g. Eunectes achieving 2 full S3 casts and a Boss TTC of $656.2\text{s}$ vs $1800\text{s}$ timeout without MOD-X).

---

### 4.6 DP Generation Modeling

DP generation is tracked through exact talent triggers and skill executions:
- **Flagbearers (Myrtle, Elysium, Saileach)**: Total DP yield per cast ($14.0\text{ DP}$ for Myrtle, $18.0\text{ DP}$ for Elysium) is disbursed across the skill channeling window.
- **Pioneers (Texas, Siege, Courier)**: Instant DP bursts upon cast completion ($12.0\text{ DP}$ for Texas S2).
- **Agents (Cantabile, Ines)**: DP generated on each successful hit during active skill deployment.

---

## 5. Deployment, Build & Execution Guide

### 5.1 System Prerequisites
- **Rust Toolchain**: 1.75+ with Cargo ([rustup.rs](https://rustup.rs))
- **Python Runtime**: 3.10+ with `reportlab` installed:
  ```bash
  pip install reportlab
  ```

### 5.2 Building & Running

#### One-Click Launch (Windows)
Double-click [`ejecutar_programa.bat`] in the project root. This executes:
```cmd
cd rust_engine
cargo run --release
```

#### Manual Console Launch
```bash
cd rust_engine
cargo run --release
```

The server initializes on `http://127.0.0.1:8000`. Navigate to `http://127.0.0.1:8000/tierlist` in any modern web browser.

#### Environment Variables
- `PORT`: Overrides the default HTTP port (e.g., `set PORT=8080 && cargo run --release`).

---

## 6. Acknowledgements & Credits

- **Database Reference**: This analytical engine builds upon the open gamedata structures, formulas, and mechanics cataloged by **[myrtle.moe](https://myrtle.moe)**.
- **Intellectual Property**: *Arknights* is a registered trademark of **Hypergryph / Studio Montagne / Yostar**. All assets, illustrations, and gamedata belong to their respective owners.
