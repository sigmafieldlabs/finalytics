"""Threads: long engine calls release the GIL, so other Python threads keep running."""

import threading
import time

import numpy as np

import finalytics as fl


def stalls(work):
    """How long `work` took on another thread, and the longest this thread waited meanwhile."""
    done = threading.Event()

    def run():
        work()
        done.set()

    worker = threading.Thread(target=run)
    ticks = [time.perf_counter()]
    worker.start()
    while not done.is_set():
        time.sleep(0.002)
        ticks.append(time.perf_counter())
    worker.join()
    return ticks[-1] - ticks[0], max(b - a for a, b in zip(ticks, ticks[1:]))


def test_other_python_threads_run_while_the_engine_compares():
    assets = fl.Assets(fl.datasets.sp500(), risk_free_asset="T-Bills")
    duration, longest = stalls(
        lambda: assets.compare(
            methods=["minimum_variance", "maximum_sharpe", "risk_parity"],
            validation=fl.WalkForward(years=5),
            max_weight=0.1,
        )
    )
    # The engine's work takes most of the call; holding the GIL for it would stall this thread
    # for most of the call. Released, this thread keeps ticking.
    assert duration > 0.2, duration
    assert longest < duration / 2, (longest, duration)


def test_other_python_threads_run_while_a_problem_is_resampled():
    rng = np.random.default_rng(2026)
    loadings = rng.normal(size=(60, 60))
    covariance = loadings @ loadings.T * 1e-3 / 60 + np.eye(60) * 1e-3
    labels = [f"asset {i}" for i in range(60)]
    problem = fl.opt.Problem.from_covariance(
        fl.risk.matrix(covariance.tolist(), labels), expected_returns=list(rng.uniform(0.02, 0.1, 60))
    )
    duration, longest = stalls(
        lambda: problem.resampled(fl.opt.MeanVariance(risk_aversion=4.0), observations=120, samples=300, seed=1)
    )
    assert duration > 0.1, duration
    assert longest < duration / 2, (longest, duration)
