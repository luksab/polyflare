import numpy as np
import matplotlib.pyplot as plt
import matplotlib as mpl

run_data = np.genfromtxt("data/10runs.csv", delimiter=",")
run_data_nofit = np.genfromtxt("data/10runs_nofit.csv", delimiter=",")

run_omp_cheap = np.genfromtxt("data/omp_cheap10Runs.csv", delimiter=",")
# remove first column
run_omp_cheap = run_omp_cheap[:,1:]
run_omp = np.genfromtxt("data/omp_10Runs.csv", delimiter=",")
# remove first column
run_omp = run_omp[:,1:]
# standard deviation and mean
print(f"omp_cheap: mean: {np.mean(run_omp_cheap)}, std: {np.std(run_omp_cheap)}")
print(f"omp: mean: {np.mean(run_omp)}, std: {np.std(run_omp)}")

# violin plot
# plt.violinplot(run_omp, showmeans=True)
plt.hist(run_omp, bins=10, weights=np.zeros_like(run_omp) + 1. / run_omp.size)

# plt.xlabel("run")
plt.title("OMP with replacement run variation")
plt.xlabel("error")
plt.ylabel("frequency")
# plt.yscale("log")
plt.savefig("runs_omp_full.png")
plt.savefig("runs_omp_full.svg")
plt.clf()

# violin plot
# plt.violinplot(run_omp_cheap, showmeans=True)
plt.hist(run_omp_cheap, bins=10, weights=np.zeros_like(run_omp_cheap) + 1. / run_omp_cheap.size)
# plt.xlabel("run")
plt.title("OMP run variation")
plt.xlabel("error")
plt.ylabel("frequency")
# plt.yscale("log")
plt.savefig("runs_omp_cheap.png")
plt.savefig("runs_omp_cheap.svg")
plt.clf()



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