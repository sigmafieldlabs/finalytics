"""The data types Finalytics reads and returns, as frozen dataclasses.

They are generated from the engine's schema, so their fields and documentation are those of the
data the engine reads and writes. Unions such as :data:`Objective` or :data:`TransactionKind`
have one class per variant, named after the variant (with the union's name added where a bare name
would be unclear); the ``type`` of each class gives the variant's name in the data.
"""

from finalytics._types import *  # noqa: F403
from finalytics._types import __all__  # noqa: F401
