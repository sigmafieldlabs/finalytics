"""Generate reference plans for the finalytics planning tests.

Each plan is simulated afresh here, from the plan's definition rather than the engine's code, on
the asset-class dataset's monthly returns with the historical model, which replays every stretch
of the history as long as the plan, in order, and so draws nothing at random. Each month, in
today's money: the value at the start is recorded; savings are paid in; spending and any goal
payment are taken out as far as the money goes; and the value grows by the mix's return after
the fee and inflation, (1 + r - fee / 12) / (1 + inflation a month), at least zero. The weights
are rebalanced every month, and a glide path moves in a straight line between its points.

The figures: the chance of the goal; percentiles of the value, interpolated linearly; the
payments made and when the money runs short; the changes that reach or keep the chance wanted,
found by trying every amount on a grid in turn and rounded to three significant figures; the plan with
returns a point a year lower and with the worst twelve months of the history placed in it; and
each mix's risk.

Usage:

    pip install -r scripts/reference/requirements.txt
    python scripts/reference/planning.py

The output, ``crates/finalytics/tests/fixtures/planning.json``, is committed. Regenerate it only
when a case is added or a reference library changes, and review the difference.
"""

import json
import math
import pathlib

import numpy as np
import pandas as pd

ROOT = pathlib.Path(__file__).resolve().parents[2]
OUTPUT = ROOT / "crates/finalytics/tests/fixtures/planning.json"

prices = pd.read_csv(ROOT / "crates/finalytics/data/asset-classes.csv", index_col="date")
RETURNS = (prices / prices.shift(1) - 1).iloc[1:]
ASSETS = list(RETURNS.columns)
R = RETURNS.to_numpy()  # [month, asset]
N = len(R)
# The plans start in the month after the last return: November 2026.
START = (2026, 11)


def month(offset: int) -> str:
    y, m = divmod(START[0] * 12 + START[1] - 1 + offset, 12)
    return f"{y:04d}-{m + 1:02d}"


def offset(text: str) -> int:
    y, m = int(text[:4]), int(text[5:7])
    return y * 12 + m - 1 - (START[0] * 12 + START[1] - 1)


def history_month(h: int) -> str:
    """The month of the history's return `h`: the first is November 1996."""
    y, m = divmod(1996 * 12 + 10 + h, 12)
    return f"{y:04d}-{m + 1:02d}"


def percentile(xs, q):
    return float(np.percentile(np.asarray(xs, dtype=float), q, method="linear"))


def step_of(x):
    if x <= 0 or not math.isfinite(x):
        return 1.0
    return max(10.0 ** (math.floor(math.log10(x)) - 2), 0.01)


class Plan:
    """A plan of the options given, in months from its start."""

    def __init__(self, options):
        self.options = options
        goal = options["goal"]
        self.kind = goal["type"]
        self.amount = goal["amount"]
        if self.kind == "target":
            self.months = offset(goal["by"])
            self.pay_start = 0
        else:
            self.pay_start = offset(goal["start"])
            self.pay_end = offset(goal["end"])
            self.months = self.pay_end + 1
        self.sims = N - self.months + 1
        self.wanted = options["chance"]
        self.fee = options["fee"] / 12
        self.inflation = (1 + options["inflation"]["rate"]) ** (1 / 12) - 1
        self.start_value = options["start_value"]
        self.flows = options["flows"]

    def weights(self, mix, months):
        """The weights in each month, in the order of the assets."""
        vector = lambda w: np.array([w.get(a, 0.0) for a in ASSETS])
        if mix["type"] == "fixed":
            return np.tile(vector(mix["weights"]), (months, 1))
        points = sorted((offset(m), vector(w)) for m, w in mix["points"].items())
        out = []
        for t in range(months):
            later = [j for j, (k, _) in enumerate(points) if k > t]
            if not later:
                out.append(points[-1][1])
            elif later[0] == 0:
                out.append(points[0][1])
            else:
                (k0, w0), (k1, w1) = points[later[0] - 1], points[later[0]]
                out.append(w0 + (w1 - w0) * (t - k0) / (k1 - k0))
        return np.array(out)

    def growth(self, weights):
        """[simulation, month]: simulation s replays the history's months s, s + 1, ..."""
        g = np.empty((self.sims, self.months))
        for s in range(self.sims):
            r = (R[s:s + self.months] * weights).sum(axis=1)
            g[s] = np.maximum((1 + r - self.fee) / (1 + self.inflation), 0.0)
        return g

    def run(self, g, *, saving=None, start_value=None, amount=None, pay_start=None, shift=0.0, crash=None):
        """Every simulation: the value at the start of each month and at the end, the month of
        the first payment not made in full, the share of the payments made, and whether each
        spending flow was paid in full."""
        v = np.full(self.sims, self.start_value if start_value is None else start_value, dtype=float)
        amount = self.amount if amount is None else amount
        pay_start = self.pay_start if pay_start is None else pay_start
        until = (self.months if self.kind == "target" else pay_start) - 1
        values = np.empty((self.sims, self.months + 1))
        first_short = np.full(self.sims, -1)
        paid = np.zeros(self.sims)
        spent_in_full = [np.ones(self.sims, dtype=bool) for _ in self.flows]
        for t in range(self.months):
            values[:, t] = v
            # Savings are paid in first, then spending is taken out.
            order = [i for i, f in enumerate(self.flows) if f["type"] == "saving"]
            order += [i for i, f in enumerate(self.flows) if f["type"] != "saving"]
            for i in order:
                f = self.flows[i]
                if "on" in f:
                    due = t == offset(f["on"])
                else:
                    first = offset(f["start"]) if "start" in f else 0
                    last = offset(f["end"]) if "end" in f else until
                    every = {"month": 1, "quarter": 3, "year": 12}[f["every"]]
                    due = first <= t <= last and (t - first) % every == 0
                if not due:
                    continue
                a = f["amount"] if (saving is None or i != saving[0]) else saving[1]
                if f["type"] == "saving":
                    v = v + a
                else:
                    p = np.minimum(a, v)
                    spent_in_full[i] &= ~(p < a * (1 - 1e-12))
                    v = v - p
            if self.kind == "payments" and pay_start <= t <= self.pay_end:
                p = np.minimum(amount, v)
                paid += p
                short = (p < amount * (1 - 1e-12)) & (first_short < 0)
                first_short[short] = t
                v = v - p
            gt = g[:, t]
            if crash is not None and crash[0] <= t < crash[0] + len(crash[1]):
                gt = np.full(self.sims, crash[1][t - crash[0]])
            v = v * np.maximum(gt + shift, 0.0)
        values[:, self.months] = v
        met = v >= amount if self.kind == "target" else first_short < 0
        return values, first_short, paid, met, spent_in_full

    def chance(self, g, **changes):
        return float(self.run(g, **changes)[3].mean())

    def payments(self, first_short, paid, met):
        count = self.pay_end - self.pay_start + 1
        planned = self.amount * count
        short = first_short >= 0
        still = lambda m: float(((first_short < 0) | (first_short > m)).mean())
        months = range(self.pay_start, self.pay_end + 1)
        # The first payment month by which at least 1 - level of the simulations have missed one.
        first_below = lambda level: next((month(m) for m in months if still(m) <= level + 1e-12), None)
        return {
            "paid": float((paid / planned).mean()),
            "paid_when_short": float((paid[short] / planned).mean()) if short.any() else None,
            "runs_short": month(int(np.sort(first_short[short])[(short.sum() - 1) // 2])) if short.any() else None,
            "runs_out_weak": first_below(0.9),
            "runs_out_average": first_below(0.5),
            "runs_out_strong": first_below(0.1),
            "lasts_until": next((month(m) for m in reversed(months) if still(m) >= self.wanted), None),
        }

    def changes(self, g, chance_now, ends, chance_by_month):
        """Each change found by trying every amount on the grid of three significant figures of
        the current amount in turn, then rounded to three significant figures of its own: up for
        a saving or a start value, down for the goal's amount."""
        wanted = self.wanted
        up = chance_now < wanted
        meets = lambda **c: self.chance(g, **c) >= wanted
        round_up = lambda x: math.ceil(x / step_of(x) - 1e-9) * step_of(x)
        round_down = lambda x: math.floor(x / step_of(x) + 1e-9) * step_of(x)
        changed = lambda to, now: None if to is None or abs(to - now) <= 1e-9 * max(now, 1) else to
        out = {"to_reach": up}
        # The saving: the plan's only regular saving here.
        i = next(k for k, f in enumerate(self.flows) if f["type"] == "saving" and "every" in f)
        current = self.flows[i]["amount"]
        step = step_of(current)
        k_now = round(current / step)
        if up:
            ks = range(k_now, math.ceil(max(current * 2, 1) * 2**10 / step) + 1)
        else:
            ks = range(0, k_now + 1)
        k = next((k for k in ks if meets(saving=(i, k * step))), None)
        out["saving"] = changed(None if k is None else round_up(k * step), current)
        # The start value.
        current = self.start_value
        scale = self.amount if self.kind == "target" else self.amount * (self.pay_end - self.pay_start + 1)
        step = step_of(current if current > 0 else scale)
        k_now = round(current / step)
        if up:
            ks = range(k_now, math.ceil(max(current, scale / 4, 1) * 2**10 / step) + 1)
        else:
            ks = range(0, k_now + 1)
        k = next((k for k in ks if meets(start_value=k * step)), None)
        out["start_value"] = changed(None if k is None else round_up(k * step), current)
        # The goal's amount: for a target, the largest on the grid at or below the quantile of the
        # values at the date that still gives the chance wanted; for payments, the largest that
        # gives it.
        step = step_of(self.amount)
        if self.kind == "target":
            top = math.floor(percentile(ends, (1 - wanted) * 100) / step)
            k = next((k for k in range(top, 0, -1) if meets(amount=k * step)), None)
        else:
            k_now = round(self.amount / step)
            ks = range(k_now, -1, -1) if up else range(math.ceil(self.amount * 2**11 / step), k_now - 1, -1)
            k = next((k for k in ks if meets(amount=k * step)), None)
            k = None if k is not None and k <= 0 else k
        out["amount"] = changed(None if k is None else round_down(k * step), self.amount)
        # The date: for a target, only an earlier one (a later one needs months past the
        # history); for payments, the first month of the first payment that reaches, or keeps,
        # the chance wanted.
        if self.kind == "target":
            found = None if up else next((m for m in range(1, self.months) if chance_by_month[m] >= wanted), None)
        else:
            ks = range(self.pay_start, self.pay_end + 1) if up else range(0, self.pay_start + 1)
            found = next((k for k in ks if meets(pay_start=k)), None)
            found = None if found == self.pay_start else found
        out["date"] = None if found is None else month(found)
        return out

    def risk(self, g, weights):
        levels = np.concatenate([np.ones((self.sims, 1)), np.cumprod(g, axis=1)], axis=1)
        peaks = np.maximum.accumulate(levels, axis=1)
        falls = (levels / peaks - 1).min(axis=1)
        years = [levels[:, t + 12] / levels[:, t] - 1 for t in range(0, self.months - 11, 12)]
        typical = np.where(levels[:, -1] > 0, levels[:, -1] ** (12 / self.months) - 1, -1.0)
        r = R @ weights[0]
        return {
            "typical_return": percentile(typical, 50),
            "worst_year": percentile(np.concatenate(years), 5),
            "largest_fall": percentile(falls, 50),
            "volatility": float(np.sqrt(r.var(ddof=1) * 12)),
        }

    def crash(self, weights):
        """The worst twelve months of the history for the weights where they are placed, and
        the growth of those months placed there."""
        at = self.pay_start if self.kind == "payments" else max(self.months - 12, 0)
        span = min(12, self.months - at)
        w = weights[min(at, len(weights) - 1)]
        r = R @ w
        totals = [np.prod(1 + r[h:h + span]) - 1 for h in range(N - span + 1)]
        start = int(np.argmin(totals))
        factors = []
        for k in range(span):
            rk = R[start + k] @ weights[min(at + k, len(weights) - 1)]
            factors.append(max((1 + rk - self.fee) / (1 + self.inflation), 0.0))
        return at, start, float(totals[start]), factors


def outcome(plan, run):
    values, first_short, paid, met, _ = run
    ends = values[:, -1]
    out = {"chance": float(met.mean()), "typical": percentile(ends, 50), "runs_out": None}
    if plan.kind == "payments":
        out["runs_out"] = plan.payments(first_short, paid, met)["runs_out_average"]
    return out


def case(name, options):
    plan = Plan(options)
    weights = plan.weights(options["weights"], plan.months)
    g = plan.growth(weights)
    run = plan.run(g)
    values, first_short, paid, met, spent = run
    ends = values[:, -1]
    chance = float(met.mean())
    chance_by_month = (values >= plan.amount).mean(axis=0) if plan.kind == "target" else None
    expected = {
        "simulations": plan.sims,
        "chance": chance,
        "end": {f"p{q}": percentile(ends, q) for q in (10, 25, 50, 75, 90)},
        "median_by_month": [percentile(values[:, t], 50) for t in range(plan.months + 1)],
        "paid_in_full": [float(s.mean()) if f["type"] == "spending" else None for f, s in zip(plan.flows, spent)],
        "changes": plan.changes(g, chance, ends, chance_by_month),
        "lower_returns": outcome(plan, plan.run(g, shift=-0.01 / 12)),
        "risk": plan.risk(g, weights),
    }
    if plan.kind == "target":
        short = ends[~met]
        expected["shortfall"] = percentile(plan.amount - short, 50) if len(short) else None
        expected["chance_by_month"] = [float(c) for c in chance_by_month]
    else:
        expected["payments"] = plan.payments(first_short, paid, met)
    at, start, loss, factors = plan.crash(weights)
    expected["crash"] = outcome(plan, plan.run(g, crash=(at, factors)))
    expected["crash_window"] = {
        "from": history_month(start),
        "to": history_month(start + len(factors) - 1),
        "loss": loss,
        "placed": month(at),
    }
    return {"name": name, "options": options, "expected": expected}


cases = [
    case(
        "a house deposit with a car bought along the way",
        {
            "goal": {"type": "target", "amount": 150000.0, "by": "2030-06"},
            "chance": 0.8,
            "start_value": 60000.0,
            # Listed before the saving, which is still paid in first.
            "flows": [
                {"type": "spending", "name": "Car", "amount": 25000.0, "on": "2028-03"},
                {"type": "saving", "amount": 2500.0, "every": "month"},
            ],
            "weights": {"type": "fixed", "weights": {"US Equity": 0.6, "Bonds": 0.4}},
            "fee": 0.0025,
            "inflation": {"type": "rate", "rate": 0.025},
            "model": {"type": "historical"},
        },
    ),
    case(
        "a college fund with room to spare",
        {
            "goal": {"type": "target", "amount": 60000.0, "by": "2034-09"},
            "chance": 0.8,
            "start_value": 25000.0,
            "flows": [{"type": "saving", "amount": 400.0, "every": "month"}],
            "weights": {"type": "fixed", "weights": {"US Equity": 0.3, "Bonds": 0.5, "T-Bills": 0.2}},
            "fee": 0.0,
            "inflation": {"type": "rate", "rate": 0.03},
            "model": {"type": "historical"},
        },
    ),
    case(
        "an income for ten years after five of saving, along a glide path",
        {
            "goal": {"type": "payments", "amount": 2000.0, "start": "2031-11", "end": "2041-10"},
            "chance": 0.85,
            "start_value": 100000.0,
            "flows": [{"type": "saving", "amount": 1000.0, "every": "month"}],
            "weights": {
                "type": "glide_path",
                "points": {
                    "2026-11": {"US Equity": 0.8, "Bonds": 0.2},
                    "2031-11": {"US Equity": 0.4, "Bonds": 0.6},
                },
            },
            "fee": 0.005,
            "inflation": {"type": "rate", "rate": 0.03},
            "model": {"type": "historical"},
        },
    ),
    case(
        "an income with room to spare",
        {
            "goal": {"type": "payments", "amount": 1000.0, "start": "2031-11", "end": "2041-10"},
            "chance": 0.8,
            "start_value": 120000.0,
            "flows": [{"type": "saving", "amount": 500.0, "every": "month"}],
            "weights": {"type": "fixed", "weights": {"US Equity": 0.5, "Bonds": 0.5}},
            "fee": 0.0025,
            "inflation": {"type": "rate", "rate": 0.025},
            "model": {"type": "historical"},
        },
    ),
]
OUTPUT.write_text(json.dumps({"cases": cases}, indent=1) + "\n")
print(f"wrote {OUTPUT.relative_to(ROOT)}: {len(cases)} cases")
for c in cases:
    e = c["expected"]
    print(f"  {c['name']}: {e['simulations']} simulations, chance {e['chance']:.4f}, changes {e['changes']}")
