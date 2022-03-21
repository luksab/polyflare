import numpy as np
import matplotlib.pyplot as plt
import matplotlib as mpl

omp_data = np.genfromtxt("../omp.csv", delimiter=",")
# remove first column
omp_data = omp_data[:,1:]
omp_data = omp_data * omp_data # square
omp_data = omp_data / 2000 # divide by num_samples
omp_data = np.sqrt(omp_data) # square root

X = np.linspace(0, len(omp_data), len(omp_data))
plt.plot(X, omp_data)
plt.xlabel("step")
plt.ylabel("error")
plt.yscale("log")
plt.savefig("omp.png")
plt.savefig("omp.svg")
plt.clf()

sa_data = np.genfromtxt("../sim_ann.csv", delimiter=",")
# remove first column
sa_data = sa_data[:,1:]
# exchange column 0 and 1
sa_data = sa_data[:, [1, 0]]

sa_data = sa_data * sa_data # square
sa_data = sa_data / 2000 # divide by num_samples
sa_data = np.sqrt(sa_data) # square root

X = np.linspace(0, len(sa_data), len(sa_data))
# set plt to log axis
plt.yscale('log')
lineObjects = plt.plot(X, sa_data)
plt.legend(iter(lineObjects), ('iteration error', 'current error'))


plt.xlabel("step")
plt.ylabel("error")
plt.savefig("sim_ann.png")
plt.savefig("sim_ann.svg")
plt.clf()



# plot together
X = np.linspace(0, len(sa_data), len(sa_data))
X = X / len(sa_data)

# only look at actual error
sa_data = sa_data[:,1:]
plt.plot(X, sa_data, label="sa")
X = np.linspace(0, len(omp_data), len(omp_data))
X = X / len(omp_data)
plt.plot(X, omp_data, label="omp")
plt.legend()
plt.xlabel("completion")
plt.ylabel("error")
# plt.yscale("log")
plt.savefig("omp_vs_sim_ann.png")
plt.savefig("omp_vs_sim_ann.svg")
plt.clf()

