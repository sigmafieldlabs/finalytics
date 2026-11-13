import { datasets } from "finalytics";

for (const account of datasets.portfolio().data.accounts) {
  console.log(account.id, account.name, account.taxable ?? true,
    JSON.stringify(account.classifications));
}
