"""Generate reference values for the covariance and mean estimators of finalytics-risk.

Each estimator is written here from its published source, in NumPy, apart from the Rust:

- shrinkage towards constant correlation: the authors' MATLAB ``covCor`` (Ledoit and Wolf,
  "Honey, I shrunk the sample covariance matrix", 2004);
- the statistical factor model: principal components of the sample covariance from
  ``numpy.linalg.eigh``, with each asset's variance kept;
- denoising: the sample correlation's eigenvalues below the Marchenko-Pastur edge
  ``(1 + sqrt(n / t))^2`` replaced by their average (Laloux, Cizeau, Bouchaud and Potters,
  2000; Lopez de Prado, 2020), from ``numpy.linalg.eigh``;
- analytical nonlinear shrinkage: the authors' MATLAB ``analytical_shrinkage`` (Ledoit and
  Wolf, Annals of Statistics, 2020), for more returns than assets and for fewer;
- the James-Stein mean: shrinkage of the sample means towards their grand mean with the
  Efron-Morris intensity ``(n - 3) / (t (m - g)' S^-1 (m - g))``, from ``numpy.linalg.solve``;
- the exponentially weighted mean: pandas ``ewm(alpha=1 - decay, adjust=True)``.

Usage:

    pip install -r scripts/reference/requirements.txt
    python scripts/reference/estimators.py

The output, ``estimators.json`` in ``crates/finalytics-risk/tests/fixtures``, is committed.
"""

import json
import pathlib

import numpy as np
import pandas as pd

ROOT = pathlib.Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "crates/finalytics-risk/tests/fixtures"


def cov_cor(x):
    """covCor.m: shrinkage towards constant correlation."""
    t, n = x.shape
    x = x - x.mean(axis=0)
    sample = x.T @ x / t
    var = np.diag(sample).reshape(-1, 1)
    sqrtvar = np.sqrt(var)
    r_bar = (np.sum(sample / (sqrtvar @ sqrtvar.T)) - n) / (n * (n - 1))
    prior = r_bar * (sqrtvar @ sqrtvar.T)
    np.fill_diagonal(prior, var.ravel())
    y = x**2
    phi_mat = y.T @ y / t - 2 * (x.T @ x) * sample / t + sample**2
    phi = phi_mat.sum()
    term1 = (x**3).T @ x / t
    help_ = x.T @ x / t
    help_diag = np.diag(help_).reshape(-1, 1)
    term2 = np.tile(help_diag, (1, n)) * sample
    term3 = help_ * np.tile(var, (1, n))
    term4 = np.tile(var, (1, n)) * sample
    theta = term1 - term2 - term3 + term4
    np.fill_diagonal(theta, 0.0)
    rho = np.sum(np.diag(phi_mat)) + r_bar * np.sum((1.0 / sqrtvar) @ sqrtvar.T * theta)
    gamma = np.linalg.norm(sample - prior, "fro") ** 2
    kappa = (phi - rho) / gamma
    shrinkage = max(0.0, min(1.0, kappa / t))
    return shrinkage * prior + (1 - shrinkage) * sample, shrinkage


def statistical(x, factors):
    sample = np.cov(x, rowvar=False, ddof=1)
    values, vectors = np.linalg.eigh(sample)
    order = np.argsort(values)[::-1][:factors]
    loadings = vectors[:, order] * np.sqrt(np.maximum(values[order], 0.0))
    common = loadings @ loadings.T
    specific = np.maximum(np.diag(sample) - np.diag(common), 0.0)
    return common + np.diag(specific)


def denoised(x):
    t, n = x.shape
    sample = np.cov(x, rowvar=False, ddof=1)
    sd = np.sqrt(np.diag(sample))
    correlation = sample / np.outer(sd, sd)
    values, vectors = np.linalg.eigh(correlation)
    edge = (1.0 + np.sqrt(n / t)) ** 2
    noise = values < edge
    if noise.any():
        values = values.copy()
        values[noise] = values[noise].mean()
    rebuilt = vectors @ np.diag(values) @ vectors.T
    scale = np.sqrt(np.diag(rebuilt))
    rebuilt = rebuilt / np.outer(scale, scale)
    return rebuilt * np.outer(sd, sd)


def analytical_shrinkage(x):
    """analytical_shrinkage.m, with the mean removed (k = 1)."""
    n, p = x.shape
    x = x - x.mean(axis=0)
    n = n - 1
    sample = x.T @ x / n
    lam, u = np.linalg.eigh(sample)
    order = np.argsort(lam)
    lam, u = lam[order], u[:, order]
    lam = lam[max(0, p - n):]
    m = min(p, n)
    big_l = np.tile(lam.reshape(-1, 1), (1, m))
    h = n ** (-1.0 / 3.0)
    big_h = h * big_l.T
    xx = (big_l - big_l.T) / big_h
    ftilde = (3 / 4 / np.sqrt(5)) * np.mean(np.maximum(1 - xx**2 / 5, 0) / big_h, axis=1)
    with np.errstate(divide="ignore", invalid="ignore"):
        hftemp = (-3 / 10 / np.pi) * xx + (3 / 4 / np.sqrt(5) / np.pi) * (1 - xx**2 / 5) * np.log(
            np.abs((np.sqrt(5) - xx) / (np.sqrt(5) + xx))
        )
    edge = np.abs(np.abs(xx) - np.sqrt(5)) < 1e-12
    hftemp[edge] = (-3 / 10 / np.pi) * xx[edge]
    hftilde = np.mean(hftemp / big_h, axis=1)
    if p <= n:
        dtilde = lam / ((np.pi * (p / n) * lam * ftilde) ** 2 + (1 - (p / n) - np.pi * (p / n) * lam * hftilde) ** 2)
    else:
        hftilde0 = (1 / np.pi) * (
            3 / 10 / h**2 + 3 / 4 / np.sqrt(5) / h * (1 - 1 / 5 / h**2) * np.log((1 + np.sqrt(5) * h) / (1 - np.sqrt(5) * h))
        ) * np.mean(1 / lam)
        dtilde0 = 1 / (np.pi * (p - n) / n * hftilde0)
        dtilde1 = lam / (np.pi**2 * lam**2 * (ftilde**2 + hftilde**2))
        dtilde = np.concatenate([np.full(p - n, dtilde0), dtilde1])
    return u @ np.diag(dtilde) @ u.T


def james_stein(x):
    t, n = x.shape
    means = x.mean(axis=0)
    grand = means.mean()
    sample = np.cov(x, rowvar=False, ddof=1)
    gap = means - grand
    distance = gap @ np.linalg.solve(sample, gap)
    intensity = min(1.0, max(0.0, (n - 3) / (t * distance))) if distance > 0 else 1.0
    return (1 - intensity) * means + intensity * grand, intensity


def ewma_mean(x, decay):
    return pd.DataFrame(x).ewm(alpha=1 - decay, adjust=True).mean().iloc[-1].to_numpy()


def case(name, x, factors):
    labels = [f"a{i}" for i in range(x.shape[1])]
    constant, shrinkage = cov_cor(x)
    # The James-Stein intensity needs the inverse of the sample covariance: more returns than
    # assets.
    js, intensity = james_stein(x) if x.shape[0] > x.shape[1] else (None, None)
    return {
        "name": name,
        "labels": labels,
        "columns": x.T.tolist(),
        "constant_correlation": {"matrix": constant.ravel().tolist(), "shrinkage": shrinkage},
        "statistical": {"factors": factors, "matrix": statistical(x, factors).ravel().tolist()},
        "denoised": denoised(x).ravel().tolist(),
        "nonlinear_shrinkage": analytical_shrinkage(x).ravel().tolist(),
        "james_stein": None if js is None else {"means": js.tolist(), "intensity": intensity},
        "ewma_mean": {"decay": 0.97, "means": ewma_mean(x, 0.97).tolist()},
    }


def main():
    rng = np.random.default_rng(20261104)
    # Eight assets driven by two factors, with fat-tailed noise: 120 months.
    loadings = rng.normal(0.0, 1.0, (8, 2))
    factors = 0.03 * rng.standard_t(6, (120, 2))
    many = factors @ loadings.T + 0.02 * rng.standard_t(5, (120, 8)) + rng.normal(0.005, 0.003, 8)
    # Thirty assets and twenty returns: more assets than returns.
    loadings = rng.normal(0.0, 1.0, (30, 3))
    factors = 0.02 * rng.standard_normal((20, 3))
    few = factors @ loadings.T + 0.01 * rng.standard_normal((20, 30)) + 0.001
    content = {
        "generated_by": "scripts/reference/estimators.py",
        "cases": [case("more returns than assets", many, 2), case("more assets than returns", few, 3)],
    }
    path = FIXTURES / "estimators.json"
    path.write_text(json.dumps(content, indent=1) + "\n")
    print(f"wrote {path.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
