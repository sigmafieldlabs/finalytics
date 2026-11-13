import { datasets } from "finalytics";

const household = datasets.portfolio().household(undefined, {
  group_by: ["owner"],
});
console.log("Household:", household.household.time_weighted.toFixed(6));
for (const account of household.accounts) {
  console.log(account.name, account.time_weighted.toFixed(6),
    account.contribution.toFixed(6));
}
