import finalytics as fl
import finalytics.pandas as flpd

performance = fl.datasets.portfolio().performance()
frame = flpd.frame(performance.contributions())
columns = ["holding", "contribution", "currency_effect"]
print(frame[columns].round(6).to_string(index=False))
