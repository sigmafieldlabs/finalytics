import { drawdowns } from "finalytics";

const levels = [100.0, 95.0, 101.0, 90.0, 92.0, 104.0];
const dates = [
  "2026-01-02", "2026-01-05", "2026-01-06",
  "2026-01-07", "2026-01-08", "2026-01-09",
];
for (const item of drawdowns(dates, levels)) {
  const recovery = item.recovery ?? "open";
  const days = item.days_to_recovery ?? "None";
  console.log(
    `${item.peak} ${item.trough} ${recovery} ` +
    `${item.depth.toFixed(6)} ${item.days_to_trough} ${days}`,
  );
}
