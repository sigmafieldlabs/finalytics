import { datasets, readDocument, writeDocument } from "finalytics";

const original = datasets.portfolio().data;
const restored = readDocument(writeDocument(original)).portfolio;
console.log("Equal records:", JSON.stringify(original) ===
  JSON.stringify(restored));
console.log("Instruments:", restored.instruments.length);
