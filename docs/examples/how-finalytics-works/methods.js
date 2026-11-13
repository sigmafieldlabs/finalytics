import { methods } from "finalytics";

for (const method of methods()) {
  console.log(method.label);
}
