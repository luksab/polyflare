import numpy as np
import matplotlib.pyplot as plt

data = np.genfromtxt("coefficients.csv", delimiter=",")
data = data / data[0]

X = np.linspace(0, len(data), len(data))
plt.plot(X, data)
plt.savefig("coefficients.png")
