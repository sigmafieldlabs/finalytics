import { datasets } from "finalytics";

const household = datasets.portfolio().household(undefined, {
  group_by: ["owner", "wrapper"],
});
for (const scheme of household.groups) {
  for (const group of scheme.groups) {
    console.log(scheme.scheme, group.name, group.accounts.join(","),
      group.contribution.toFixed(6));
  }
}
