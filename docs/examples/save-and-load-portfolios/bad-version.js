import { Portfolio } from "finalytics";

try {
  Portfolio.fromJson(
    '{"format":"finalytics.portfolio","format_version":2,' +
    '"portfolio":{}}',
  );
} catch (error) {
  for (const issue of error.issues) {
    console.log(issue.path, "|", issue.message);
  }
}
