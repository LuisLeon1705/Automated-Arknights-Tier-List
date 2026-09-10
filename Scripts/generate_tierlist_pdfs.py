import os
import sys
import json
import zipfile
import urllib.request
import tempfile
import math
from datetime import datetime

import reportlab
from reportlab.lib.pagesizes import letter, landscape
from reportlab.platypus import SimpleDocTemplate, Paragraph, Spacer, Table, TableStyle, PageBreak
from reportlab.lib.styles import getSampleStyleSheet, ParagraphStyle
from reportlab.lib import colors
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont

sys.stdout.reconfigure(encoding='utf-8')

# Try to register a font with Chinese/Unicode support if available
FONT_NAME = 'Helvetica'
FONT_BOLD = 'Helvetica-Bold'
for font_path in [r'C:\Windows\Fonts\msyh.ttc', r'C:\Windows\Fonts\simsun.ttc', r'C:\Windows\Fonts\arial.ttf']:
    if os.path.exists(font_path):
        try:
            pdfmetrics.registerFont(TTFont('CustomFont', font_path))
            FONT_NAME = 'CustomFont'
            FONT_BOLD = 'CustomFont'
            print(f"Registered font: {font_path}")
            break
        except Exception as e:
            print(f"Could not load font {font_path}: {e}")

# Color Palette
COLOR_BG_HEADER = colors.HexColor('#1e293b')
COLOR_ROW_ALT1 = colors.HexColor('#0f172a')
COLOR_ROW_ALT2 = colors.HexColor('#1e293b')
COLOR_TEXT_MAIN = colors.HexColor('#f8fafc')
COLOR_TEXT_MUTED = colors.HexColor('#94a3b8')
COLOR_BORDER = colors.HexColor('#334155')

TIER_COLORS = {
    'OP': colors.HexColor('#ef4444'),
    'S': colors.HexColor('#f97316'),
    'A': colors.HexColor('#eab308'),
    'B': colors.HexColor('#22c55e'),
    'C': colors.HexColor('#06b6d4'),
    'D': colors.HexColor('#3b82f6'),
    'E': colors.HexColor('#8b5cf6'),
    'F': colors.HexColor('#64748b')
}

def fetch_json(url):
    try:
        req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
        with urllib.request.urlopen(req, timeout=30) as res:
            return json.loads(res.read())
    except Exception as e:
        print(f"Fetch failed for {url}: {e}")
        return None

def build_pdf_document(filepath, title, subtitle, columns, col_widths, data_rows):
    doc = SimpleDocTemplate(
        filepath,
        pagesize=landscape(letter),
        leftMargin=20,
        rightMargin=20,
        topMargin=25,
        bottomMargin=25
    )
    
    styles = getSampleStyleSheet()
    
    title_style = ParagraphStyle(
        'DocTitle',
        fontName=FONT_BOLD,
        fontSize=14,
        leading=17,
        textColor=colors.HexColor('#38bdf8'),
        spaceAfter=3
    )
    subtitle_style = ParagraphStyle(
        'DocSubtitle',
        fontName=FONT_NAME,
        fontSize=9,
        leading=12,
        textColor=COLOR_TEXT_MUTED,
        spaceAfter=10
    )
    
    cell_style = ParagraphStyle(
        'CellText',
        fontName=FONT_NAME,
        fontSize=7.5,
        leading=9,
        textColor=COLOR_TEXT_MAIN
    )
    cell_bold = ParagraphStyle(
        'CellBold',
        fontName=FONT_BOLD,
        fontSize=7.5,
        leading=9,
        textColor=COLOR_TEXT_MAIN
    )
    
    elements = []
    elements.append(Paragraph(title, title_style))
    elements.append(Paragraph(f"{subtitle} | Generado el {datetime.now().strftime('%Y-%m-%d %H:%M')}", subtitle_style))
    
    # Table header
    table_data = [[Paragraph(f"<b>{c}</b>", cell_bold) for c in columns]]
    
    # Table rows
    table_styles = [
        ('BACKGROUND', (0, 0), (-1, 0), COLOR_BG_HEADER),
        ('ALIGN', (0, 0), (-1, -1), 'LEFT'),
        ('VALIGN', (0, 0), (-1, -1), 'MIDDLE'),
        ('GRID', (0, 0), (-1, -1), 0.5, COLOR_BORDER),
        ('TOPPADDING', (0, 0), (-1, -1), 3),
        ('BOTTOMPADDING', (0, 0), (-1, -1), 3),
        ('LEFTPADDING', (0, 0), (-1, -1), 4),
        ('RIGHTPADDING', (0, 0), (-1, -1), 4),
    ]
    
    for row_idx, row in enumerate(data_rows, start=1):
        formatted_row = []
        tier = str(row[1]) if len(row) > 1 else ''
        t_color = TIER_COLORS.get(tier, colors.HexColor('#64748b'))
        
        for col_idx, val in enumerate(row):
            if col_idx == 1 and tier in TIER_COLORS:
                p = Paragraph(f"<font color='{t_color.hexval()}'><b>{val}</b></font>", cell_bold)
            elif col_idx == 0 or col_idx == 2:
                p = Paragraph(str(val), cell_bold)
            else:
                p = Paragraph(str(val), cell_style)
            formatted_row.append(p)
            
        table_data.append(formatted_row)
        bg = COLOR_ROW_ALT1 if row_idx % 2 == 1 else COLOR_ROW_ALT2
        table_styles.append(('BACKGROUND', (0, row_idx), (-1, row_idx), bg))

    t = Table(table_data, colWidths=col_widths, repeatRows=1)
    t.setStyle(TableStyle(table_styles))
    elements.append(t)
    
    doc.build(elements)
    print(f"Generated PDF: {os.path.basename(filepath)} ({len(data_rows)} rows)", flush=True)

def main():
    out_dir = tempfile.mkdtemp(prefix="arknights_pdfs_")
    print(f"Temporary output dir: {out_dir}", flush=True)
    
    base_url = "http://127.0.0.1:8000"
    is_server_online = False
    try:
        urllib.request.urlopen(f"{base_url}/api/enemy_tierlist_data?category=boss", timeout=2)
        is_server_online = True
    except Exception:
        pass
    
    print(f"Server is online: {is_server_online}", flush=True)
    
    generated_pdfs = []
    
    # -------------------------------------------------------------
    # 1. GENERATE OPERATOR TIER LISTS
    # -------------------------------------------------------------
    # Define target categories (evaluation contexts) and ranking metrics (sorting criteria).
    # This dynamic approach generates all required combinations (6 categories × 11 metrics = 66 PDFs).

    target_categories = [
        ("general", "General"),
        ("normal", "Normal"),
        ("elite", "Elite"),
        ("boss", "Boss"),
        ("ra", "Reclamation Algorithm"),
        ("is", "Integrated Strategies"),
        ("dp", "DP Generators"),
    ]

    ranking_metrics = [
        ("score", "Overall Combat Power", "Score"),
        ("wave_ttc_inv", "Wave Clearer", "Wave TTC"),
        ("boss_ttc_inv", "Boss Killer", "Boss TTC"),
        ("phys_dmg", "Physical DPS", "Phys DPS"),
        ("arts_dmg", "Arts DPS", "Arts DPS"),
        ("elemental_dmg", "Elemental DPS", "Ele DPS"),
        ("heal", "Effective HPS", "Effective HPS"),
        ("heal_phys", "Physical Healing & Mitigation", "Phys Mitig"),
        ("heal_arts", "Arts Healing & Mitigation", "Arts Mitig"),
        ("heal_ele", "Elemental Healing", "Ele Heal"),
        ("support", "Support Utility", "Support"),
        ("surv", "Overall Survivability", "Surv Score"),
        ("phys_surv", "Physical Survivability", "Phys EHP"),
        ("arts_surv", "Arts Survivability", "Arts EHP"),
        ("dp", "DP Generation", "Total DP"),
    ]

    operator_variations = []
    file_counter = 1
    for cat_key, cat_name in target_categories:
        for metric_key, metric_title, metric_col in ranking_metrics:
            filename = f"{file_counter:02d}_TierList_{cat_name.replace(' ', '_')}_{metric_title.replace(' ', '_')}.pdf"
            title = f"TIER LIST - {cat_name.upper()} ({metric_title})"
            subtitle = f"Ranking de operadores en entorno {cat_name} ordenado por {metric_title}"
            operator_variations.append((filename, cat_key, metric_key, title, subtitle, metric_col))
            file_counter += 1
    
    op_columns = ["Rank", "Tier", "Operador", "Rareza", "Habilidad", "Módulo", "Métrica Clave", "DPS Físico", "DPS Artes", "DPS Ele", "HPS", "EHP Físico"]
    op_widths = [35, 30, 110, 45, 120, 80, 75, 55, 55, 50, 50, 50]
    
    # Pre-fetch operator data for each unique category (using full detailed variations)
    cached_op_data = {}
    if is_server_online:
        for cat_key, cat_name in target_categories:
            print(f"Pre-fetching operator data for category: {cat_name}...", flush=True)
            res = fetch_json(f"{base_url}/api/tierlist_data?category={cat_key}")
            cached_op_data[cat_key] = (res.get("detailed") or res.get("general", [])) if res else []

    for filename, cat, sort_metric, title, subtitle, metric_col_name in operator_variations:
        columns = list(op_columns)
        columns[6] = metric_col_name
        pdf_path = os.path.join(out_dir, filename)
        
        data_rows = []
        if is_server_online and cat in cached_op_data:
            ops = cached_op_data[cat]
            
            def get_sort_val(item):
                if sort_metric == "boss_ttc_inv":
                    t = float(item.get("boss_ttc", 9999.0))
                    return -t if t > 0 else -9999.0
                elif sort_metric == "wave_ttc_inv":
                    t = float(item.get("wave_ttc", 9999.0))
                    return -t if t > 0 else -9999.0
                elif sort_metric == "phys_dmg":
                    return float(item.get("phys_dmg", 0)) / 300.0
                elif sort_metric == "arts_dmg":
                    return float(item.get("arts_dmg", 0)) / 300.0
                elif sort_metric == "elemental_dmg":
                    return float(item.get("elemental_dmg", 0)) / 300.0
                elif sort_metric == "heal":
                    return float(item.get("heal", 0)) / 300.0
                elif sort_metric == "heal_phys":
                    return float(item.get("heal_phys", 0)) / 300.0
                elif sort_metric == "heal_arts":
                    return float(item.get("heal_arts", 0)) / 300.0
                elif sort_metric == "heal_ele":
                    return float(item.get("heal_ele", 0)) / 300.0
                elif sort_metric == "support":
                    return float(item.get("support", 0))
                elif sort_metric == "surv":
                    return float(item.get("surv", 0))
                elif sort_metric == "phys_surv":
                    return float(item.get("phys_surv", 0))
                elif sort_metric == "arts_surv":
                    return float(item.get("arts_surv", 0))
                elif sort_metric == "dp":
                    return float(item.get("dp", 0))
                else:
                    return float(item.get("score", 0))

            ops_sorted = sorted(ops, key=get_sort_val, reverse=True)
            total_ops = len(ops_sorted)
                
            for idx, op in enumerate(ops_sorted):
                pct = (idx + 1) / total_ops
                tier = "OP" if pct <= 0.03 else ("S" if pct <= 0.12 else ("A" if pct <= 0.28 else ("B" if pct <= 0.50 else ("C" if pct <= 0.72 else ("D" if pct <= 0.86 else ("E" if pct <= 0.95 else "F"))))))
                
                if sort_metric == "boss_ttc_inv":
                    b_ttc = float(op.get("boss_ttc", 0))
                    m_val = f"{b_ttc:.1f}s" if b_ttc > 0 else "N/A"
                elif sort_metric == "wave_ttc_inv":
                    w_ttc = float(op.get("wave_ttc", 0))
                    m_val = f"{w_ttc:.1f}s" if w_ttc > 0 else "N/A"
                elif sort_metric in ["phys_dmg", "arts_dmg", "elemental_dmg", "heal", "heal_phys", "heal_arts", "heal_ele"]:
                    v = float(op.get(sort_metric, 0)) / 300.0
                    m_val = f"{v:,.0f}"
                elif sort_metric == "support":
                    m_val = f"{float(op.get('support', 0)):,.0f}"
                elif sort_metric in ["surv", "phys_surv", "arts_surv"]:
                    m_val = f"{float(op.get(sort_metric, 0)):,.0f}"
                elif sort_metric == "dp":
                    v = float(op.get("dp", 0))
                    m_val = f"{v:,.0f} DP ({v/300.0:.2f}/s)"
                else:
                    m_val = f"{float(op.get('score', 0)):,.0f}"

                rarity = f"{op.get('rarity', 6)}★"
                p_dps = f"{float(op.get('phys_dmg', 0)) / 300.0:,.0f}"
                a_dps = f"{float(op.get('arts_dmg', 0)) / 300.0:,.0f}"
                e_dps = f"{float(op.get('elemental_dmg', 0)) / 300.0:,.0f}"
                hps = f"{float(op.get('heal', 0)) / 300.0:,.0f}"
                ehp = f"{float(op.get('phys_surv', 0)):,.0f}"
                
                sk_tag = op.get("skill_tag") or ("RAW" if op.get("skill_name") == "RAW" else "SKILL")
                mod_tag = op.get("module_tag") or ("NO MOD" if not op.get("module_name") or op.get("module_name") == "No Module" else "MOD")
                sk_text = f"[{sk_tag}] {op.get('skill_name', 'RAW')}"
                mod_text = f"[{mod_tag}] {op.get('module_name', 'None')}"

                data_rows.append([
                    str(idx + 1),
                    tier,
                    str(op.get("operator_name", "")),
                    rarity,
                    sk_text,
                    mod_text,
                    m_val,
                    p_dps,
                    a_dps,
                    e_dps,
                    hps,
                    ehp
                ])
        
        build_pdf_document(pdf_path, title, subtitle, columns, op_widths, data_rows)
        generated_pdfs.append(pdf_path)

    # -------------------------------------------------------------
    # 2. GENERATE ENEMY TIER LISTS
    # -------------------------------------------------------------
    en_columns = ["Rank", "Tier", "ID", "Nombre", "Tipo", "Nivel de Amenaza", "HP", "ATK", "DEF", "RES", "INT", "Inmunidades"]
    en_widths = [35, 30, 110, 110, 50, 80, 60, 50, 50, 40, 40, 95]

    enemy_categories = [
        ("general", "General"),
        ("normal", "Normal"),
        ("elite", "Elite"),
        ("boss", "Boss"),
    ]

    enemy_metrics = [
        ("threat_score", "Threat Score", "Threat Score"),
        ("hp", "HP", "HP"),
        ("ehp", "EHP", "EHP"),
        ("dps", "DPS", "DPS"),
        ("atk", "ATK", "ATK"),
        ("def", "DEF", "DEF"),
        ("res", "RES", "RES"),
        ("weight", "Weight", "Weight"),
    ]

    enemy_variations = []
    file_counter = 1
    for cat_key, cat_name in enemy_categories:
        for metric_key, metric_title, metric_col in enemy_metrics:
            filename = f"{file_counter:02d}_Enemy_TierList_{cat_name}_{metric_title}.pdf"
            title = f"TIER LIST DE AMENAZA - {cat_name.upper()} ({metric_title})"
            subtitle = f"Ranking de enemigos {cat_name.lower()} ordenado por {metric_title.lower()}"
            enemy_variations.append((filename, cat_key, metric_key, title, subtitle, metric_col))
            file_counter += 1

    # Pre-fetch enemy data for each unique category
    cached_enemy_data = {}
    if is_server_online:
        for cat_key, cat_name in enemy_categories:
            api_cat = "all" if cat_key == "general" else cat_key
            print(f"Pre-fetching enemy data for category: {cat_name}...", flush=True)
            res = fetch_json(f"{base_url}/api/enemy_tierlist_data?category={api_cat}")
            cached_enemy_data[cat_key] = res.get("enemies", []) if res else []

    for filename, cat, sort_metric, title, subtitle, metric_col_name in enemy_variations:
        pdf_path = os.path.join(out_dir, filename)
        data_rows = []

        if is_server_online and cat in cached_enemy_data:
            enemies = cached_enemy_data[cat]
            def get_sort_val(e):
                if sort_metric == "threat_score":
                    return float(e.get("threat_score", 0))
                elif sort_metric == "hp":
                    return float(e.get("hp", 0))
                elif sort_metric == "ehp":
                    return float(e.get("ehp", e.get("hp", 0)))
                elif sort_metric == "dps":
                    return float(e.get("dps", 0))
                elif sort_metric == "atk":
                    return float(e.get("atk", 0))
                elif sort_metric == "def":
                    return float(e.get("def", 0))
                elif sort_metric == "res":
                    return float(e.get("res", 0))
                elif sort_metric == "weight":
                    return float(e.get("weight", 0))
                else:
                    return 0.0
            enemies_sorted = sorted(enemies, key=get_sort_val, reverse=True)
            total_en = len(enemies_sorted)
            for idx, e in enumerate(enemies_sorted):
                pct = (idx + 1) / (total_en or 1)
                if pct <= 0.03: tier = 'OP'
                elif pct <= 0.12: tier = 'S'
                elif pct <= 0.28: tier = 'A'
                elif pct <= 0.50: tier = 'B'
                elif pct <= 0.72: tier = 'C'
                elif pct <= 0.86: tier = 'D'
                elif pct <= 0.95: tier = 'E'
                else: tier = 'F'
                t_type = e.get("tier_type") or e.get("category") or "NORMAL"
                imms = []
                if e.get("immune_stun"): imms.append("STUN")
                if e.get("immune_silence"): imms.append("SIL")
                if e.get("immune_freeze"): imms.append("FRZ")
                if e.get("immune_sleep"): imms.append("SLP")
                if e.get("immune_levitate"): imms.append("LEV")
                imm_str = ", ".join(imms) if imms else "Ninguna"
                data_rows.append([
                    str(idx + 1),
                    tier,
                    str(e.get("id", "")),
                    str(e.get("name", "")),
                    t_type,
                    f"{float(e.get('threat_score', 0)):,.0f}",
                    f"{float(e.get('hp', 0)):,.0f}",
                    f"{float(e.get('atk', 0)):.0f}",
                    f"{float(e.get('def', 0)):.0f}",
                    f"{float(e.get('res', 0)):.0f}",
                    f"{float(e.get('attack_interval', 2.5)):.1f}s",
                    imm_str,
                ])

        build_pdf_document(pdf_path, title, subtitle, en_columns, en_widths, data_rows)
        generated_pdfs.append(pdf_path)

    # -------------------------------------------------------------
    # 3. PACK INTO ZIP FILE
    # -------------------------------------------------------------
    script_dir = os.path.dirname(os.path.abspath(__file__))
    project_root = os.path.dirname(script_dir) if os.path.basename(script_dir).lower() == "scripts" else script_dir
    out_zip_data = os.path.abspath(os.path.join(project_root, "data", "Arknights_Tier_Lists_PDF.zip"))
    os.makedirs(os.path.dirname(out_zip_data), exist_ok=True)
    
    with zipfile.ZipFile(out_zip_data, 'w', compression=zipfile.ZIP_DEFLATED) as zipf:
        for pdf_file in generated_pdfs:
            zipf.write(pdf_file, arcname=os.path.basename(pdf_file))
            
    print(f"\n=======================================================")
    print(f"PACKED {len(generated_pdfs)} PDFS INTO: {out_zip_data}")
    print(f"ZIP FILE SIZE: {os.path.getsize(out_zip_data) / 1024 / 1024:.2f} MB")
    print(f"=======================================================\n")
    
    # Cleanup temp dir
    for pdf_file in generated_pdfs:
        try: os.remove(pdf_file)
        except Exception: pass
    try: os.rmdir(out_dir)
    except Exception: pass

if __name__ == "__main__":
    main()
