import re

import finalytics


def test_package_version_follows_python_conventions() -> None:
    assert re.fullmatch(r"\d+\.\d+\.\d+((a|b|rc)\d+)?", finalytics.__version__)


def test_engine_version_matches_the_package() -> None:
    engine = finalytics.version()
    assert re.fullmatch(r"\d+\.\d+\.\d+(-(alpha|beta|rc)\.\d+)?", engine)
    assert engine.split("-")[0] == re.split(r"[a-z]", finalytics.__version__)[0]
