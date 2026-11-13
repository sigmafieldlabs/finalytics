import { Performance } from "finalytics";

for (const timing of ["start_of_day", "end_of_day"]) {
  const performance = Performance.fromValues(
    ["2026-01-30", "2026-02-27"],
    [1050.0, 1600.0], [0.0, 500.0], timing,
  );
  console.log(timing, performance.timeWeighted().toFixed(6));
}
