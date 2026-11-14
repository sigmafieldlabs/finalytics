---
description: Keep WebAssembly work off the browser's main thread with the JavaScript worker client, remote handles and explicit cleanup.
---

# Run off the main thread

Finalytics' worker client is a JavaScript feature. It sends WebAssembly calls to a module worker and returns promises, leaving the page free to handle input and rendering.

## Hold assets in the worker

Create the worker from the package's module entry, then pass it to `createEngine`. Static dataset calls and constructors return either plain data or remote handles.

[remote-assets.js](../../examples/run-off-main-thread/remote-assets.js)

Call methods with `remote.call("method", ...)` and properties with `remote.get("property")`. Free remote objects when finished and terminate the worker in `finally`, including after an error.

## Return text and HTML explicitly

Worker results are data, not main-thread objects with convenience methods. Use remote `reportText` and `reportHtml` calls rather than trying to call `show()` on the returned report.

[remote-report.js](../../examples/run-off-main-thread/remote-report.js)

The example reports the HTML size so it also runs under Node.js. In a browser, give that HTML to your report frame or viewer; do not inject untrusted HTML into a page.

> [!NOTE]
> These examples are JavaScript-only because Python and Rust do not expose this browser worker client. Rust runs natively, and the site's Python and JavaScript guide runners already execute away from the visible page.

For an ordinary local JavaScript call, `show()` remains the simplest interface. The explicit worker route is for applications managing their own worker and rendering lifecycle.
