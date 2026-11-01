"""The data types Finalytics reads and returns, as frozen dataclasses.

They are generated from the engine's schema, so their fields and documentation match the Rust
types exactly. Unions such as :data:`Objective` or :data:`TransactionKind` have one class per
variant.
"""

from finalytics._types import *  # noqa: F403
from finalytics._types import __all__  # noqa: F401
