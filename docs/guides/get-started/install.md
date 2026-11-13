---
description: Install Finalytics in Python, JavaScript or Rust and check that the engine runs.
---

# Install Finalytics

Finalytics is one Rust engine with packages for Python, JavaScript and Rust. Choose the
package for your language; you do not need to install Rust to use Python or JavaScript.

## Install the package

These guides accompany Finalytics 1.0.0-rc.7. Until the stable 1.0.0 release, install that
release candidate explicitly:

| Language | Command |
| --- | --- |
| Python | `pip install finalytics==1.0.0rc7` |
| JavaScript | `npm install finalytics@1.0.0-rc.7` |
| Rust | `cargo add finalytics@1.0.0-rc.7` |

Python requires Python 3.10 or later. The asset-class examples use pandas through
`fl.datasets.asset_class_prices()`; install it with the package if you do not have it:

```sh
pip install "finalytics[pandas]==1.0.0rc7"
```

Without pandas, use `fl.datasets.asset_class_table()` instead. JavaScript includes
TypeScript types and runs in Node.js, browsers and workers. Rust requires Rust 1.88 or
later.

## Check the installation

Print the engine's version. The same function is available in every language, so this is a
quick check that the package and its native engine are ready:

[version.py](../../examples/install/version.py)

> [!NOTE]
> Python also exposes the distribution version as `fl.__version__`. Python writes this
> pre-release as `1.0.0rc7`, while `fl.version()` reports the engine version shown above.

## Initialise a browser

Node.js initialises the WebAssembly engine when you import it. In your own browser
application, import the default export and initialise it once before the first call:

```js
import init, { version } from "finalytics";

await init();
console.log(version());
```

The runnable examples on this site already do this for you. A new web worker or another
browser context needs its own initialisation.

## Keep the version with your results

Every text report ends with the Finalytics version and a digest of its inputs. Keep that
output with your analysis when you compare results or upgrade packages. The next guide
builds a complete portfolio report from the bundled household dataset.
