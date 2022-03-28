import numpy as np
import matplotlib.pyplot as plt
import matplotlib as mpl

run_data = np.genfromtxt("10runs.csv", delimiter=",")
run_data_nofit = np.genfromtxt("10runs_nofit.csv", delimiter=",")

# standard deviation
print("std fit", np.std(run_data, axis=0))
print("std nofit", np.std(run_data_nofit, axis=0))
# average
print("mean fit", np.mean(run_data, axis=0))
print("mean nofit", np.mean(run_data_nofit, axis=0))

# violin plot
# plt.violinplot(run_data, showmeans=True)
plt.hist(run_data, bins=10, weights=np.zeros_like(run_data) + 1. / run_data.size)

# plt.xlabel("run")
plt.title("SA run variation")
plt.xlabel("error")
plt.ylabel("frequency")
# plt.yscale("log")
plt.savefig("runs.png")
plt.savefig("runs.svg")
plt.clf()

### no fit
# violin plot
# plt.violinplot(run_data_nofit, showmeans=True)
# histogram
plt.hist(run_data_nofit, bins=10, weights=np.zeros_like(run_data_nofit) + 1. / run_data_nofit.size)

# plt.xlabel("run")
plt.title("SA_no_fit run variation")
plt.xlabel("error")
plt.ylabel("frequency")
# plt.yscale("log")
plt.savefig("runs_nofit.png")
plt.savefig("runs_nofit.svg")
plt.clf()