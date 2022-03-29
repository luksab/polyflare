import numpy as np
import matplotlib.pyplot as plt
# import os

# directory = os.fsencode("./")
    
# for file in os.listdir(directory):
#     filename = os.fsdecode(file)
#     if filename.endswith(".csv"):
#         data = np.genfromtxt(filename, delimiter=",")
#         data = data / data[0]

#         X = np.linspace(0, len(data), len(data))
#         plt.plot(X, data)
#         plot_name = filename.split(".")[0] + ".png"
#         print(plot_name)
#         plt.savefig(plot_name)

data = np.genfromtxt("data/coefficients sparse.csv", delimiter=",")
data = data / data[0]

X = np.linspace(0, len(data), len(data))
plt.plot(X, data)
plt.savefig("coefficients sparse.png")
