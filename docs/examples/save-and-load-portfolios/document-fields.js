import { datasets } from "finalytics";

const document = JSON.parse(datasets.portfolio().toJson());
console.log(document.format);
console.log(document.format_version);
console.log("Has market:", String("market" in document));
