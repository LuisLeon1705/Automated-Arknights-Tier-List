// Hexagonal 6-axis radar chart for a team's axis scores, styled after the classic
// "spec sheet" radar (grade letters ringing a shaded polygon). Renders raw SVG, no chart library.

const TEAM_AXES = [
    { key: 'boss_killing', label: 'BOSS KILLING', abbr: 'BK' },
    { key: 'utility', label: 'UTILITY', abbr: 'UT' },
    { key: 'consistency', label: 'CONSISTENCY', abbr: 'CO' },
    { key: 'reliability', label: 'RELIABILITY', abbr: 'RE' },
    { key: 'resistance', label: 'RESISTANCE', abbr: 'RS' },
    { key: 'lane_holding', label: 'LANE HOLDING', abbr: 'LH' },
];

const GRADE_COLOR = { A: '#f59e0b', B: '#22c55e', C: '#3b82f6', D: '#94a3b8', E: '#64748b' };

// `mini: true` renders a smaller chart with short 2-letter axis abbreviations + grade letters
// instead of the full spelled-out labels (used on Team Tier List cards, where full labels
// wouldn't fit) — it still always shows WHAT each point of the hexagon means and how it graded,
// never a bare unlabeled shape.
function drawTeamRadar(container, axisScores, grades, opts) {
    opts = opts || {};
    const mini = !!opts.mini;
    const size = 420;
    const cx = size / 2, cy = size / 2;
    const rMax = 150;
    const n = TEAM_AXES.length;
    const angleFor = (i) => (Math.PI * 2 * i) / n - Math.PI / 2;

    const point = (i, r) => {
        const a = angleFor(i);
        return [cx + r * Math.cos(a), cy + r * Math.sin(a)];
    };

    // Ring grid (4 rings at 25/50/75/100%)
    let rings = '';
    [0.25, 0.5, 0.75, 1.0].forEach(frac => {
        const pts = TEAM_AXES.map((_, i) => point(i, rMax * frac).join(',')).join(' ');
        rings += `<polygon points="${pts}" fill="none" stroke="var(--border)" stroke-width="1" opacity="0.5"/>`;
    });

    // Spokes
    let spokes = '';
    TEAM_AXES.forEach((_, i) => {
        const [x, y] = point(i, rMax);
        spokes += `<line x1="${cx}" y1="${cy}" x2="${x}" y2="${y}" stroke="var(--border)" stroke-width="1" opacity="0.5"/>`;
    });

    // Data polygon
    const dataPts = TEAM_AXES.map((ax, i) => {
        const val = Math.max(0, Math.min(100, axisScores[ax.key] || 0));
        return point(i, rMax * (val / 100)).join(',');
    }).join(' ');

    // Labels + grade letters — always present (mini just uses the short abbreviation + a
    // smaller font instead of the full axis name), so the shape is never meaningless on its own.
    // Font sizes are in SVG viewBox units (this chart is always drawn on a 420-wide viewBox and
    // scaled down by CSS `max-width`), so the EFFECTIVE rendered size depends on both this number
    // and the container's max-width below — a "9" here at max-width:190px renders as an illegible
    // ~4px. Sized so mini-mode text reads at roughly the same real pixel size as normal body text.
    let labels = '';
    const labelR = rMax + (mini ? 34 : 46);
    const nameFontSize = mini ? 17 : 11;
    const gradeFontSize = mini ? 26 : 20;
    TEAM_AXES.forEach((ax, i) => {
        const [lx, ly] = point(i, labelR);
        const gradeChar = (grades && grades[ax.key]) || 'E';
        const color = GRADE_COLOR[gradeChar] || '#94a3b8';
        const anchor = Math.abs(Math.cos(angleFor(i))) < 0.3 ? 'middle' : (Math.cos(angleFor(i)) > 0 ? 'start' : 'end');
        const nameText = mini ? ax.abbr : ax.label;
        labels += `
            <text x="${lx}" y="${ly - (mini ? 10 : 10)}" text-anchor="${anchor}" font-size="${nameFontSize}" fill="var(--text-secondary)" font-weight="700">${nameText}</text>
            <text x="${lx}" y="${ly + (mini ? 18 : 14)}" text-anchor="${anchor}" font-size="${gradeFontSize}" fill="${color}" font-weight="800">${gradeChar}</text>
        `;
    });

    const viewBoxH = size + (mini ? 40 : 20);
    container.innerHTML = `
        <svg viewBox="0 0 ${size} ${viewBoxH}" style="width:100%; max-width:${mini ? 270 : 460}px; height:auto; overflow:visible;">
            ${rings}
            ${spokes}
            <polygon points="${dataPts}" fill="#38bdf8" fill-opacity="0.28" stroke="#38bdf8" stroke-width="2"/>
            ${labels}
        </svg>
    `;
}
