//! The planning report as a self-contained HTML page, in the style of the other reports:
//! inline styles and SVG charts, no scripts, light and dark themes.

use std::fmt::Write as _;

use finalytics_plan::{Mix, Month, Status};

use crate::backtest_html::{Frame, colour, frame, section};
use crate::html::{esc, on, ticks, tiles};
use crate::planning::{
    IMPORTANT, Kind, MixRow, Money, NOT_ADVICE, PlanReport, a_chance, levers, levers_heading,
    levers_note, method_notes, outcome_money, pc, pc1, status_style, steps, steps_note,
    stress_meaning, stress_rows,
};

/// The planning page's styles, added to the reports' own.
const STYLE: &str = include_str!("../assets/planning.css");

/// The most points drawn along a line of the chart of the value over time.
const POINTS: usize = 150;

// ── Charts ───────────────────────────────────────────────────────────────────────────────────

/// The chance as a ring, with a tick at the chance wanted and an arc over the on-track zone.
fn ring(chance: f64, wanted: f64, spare: f64, class: &str) -> String {
    use std::f64::consts::PI;
    let (r, cx) = (54.0, 68.0);
    let c = 2.0 * PI * r;
    let a = (-90.0 + 360.0 * wanted).to_radians();
    let (x1, y1) = (cx + (r - 11.0) * a.cos(), cx + (r - 11.0) * a.sin());
    let (x2, y2) = (cx + (r + 11.0) * a.cos(), cx + (r + 11.0) * a.sin());
    let ro = r + 10.0;
    let co = 2.0 * PI * ro;
    format!(
        r#"<svg class="ring" viewBox="0 0 136 136" role="img" aria-label="{chance} chance; you want {want}"><circle cx="{cx}" cy="{cx}" r="{r}" style="fill:none;stroke:var(--grid);stroke-width:12"/><circle cx="{cx}" cy="{cx}" r="{ro}" transform="rotate(-90 {cx} {cx})" style="fill:none;stroke:var(--good);stroke-width:3;stroke-dasharray:0 {d1:.1} {d2:.1} {co:.1}"><title>On track from {want}; room to spare from {up}</title></circle><circle cx="{cx}" cy="{cx}" r="{r}" transform="rotate(-90 {cx} {cx})" style="fill:none;stroke:var(--{class});stroke-width:12;stroke-linecap:round;stroke-dasharray:{arc:.1} {c:.1}"/><line x1="{x1:.1}" y1="{y1:.1}" x2="{x2:.1}" y2="{y2:.1}" style="stroke:var(--ink);stroke-width:2.5"><title>You want {want}</title></line><text x="{cx}" y="{t1}" text-anchor="middle" style="fill:var(--ink);font-size:30px;font-weight:650">{chance}</text><text x="{cx}" y="{t2}" text-anchor="middle" style="fill:var(--ink2);font-size:12px">chance</text></svg>"#,
        chance = pc(chance),
        want = pc(wanted),
        up = pc(spare),
        d1 = co * wanted,
        d2 = co * (spare - wanted).max(0.0),
        arc = c * chance,
        t1 = cx + 4.0,
        t2 = cx + 24.0,
    )
}

/// The weak to strong range of the value at the end, with the average market as a dot and the
/// goal, if any, as a mark.
fn range_bar(m: Money, weak: f64, average: f64, strong: f64, goal: Option<f64>) -> String {
    let w = 360.0;
    let lo_value = goal.map_or(weak, |g| g.min(weak)).max(0.0);
    let hi_value = goal.map_or(strong, |g| g.max(strong));
    let pad = (hi_value - lo_value).max(hi_value.abs() * 0.1).max(1.0) * 0.12;
    let (lo, hi) = ((lo_value - pad).max(0.0), hi_value + pad);
    let x = |v: f64| 8.0 + (v - lo) / (hi - lo) * (w - 16.0);
    let mut out = format!(
        r#"<svg viewBox="0 0 {w} 62" style="margin-top:10px" role="img" aria-label="Weak to strong market{}"><line x1="8" x2="{}" y1="38" y2="38" style="stroke:var(--grid);stroke-width:8;stroke-linecap:round"/><line x1="{:.1}" x2="{:.1}" y1="38" y2="38" style="stroke:var(--s1);stroke-width:8;stroke-linecap:round;opacity:.45"/><circle cx="{:.1}" cy="38" r="7" style="fill:var(--s1);stroke:var(--surface);stroke-width:2"/>"#,
        if goal.is_some() {
            " against the goal"
        } else {
            ""
        },
        w - 8.0,
        x(weak),
        x(strong),
        x(average)
    );
    let mut label = |v: f64, text: &str| {
        let xv = x(v);
        let near = goal.is_some_and(|g| (xv - x(g)).abs() < 34.0);
        let (xt, anchor) = match goal {
            Some(g) if near && xv <= x(g) => (xv - 6.0, "end"),
            Some(_) if near => (xv + 6.0, "start"),
            _ => (xv, "middle"),
        };
        let _ = write!(
            out,
            r#"<text x="{xt:.1}" y="58" text-anchor="{anchor}" class="tick">{text}</text>"#
        );
    };
    label(weak, "Weak");
    label(strong, "Strong");
    if let Some(g) = goal {
        let xg = x(g);
        let _ = write!(
            out,
            r#"<line x1="{xg:.1}" x2="{xg:.1}" y1="24" y2="52" style="stroke:var(--ink);stroke-width:2"/><text x="{:.1}" y="16" text-anchor="middle" class="annb">Goal {}</text>"#,
            xg.clamp(40.0, w - 40.0),
            esc(&m.short(g))
        );
    }
    out.push_str("</svg>");
    out
}

/// How long the money lasts in the strong, average and weak markets, from the first payment to
/// the last.
fn lifelines(start: Month, end: Month, out: [Option<Month>; 3]) -> String {
    let w = 360.0;
    let t1 = start.months_until(end) as f64 + 1.0;
    let x =
        |m: Month| 110.0 + (f64::from(start.months_until(m)) / t1).clamp(0.0, 1.0) * (w - 118.0);
    let mut svg = format!(
        r#"<svg viewBox="0 0 {w} 92" style="margin-top:12px" role="img" aria-label="How long the money lasts">"#
    );
    for (i, (name, runs_out)) in ["Strong", "Average", "Weak"].iter().zip(out).enumerate() {
        let y = 14.0 + i as f64 * 26.0;
        let last = runs_out.unwrap_or(end.plus(1));
        let colour = if runs_out.is_none() {
            "var(--s1)"
        } else {
            "var(--bad)"
        };
        let _ = write!(
            svg,
            r#"<text class="ann" x="0" y="{ty}">{name} market</text><line x1="{a:.1}" x2="{b:.1}" y1="{y}" y2="{y}" style="stroke:var(--grid);stroke-width:8;stroke-linecap:round"/><line x1="{a:.1}" x2="{e:.1}" y1="{y}" y2="{y}" style="stroke:{colour};stroke-width:8;stroke-linecap:round"/>"#,
            ty = y + 4.0,
            a = x(start),
            b = x(end.plus(1)),
            e = x(last),
        );
    }
    let _ = write!(
        svg,
        r#"<text class="tick" x="{:.1}" y="88">{}</text><text class="tick" x="{:.1}" y="88" text-anchor="end">{}</text></svg>"#,
        x(start),
        start.year(),
        x(end.plus(1)),
        end.year()
    );
    svg
}

/// The months to label along the chart's axis, as (month offset, label).
fn month_ticks(start: Month, months: i32) -> Vec<(i32, String)> {
    let last = start.plus(months);
    if months <= 24 {
        return (0..=months)
            .filter(|&t| matches!(start.plus(t).month(), 1 | 7))
            .map(|t| (t, start.plus(t).short_name()))
            .collect();
    }
    let years = f64::from(months) / 12.0;
    let step = [1, 2, 5, 10, 20, 25, 50]
        .into_iter()
        .find(|s| years / f64::from(*s) <= 6.5)
        .unwrap_or(50);
    (start.year()..=last.year())
        .filter(|y| y % step == 0)
        .filter_map(|y| {
            let t = start.months_until(Month::new(y, 1).ok()?);
            (0..=months).contains(&t).then(|| (t, y.to_string()))
        })
        .collect()
}

/// The value over time: 100 simulations spread from the best outcome to the worst, coloured by
/// whether they meet the goal, with the average market drawn over them.
fn scenarios_chart(r: &PlanReport) -> (String, usize) {
    let f = &r.figures;
    let m = r.money();
    let kind = r.kind();
    let h = 320.0;
    let (xa, xb, top, bot) = (56.0, 664.0, 14.0, h - 28.0);
    let months = f.start.months_until(f.end).max(1);
    let n = f.months.len();
    let stride = n.div_ceil(POINTS).max(1);
    let mut picks: Vec<usize> = (0..n).step_by(stride).collect();
    if picks.last() != Some(&(n - 1)) {
        picks.push(n - 1);
    }
    let t_of = |k: usize| f64::from(f.start.months_until(f.months[k]));
    let x = |t: f64| xa + t / f64::from(months) * (xb - xa);
    let goal = match kind {
        Kind::Target { amount, .. } => Some(amount),
        _ => None,
    };
    // For payments, the upper quartile sets the scale, so that the money running short shows.
    let upper = |v: &finalytics_plan::Percentiles| match kind {
        Kind::Payments { .. } => v.p75 * 1.2,
        _ => v.p90,
    };
    let highest = f
        .values
        .iter()
        .map(upper)
        .fold(goal.map_or(0.0, |g| g * 1.25), f64::max)
        .max(1.0);
    let marks = ticks(0.0, highest, 4);
    let ymax = marks.last().copied().unwrap_or(highest).max(1.0);
    let y = |v: f64| bot - v.clamp(0.0, ymax * 1.08) / ymax * (bot - top);
    let path = |values: &[f64]| -> String {
        let mut d = String::new();
        for (i, &k) in picks.iter().enumerate() {
            let _ = write!(
                d,
                "{}{:.1},{:.1}",
                if i == 0 { "M" } else { " L" },
                x(t_of(k)),
                y(values[k])
            );
        }
        d
    };
    let mut s = format!(
        r#"<svg class="chart" viewBox="0 0 880 {h}" role="img" aria-label="The value over time in 100 simulated markets">"#
    );
    for v in &marks {
        let label = if *v == 0.0 { m.whole(0.0) } else { m.short(*v) };
        let _ = write!(
            s,
            r#"<line class="grid" x1="{xa}" x2="{xb}" y1="{yv:.1}" y2="{yv:.1}"/><text class="tick" x="48" y="{ty:.1}" text-anchor="end">{}</text>"#,
            esc(&label),
            yv = y(*v),
            ty = y(*v) + 4.0
        );
    }
    for (t, label) in month_ticks(f.start, months) {
        let _ = write!(
            s,
            r#"<text class="tick" x="{:.1}" y="{}" text-anchor="middle">{}</text>"#,
            x(f64::from(t)),
            h - 8.0,
            esc(&label)
        );
    }
    // A mark at the first payment, or at the largest single payment in or out.
    let marker = match kind {
        Kind::Payments {
            amount,
            every,
            start,
            ..
        } => Some((
            start,
            format!("Payments of {} a {} start", m.whole(amount), every.noun()),
        )),
        _ => f
            .flows
            .iter()
            .filter(|fl| fl.every.is_none())
            .max_by(|a, b| a.amount.total_cmp(&b.amount))
            .map(|fl| {
                let sign = if fl.saving { 1.0 } else { -1.0 };
                (
                    fl.first,
                    format!("{}, {}", fl.name, m.change(sign * fl.amount)),
                )
            }),
    };
    if let Some((month, label)) = marker {
        let xm = x(f64::from(f.start.months_until(month)));
        let (xt, anchor) = if xm > xb - 200.0 {
            (xm - 6.0, "end")
        } else {
            (xm + 6.0, "start")
        };
        let _ = write!(
            s,
            r#"<line class="ref" x1="{xm:.1}" x2="{xm:.1}" y1="{top}" y2="{bot}"/><text class="ann" x="{xt:.1}" y="{}" text-anchor="{anchor}">{}</text>"#,
            top + 12.0,
            esc(&label)
        );
    }
    let _ = write!(
        s,
        r#"<clipPath id="plan-lines"><rect x="{xa}" y="{}" width="{}" height="{}"/></clipPath><g clip-path="url(#plan-lines)">"#,
        top - 2.0,
        xb - xa,
        bot - top + 4.0
    );
    let projection = matches!(kind, Kind::Projection);
    let met_opacity = match kind {
        Kind::Payments { .. } => 0.2,
        Kind::Projection => 0.25,
        Kind::Target { .. } => 0.32,
    };
    let mut above = 0;
    for p in &f.sample {
        if p.values.iter().any(|v| *v > ymax) {
            above += 1;
        }
    }
    for p in f.sample.iter().filter(|p| p.met || projection) {
        let _ = write!(
            s,
            r#"<path d="{}" style="fill:none;stroke:var(--s1);stroke-width:1;opacity:{met_opacity}"/>"#,
            path(&p.values)
        );
    }
    for p in f.sample.iter().filter(|p| !p.met && !projection) {
        let _ = write!(
            s,
            r#"<path d="{}" style="fill:none;stroke:var(--bad);stroke-width:1.1;opacity:.55"/>"#,
            path(&p.values)
        );
    }
    let median: Vec<f64> = f.values.iter().map(|v| v.p50).collect();
    let _ = write!(
        s,
        r#"<path d="{}" style="fill:none;stroke:var(--ink);stroke-width:2.5;stroke-linejoin:round"/></g>"#,
        path(&median)
    );
    if let (Some(g), Kind::Target { by, .. }) = (goal, kind) {
        let gy = y(g);
        let _ = write!(
            s,
            r#"<line class="goalline" x1="{xa}" x2="{xb}" y1="{gy:.1}" y2="{gy:.1}"/><text class="annb" x="{}" y="{:.1}">{}</text>"#,
            xa + 6.0,
            gy - 8.0,
            esc(&format!("Goal: {} by {}", m.whole(g), by.short_name()))
        );
    }
    let end_value = median.last().copied().unwrap_or(0.0);
    let end_text = match kind {
        Kind::Payments { .. } => crate::planning::left_or_out(m, end_value, None),
        _ => m.about(end_value),
    };
    let ye = y(end_value);
    let _ = write!(
        s,
        r#"<text class="elab" x="{}" y="{:.1}">Average market</text><text class="eval" x="{}" y="{:.1}">{}</text>"#,
        xb + 12.0,
        ye - 3.0,
        xb + 12.0,
        ye + 13.0,
        esc(&end_text)
    );
    let xs: Vec<f64> = picks.iter().map(|&k| x(t_of(k))).collect();
    for (i, &k) in picks.iter().enumerate() {
        let a = if i == 0 {
            xa
        } else {
            (xs[i - 1] + xs[i]) / 2.0
        };
        let b = if i == xs.len() - 1 {
            xb
        } else {
            (xs[i] + xs[i + 1]) / 2.0
        };
        let v = &f.values[k];
        let tip = format!(
            "{}\nAverage market: {}\nStrong market: {}\nWeak market: {}",
            f.months[k].short_name(),
            m.about(v.p50),
            m.about(v.p90),
            m.about(v.p10)
        );
        let _ = write!(
            s,
            r#"<rect class="hit" x="{a:.1}" y="{top}" width="{:.1}" height="{}"><title>{}</title></rect>"#,
            (b - a).max(0.5),
            bot - top,
            esc(&tip)
        );
    }
    s.push_str("</svg>");
    let met = f.sample.iter().filter(|p| p.met).count();
    let total = f.sample.len();
    let (yes, no) = match kind {
        Kind::Payments { .. } => ("Pays in full to the end", "Runs out"),
        _ => ("Reaches the goal", "Falls short"),
    };
    let mut legend = String::from(r#"<div class="legend">"#);
    if projection {
        legend.push_str(r#"<span class="key"><i style="background:var(--s1);opacity:.6"></i>Simulated markets</span>"#);
    } else {
        let _ = write!(
            legend,
            r#"<span class="key"><i style="background:var(--s1);opacity:.6"></i>{yes}: {met} of these {total}</span><span class="key"><i style="background:var(--bad);opacity:.8"></i>{no}: {}</span>"#,
            total - met
        );
    }
    legend.push_str(
        r#"<span class="key"><i style="background:var(--ink)"></i>Average market</span>"#,
    );
    if goal.is_some() {
        legend.push_str(r#"<span class="key"><i style="background:transparent;border:1.5px dashed var(--ink)"></i>Your goal</span>"#);
    }
    legend.push_str("</div>");
    (legend + &s, above)
}

/// The mix as one bar of its assets' weights.
fn mixbar(r: &PlanReport, weights: &[(String, f64)], class: &str, labels: bool) -> String {
    let mut parts: Vec<&(String, f64)> = weights.iter().filter(|(_, w)| *w > 0.0).collect();
    parts.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut out = format!(r#"<div class="{class}">"#);
    for (asset, w) in parts {
        let k = r
            .context
            .assets
            .iter()
            .position(|a| a == asset)
            .unwrap_or(0);
        let c = colour(k);
        let text = format!("{asset} {}", pc(*w));
        let label = if labels { esc(&text) } else { String::new() };
        let _ = write!(
            out,
            r#"<span style="flex:{w};background:{c};color:{}" title="{}">{label}</span>"#,
            on(c),
            esc(&text)
        );
    }
    out.push_str("</div>");
    out
}

// ── Sections ─────────────────────────────────────────────────────────────────────────────────

fn plan_section(r: &PlanReport) -> String {
    let mut out = String::from(r#"<div class="card"><ol class="steps">"#);
    for (_, when, what, detail, amount) in steps(r) {
        let _ = write!(
            out,
            r#"<li><span class="when">{}</span><span class="what">{}<small>{}</small></span><span class="amt">{}</span></li>"#,
            esc(&when),
            esc(&what),
            esc(&detail),
            esc(&amount)
        );
    }
    let _ = write!(
        out,
        r#"</ol><p class="note">{}</p></div>"#,
        esc(&steps_note(r))
    );
    out
}

/// What the plan could absorb, as words: "less saving, a higher target or an earlier date".
fn room_words(r: &PlanReport) -> Option<String> {
    let c = r.figures.changes.as_ref().filter(|c| !c.to_reach)?;
    let payments = matches!(r.kind(), Kind::Payments { .. });
    let mut parts = Vec::new();
    if c.saving.is_some() {
        parts.push("less saving");
    }
    if c.start_value.is_some() && c.saving.is_none() {
        parts.push("less set aside today");
    }
    if c.amount.is_some() {
        parts.push(if payments {
            "larger payments"
        } else {
            "a higher target"
        });
    }
    if c.date.is_some() {
        parts.push(if payments {
            "earlier payments"
        } else {
            "an earlier date"
        });
    }
    match parts.len() {
        0 => None,
        1 => Some(parts[0].to_owned()),
        n => Some(format!("{} or {}", parts[..n - 1].join(", "), parts[n - 1])),
    }
}

fn summary_section(r: &PlanReport) -> String {
    let f = &r.figures;
    let m = r.money();
    let wanted = r.wanted();
    let end = r.end_long();
    let row = |name: &str, hint: &str, value: &str, class: &str| {
        format!(
            r#"<div class="orow"><div><div class="oname">{}</div><div class="ohint">{}</div></div><div class="oval {class}">{}</div></div>"#,
            esc(name),
            esc(hint),
            esc(value)
        )
    };
    let hints = [
        ("Strong market", "1 scenario in 10 does better"),
        ("Average market", "the middle scenario"),
        ("Weak market", "1 scenario in 10 does worse"),
    ];
    let e = f.end_value;
    let (outcomes, headline, sub) = match r.kind() {
        Kind::Payments {
            amount,
            every,
            start,
            end: last,
        } => {
            let p = f.payments.as_ref();
            let outs = [
                p.and_then(|p| p.runs_out_strong),
                p.and_then(|p| p.runs_out_average),
                p.and_then(|p| p.runs_out_weak),
            ];
            let values = [e.p90, e.p50, e.p10];
            let mut rows = String::new();
            for ((name, hint), (v, o)) in hints.iter().zip(values.iter().zip(outs)) {
                let class = if o.is_some() { "neg" } else { "" };
                rows.push_str(&row(
                    name,
                    hint,
                    &crate::planning::left_or_out(m, *v, o),
                    class,
                ));
            }
            let outcomes = format!(
                r#"<div class="outcomes"><h3>By {}</h3>{rows}{}</div>"#,
                esc(&last.long_name()),
                lifelines(start, last, outs)
            );
            let chance = f.outcome.chance.unwrap_or(0.0);
            let headline = format!(
                "You have {} of paying {} a {} in full from {} to {}.",
                a_chance(chance),
                m.whole(amount),
                every.noun(),
                start.long_name(),
                last.long_name()
            );
            let mut sub = format!(
                "In {} of every 100 market scenarios we simulated, every payment is made in full.",
                (100.0 * chance).round()
            );
            if let Some((when, share)) = p.and_then(|p| p.runs_short.zip(p.paid_when_short)) {
                let _ = write!(
                    sub,
                    " When the money runs short, it is typically in {}; even then, about {} of the money planned is paid.",
                    when.long_name(),
                    pc(share)
                );
            }
            (outcomes, headline, sub)
        }
        kind => {
            let goal = match kind {
                Kind::Target { amount, .. } => Some(amount),
                _ => None,
            };
            let mut rows = String::new();
            for ((name, hint), v) in hints.iter().zip([e.p90, e.p50, e.p10]) {
                rows.push_str(&row(name, hint, &m.about(v), ""));
            }
            let outcomes = format!(
                r#"<div class="outcomes"><h3>By {} you would have</h3>{rows}{}</div>"#,
                esc(&end),
                range_bar(m, e.p10, e.p50, e.p90, goal)
            );
            match goal {
                Some(amount) => {
                    let chance = f.outcome.chance.unwrap_or(0.0);
                    let headline = format!(
                        "You have {} of reaching {} by {end}.",
                        a_chance(chance),
                        m.whole(amount)
                    );
                    let mut sub = format!(
                        "In {} of every 100 market scenarios we simulated, you reach the goal.",
                        (100.0 * chance).round()
                    );
                    if let Some(s) = f.shortfall {
                        let _ = write!(
                            sub,
                            " If you fall short, it is usually by about {}.",
                            m.about(s)
                        );
                    }
                    (outcomes, headline, sub)
                }
                None => {
                    let mut sub = format!(
                        "In 1 scenario in 10 you would have more than {}, and in 1 in 10 less than {}.",
                        m.about(e.p90),
                        m.about(e.p10)
                    );
                    if let Some(put) = r.put_in().filter(|p| *p > 0.0) {
                        let _ = write!(sub, " What you put in comes to {}.", m.whole(put));
                    }
                    (
                        outcomes,
                        format!(
                            "In an average market, you would have about {} by {end}.",
                            m.about(e.p50)
                        ),
                        sub,
                    )
                }
            }
        }
    };
    let verdict = match (f.status, f.outcome.chance) {
        (Some(status), Some(chance)) => {
            let (class, sign) = status_style(status);
            let mut lead = format!("You want at least {}, the mark on the ring.", pc(wanted));
            if status == Status::RoomToSpare
                && let Some(words) = room_words(r)
            {
                let _ = write!(
                    lead,
                    " From {}, the plan has room to spare: it would still give {} with {words}.",
                    pc(f.spare_from),
                    a_chance(wanted)
                );
            }
            format!(
                r#"{}<div class="vt"><span class="pill {class}">{sign} {}</span><p class="headline">{}</p><p class="sub">{} {}</p></div>"#,
                ring(chance, wanted, f.spare_from, class),
                status.label(),
                esc(&headline),
                esc(&lead),
                esc(&sub)
            )
        }
        _ => format!(
            r#"<div class="vt"><p class="headline">{}</p><p class="sub">{}</p></div>"#,
            esc(&headline),
            esc(&sub)
        ),
    };
    format!(r#"<div class="card hero"><div class="verdict">{verdict}</div>{outcomes}</div>"#)
}

fn time_section(r: &PlanReport) -> String {
    let f = &r.figures;
    let m = r.money();
    let (chart, above) = scenarios_chart(r);
    let met = f.sample.iter().filter(|p| p.met).count();
    let total = f.sample.len();
    let all = crate::text::money(f.simulations as f64);
    let chance = f.outcome.chance.unwrap_or(0.0);
    let mut note = match r.kind() {
        Kind::Target { amount, .. } => {
            let mut n = format!(
                "Each line is one simulated market: {met} of these {total} reach the goal, as {} of all {all} do. They were picked evenly from the best outcome to the worst.",
                pc(chance)
            );
            match r.put_in() {
                Some(put) if put >= amount => {
                    let _ = write!(
                        n,
                        " What you put in, {}, already covers the goal on its own.",
                        m.whole(put)
                    );
                }
                Some(put) if put > 0.0 && f.end_value.p50 > put => {
                    let _ = write!(
                        n,
                        " In an average market, about {} of the {} comes from growth; the rest is what you put in.",
                        m.about(f.end_value.p50 - put),
                        m.about(f.end_value.p50)
                    );
                }
                _ => {}
            }
            n
        }
        Kind::Payments { .. } => {
            let mut n = format!(
                "Each line is one simulated market: {met} of these {total} make every payment in full, as {} of all {all} do; the red ones run out and drop to zero. They were picked evenly from the best outcome to the worst.",
                pc(chance)
            );
            if let Some(month) = r.runs_out_without_growth() {
                let _ = write!(
                    n,
                    " With no growth at all, the money would run out in {}.",
                    month.long_name()
                );
            }
            n
        }
        Kind::Projection => format!(
            "Each line is one simulated market, picked evenly from the best outcome to the worst, out of all {all}."
        ),
    };
    match above {
        0 => {}
        1 => note.push_str(" One line goes higher and runs off the top of the chart."),
        n => {
            let _ = write!(
                note,
                " {n} lines go higher and run off the top of the chart."
            );
        }
    }
    format!(
        r#"<div class="card">{chart}<p class="note">{}</p></div>"#,
        esc(&note)
    )
}

fn track_section(r: &PlanReport) -> String {
    let wanted = r.wanted();
    let up = r.figures.changes.as_ref().is_some_and(|c| c.to_reach);
    let gives = if up { "Gives" } else { "Keeps" };
    let mut out = String::from(r#"<div class="levers">"#);
    let all = levers(r);
    for (label, change, detail) in &all {
        let class = if change.chars().count() > 12 {
            "lv long"
        } else {
            "lv"
        };
        let _ = write!(
            out,
            r#"<div class="lever"><div class="lk">{}</div><div class="{class}">{}</div><div class="ld">{}</div><div class="lc">{gives} {}</div></div>"#,
            esc(label),
            esc(change),
            esc(detail),
            esc(&a_chance(wanted))
        );
    }
    out.push_str("</div>");
    if all.is_empty() {
        out.clear();
    }
    let _ = write!(out, r#"<p class="note">{}</p>"#, esc(&levers_note(r)));
    out
}

fn stress_section(r: &PlanReport) -> String {
    let wanted = r.wanted();
    let projection = matches!(r.kind(), Kind::Projection);
    let head_money = format!("In an average market, by {}", r.last_month().short_name());
    let rows = stress_rows(r);
    let plan = rows[0].2;
    let plan_money = outcome_money(r, &plan);
    let mut out = format!(
        r#"<div class="card wi{}"><div class="wi-row head"><span></span>"#,
        if projection { " nochance" } else { "" }
    );
    if !projection {
        out.push_str(r#"<span></span><span style="text-align:right">Chance</span>"#);
    }
    let _ = write!(
        out,
        r#"<span style="text-align:right">{}</span></div>"#,
        esc(&head_money)
    );
    for (i, (title, description, o)) in rows.iter().enumerate() {
        let money = outcome_money(r, o);
        // Against the plan as it stands, when the figure shown differs.
        let (chance_note, money_note) = if i == 0 {
            (String::new(), String::new())
        } else {
            let (now, was) = (pc(o.chance.unwrap_or(0.0)), pc(plan.chance.unwrap_or(0.0)));
            let chance_note = match o.chance.unwrap_or(0.0) < plan.chance.unwrap_or(0.0) {
                _ if now == was => String::new(),
                true => format!("down from {was}"),
                false => format!("up from {was}"),
            };
            let money_note = if money == plan_money {
                String::new()
            } else {
                format!("not {plan_money}")
            };
            (chance_note, money_note)
        };
        let _ = write!(
            out,
            r#"<div class="wi-row"><div class="wi-l"><b>{}</b><span>{}</span></div>"#,
            esc(title),
            esc(description)
        );
        if let Some(c) = o.chance {
            let _ = write!(
                out,
                r#"<div class="wi-bar" role="img" aria-label="{} chance"><i style="width:{:.1}%"></i><u style="left:calc({:.1}% - 1px)"></u></div><div class="wi-c">{}<small>{}</small></div>"#,
                pc(c),
                100.0 * c,
                100.0 * wanted,
                pc(c),
                esc(&chance_note)
            );
        }
        let _ = write!(
            out,
            r#"<div class="wi-m">{}<small>{}</small></div></div>"#,
            esc(&money),
            esc(&money_note)
        );
    }
    out.push_str("</div>");
    if let Some(meaning) = stress_meaning(r) {
        let _ = write!(
            out,
            r#"<div class="means"><b>What this means</b>{}</div>"#,
            esc(&meaning)
        );
    }
    out
}

fn mix_section(r: &PlanReport) -> String {
    let f = &r.figures;
    let m = r.money();
    let c = &r.context;
    let title = match c.mix.name() {
        Some(name) => format!("Your mix: {name}"),
        None => "Your mix".to_owned(),
    };
    let mut out = format!(
        r#"<div class="card"><h3>{}</h3>{}"#,
        esc(&title),
        mixbar(r, &f.weights, "mixbar", true)
    );
    match &c.mix {
        Mix::GlidePath { points, .. } => {
            let assets: Vec<&String> = c
                .assets
                .iter()
                .filter(|a| points.values().any(|w| w.get(*a).is_some_and(|v| *v > 0.0)))
                .collect();
            let _ = write!(
                out,
                r#"<p class="note" style="margin-top:6px">The weights move along a glide path, a straight line from each point to the next, and are rebalanced every month.</p><div class="tw"><table><thead><tr><th>From</th>"#
            );
            for a in &assets {
                let _ = write!(out, r#"<th class="r">{}</th>"#, esc(a));
            }
            out.push_str("</tr></thead><tbody>");
            for (month, w) in points {
                let _ = write!(out, "<tr><td>{}</td>", esc(&month.short_name()));
                for a in &assets {
                    let _ = write!(
                        out,
                        r#"<td class="r">{}</td>"#,
                        pc(w.get(*a).copied().unwrap_or(0.0))
                    );
                }
                out.push_str("</tr>");
            }
            out.push_str("</tbody></table></div></div>");
        }
        _ => out.push_str(
            r#"<p class="note" style="margin-top:6px">Rebalanced back to these weights every month.</p></div>"#,
        ),
    }
    if f.alternatives.is_empty() {
        out.push_str(&tiles(&[
            (
                "Typical return a year".to_owned(),
                pc1(f.risk.typical_return),
                Some("after inflation and fees".to_owned()),
            ),
            (
                "Worst year in 20".to_owned(),
                f.risk.worst_year.map_or_else(|| "–".to_owned(), pc),
                Some("1 twelve-month stretch in 20 loses more".to_owned()),
            ),
            (
                "Largest fall".to_owned(),
                pc(-f.risk.largest_fall.abs()),
                Some("from a peak, typically".to_owned()),
            ),
            (
                "Volatility".to_owned(),
                pc1(f.risk.volatility),
                Some("a year, in the history".to_owned()),
            ),
        ]));
        return out;
    }
    let payments = matches!(r.kind(), Kind::Payments { .. });
    let projection = matches!(r.kind(), Kind::Projection);
    let cell = |x: &MixRow<'_>, weak: bool| -> String {
        if payments {
            let (v, o) = if weak {
                (x.end.p10, x.runs_out_weak)
            } else {
                (x.end.p50, x.outcome.runs_out)
            };
            crate::planning::left_or_out(m, v, o)
        } else if weak {
            m.about(x.end.p10)
        } else {
            m.about(x.end.p50)
        }
    };
    out.push_str(r#"<div class="card"><h3>How other mixes compare</h3><div class="tw"><table class="cmp"><thead><tr><th>Mix</th><th>Holds</th>"#);
    if !projection {
        out.push_str(r#"<th class="r">Chance</th>"#);
    }
    out.push_str(r#"<th class="r">Weak market</th><th class="r">Average market</th><th class="r">Worst year in 20</th></tr></thead><tbody>"#);
    let mixes = r.mixes();
    for (x, yours) in &mixes {
        let _ = write!(
            out,
            r#"<tr class="{}"><td>{}{}</td><td>{}</td>"#,
            if *yours { "yours" } else { "" },
            esc(x.name),
            if *yours && c.mix.name().is_some() {
                r#"<span class="tag">Your mix</span>"#
            } else {
                ""
            },
            mixbar(r, x.weights, "mini", false)
        );
        if !projection {
            let _ = write!(
                out,
                r#"<td class="r"><b>{}</b></td>"#,
                pc(x.outcome.chance.unwrap_or(0.0))
            );
        }
        let _ = write!(
            out,
            r#"<td class="r">{}</td><td class="r">{}</td><td class="r">{}</td></tr>"#,
            esc(&cell(x, true)),
            esc(&cell(x, false)),
            x.risk.worst_year.map_or_else(|| "–".to_owned(), pc)
        );
    }
    let when = if payments {
        format!(
            "what is left by {}, or when the money runs out",
            r.end_long()
        )
    } else {
        format!("the value by {}", r.end_long())
    };
    let mut note = format!(
        "Weak and average markets are {when}. Worst year in 20: only 1 twelve-month stretch in 20 loses more."
    );
    let own = r.own_mix();
    if let Some((riskiest, false)) = mixes.last()
        && riskiest.end.p50 > own.end.p50
        && riskiest.end.p10 < own.end.p10
    {
        let _ = write!(
            note,
            " A riskier mix widens the range: {} does better in an average market and worse in a weak one.",
            riskiest.name
        );
    }
    let _ = write!(
        out,
        r#"</tbody></table></div><p class="note">{}</p></div>"#,
        esc(&note)
    );
    out
}

fn how_section(r: &PlanReport) -> String {
    let mut out = String::from(r#"<div class="card"><dl class="how">"#);
    for (term, text) in method_notes(r, true) {
        let _ = write!(out, "<dt>{}</dt><dd>{}</dd>", esc(&term), esc(&text));
    }
    let _ = write!(
        out,
        r#"</dl><div class="important"><p><b>Important.</b> {}</p><p style="margin-top:8px"><b>Not advice.</b> {}</p></div></div>"#,
        esc(IMPORTANT),
        esc(NOT_ADVICE)
    );
    out
}

impl PlanReport {
    /// The report as one self-contained HTML page, titled with `name` (or the report's own, or
    /// the goal's): the plan, where it stands, the value over time, the changes that would reach
    /// or keep the chance wanted, the plan when markets do worse, the investment mix, and how
    /// the figures were worked out.
    pub fn to_html(&self, name: Option<&str>) -> String {
        let title = self.title(name);
        let f = &self.figures;
        let m = self.money();
        let c = &self.context;
        let goal = match self.kind() {
            Kind::Target { amount, by } => format!("{} by {}", m.whole(amount), by.long_name()),
            Kind::Payments {
                amount,
                every,
                start,
                end,
            } => format!(
                "{} a {}, {} to {}",
                m.whole(amount),
                every.noun(),
                start.short_name(),
                end.short_name()
            ),
            Kind::Projection => format!("Projection to {}", self.end_long()),
        };
        let scenarios = match c.model {
            finalytics_plan::Model::Historical => "historical market scenarios",
            _ => "market scenarios",
        };
        let mix = c
            .mix
            .name()
            .map_or_else(String::new, |n| format!("{} · ", esc(n)));
        let meta = format!(
            "{}<br>{mix}{} {scenarios} · today's money",
            esc(&goal),
            crate::text::money(f.simulations as f64)
        );
        let projection = matches!(self.kind(), Kind::Projection);
        let heading = levers_heading(self);
        let mut nav: Vec<(&str, &str)> = vec![
            ("plan", "Your plan"),
            ("summary", "Where you stand"),
            ("time", "Over time"),
        ];
        if !projection {
            nav.push(("track", heading));
        }
        nav.extend([
            ("stress", "What if"),
            ("mix", "Your mix"),
            ("how", "How we worked it out"),
        ]);
        let wanted = self.wanted();
        let mut body = String::new();
        body.push_str(&section(
            "plan",
            "Your plan",
            "What we tested",
            &plan_section(self),
        ));
        body.push_str(&section(
            "summary",
            "Where you stand",
            "",
            &summary_section(self),
        ));
        body.push_str(&section(
            "time",
            "Your money over time",
            &format!(
                "{} of the {} simulated markets",
                f.sample.len(),
                crate::text::money(f.simulations as f64)
            ),
            &time_section(self),
        ));
        if !projection {
            let sub = match &f.changes {
                Some(ch) if ch.to_reach => {
                    format!("Changes that would raise your chance to {}", pc(wanted))
                }
                _ => format!(
                    "Changes the plan could absorb and still keep {}",
                    a_chance(wanted)
                ),
            };
            body.push_str(&section("track", heading, &sub, &track_section(self)));
        }
        let tests = if f.what_if.crash.is_some() {
            "two tougher situations"
        } else {
            "a tougher situation"
        };
        let stress_sub = if projection {
            format!("We re-ran your plan in {tests}.")
        } else {
            format!(
                "We re-ran your plan in {tests}. Each bar is your chance of meeting the goal; the mark is the {} you want.",
                pc(wanted)
            )
        };
        body.push_str(&section(
            "stress",
            "What if markets do worse?",
            &stress_sub,
            &stress_section(self),
        ));
        body.push_str(&section(
            "mix",
            "Your investment mix",
            "",
            &mix_section(self),
        ));
        body.push_str(&section(
            "how",
            "How we worked it out",
            "",
            &how_section(self),
        ));
        let source = format!(
            "{} simulations of the assets' monthly returns from {} to {}, seed {}",
            crate::text::money(f.simulations as f64),
            c.history_start.short_name(),
            c.history_end.short_name(),
            c.seed
        );
        frame(
            &Frame {
                title: &title,
                brand: Some("Planning report"),
                meta: &meta,
                nav: &nav,
                version: &c.version,
                digest: &c.digest,
                source: &source,
                style: STYLE,
            },
            &body,
        )
    }
}
