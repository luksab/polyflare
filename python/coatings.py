import numpy as np
import matplotlib.pyplot as plt
import matplotlib as mpl

data = np.genfromtxt("coating_wl_sweep.csv", delimiter=",")
# X = fist column
X = data[:,0]
# remove first column
data = data[:,1:]
# # remove last column
# data = data[:,:-1]

# X = np.linspace(0, len(data), len(data))
lineObjects = plt.plot(X, data)
plt.legend(iter(lineObjects), ('entry', 'exit'))
# plt.yscale("log")
plt.xlabel("wavelength / µm")
plt.ylabel("reflectance")
plt.title("reflectance of bk7 with coating optimized for 0.5 µm")
plt.savefig("coating_wl_sweep.png")
plt.savefig("coating_wl_sweep.svg")
plt.clf()


data = np.genfromtxt("coating_angle_sweep.csv", delimiter=",")
# X = fist column, converted from rad to deg
X = data[:,0] * 180 / np.pi
# remove first column
data = data[:,1:]
# # remove last column
# data = data[:,:-1]

# X = np.linspace(0, len(data), len(data))
lineObjects = plt.plot(X, data)
plt.legend(iter(lineObjects), ('entry', 'exit'))
# plt.yscale("log")
plt.xlabel("angle in rad")
plt.ylabel("reflectance")
plt.title("reflectance of bk7 with coating optimized for 0.5 µm")
plt.savefig("coating_angle_sweep.png")
plt.savefig("coating_angle_sweep.svg")
plt.clf()