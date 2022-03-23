import numpy as np
import matplotlib.pyplot as plt
import matplotlib as mpl

run_data = np.genfromtxt("10runs.csv", delimiter=",")

# standard deviation
print("std", np.std(run_data, axis=0))
# average
print("mean", np.mean(run_data, axis=0))

# violin plot
plt.violinplot(run_data, showmeans=True)

# plt.xlabel("run")
plt.title("SA run variation")
plt.ylabel("error")
# plt.yscale("log")
plt.savefig("runs.png")
plt.savefig("runs.svg")
plt.clf()
