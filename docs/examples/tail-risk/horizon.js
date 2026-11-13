import { tailRisk } from "finalytics";

const returns = [
    0.021, -0.013, 0.034, 0.008, -0.042, 0.017,
    0.026, -0.009, 0.012, 0.031, -0.024, 0.015,
];
const monthly = tailRisk(returns, 0.95, "parametric");
const rootTime = Math.sqrt(12);
console.log(`Monthly VaR: ${monthly.value_at_risk.toFixed(6)}`);
console.log(`Monthly ES: ${monthly.expected_shortfall.toFixed(6)}`);
const annualVar = monthly.value_at_risk * rootTime;
const annualEs = monthly.expected_shortfall * rootTime;
console.log(`Root-time annual VaR: ${annualVar.toFixed(6)}`);
console.log(`Root-time annual ES: ${annualEs.toFixed(6)}`);
