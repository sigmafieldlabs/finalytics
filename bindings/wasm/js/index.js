// The entry point for browsers and bundlers: call `await init()` once before using the engine.
// The S&P 500 datasets are fetched from the package's files when first used, and reports show in
// a frame inside an element of the page, or in a new window.
import { install as installDatasets } from "./datasets.js";
import { install as installReports } from "./reports.js";

installDatasets(async (url) => {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`could not load ${url}: ${response.status} ${response.statusText}`);
  }
  return response.text();
});

installReports((html, element) => {
  if (typeof document === "undefined") {
    throw new Error("a report can only be shown on a page; use toHtml() for its HTML");
  }
  if (element) {
    const frame = document.createElement("iframe");
    frame.title = "Report";
    frame.style.cssText = "width: 100%; height: 900px; border: 0";
    // The frame grows to the height of the report once it has laid out.
    frame.addEventListener("load", () => {
      const page = frame.contentDocument?.documentElement;
      if (page) frame.style.height = `${page.scrollHeight}px`;
    });
    frame.srcdoc = html;
    element.replaceChildren(frame);
    return undefined;
  }
  const url = URL.createObjectURL(new Blob([html], { type: "text/html" }));
  if (!window.open(url, "_blank")) {
    URL.revokeObjectURL(url);
    throw new Error("the browser blocked a new window; pass an element to show the report in");
  }
  setTimeout(() => URL.revokeObjectURL(url), 60_000);
  return undefined;
});

export * from "./finalytics.js";
export { default } from "./finalytics.js";
