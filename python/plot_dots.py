import numpy as np
import matplotlib.pyplot as plt
import matplotlib as mpl
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

# data = np.genfromtxt("dots,2,0.csv", delimiter=",")
data = np.loadtxt("dots/dots,2,0.csv", delimiter=",", skiprows=1)
data = data[(data[:,0]==0.02941176470588236)&(data[:,1]==0.02941176470588236)]
print(data.shape)
def sample(x):
    return int(np.floor((x+1)/2*N))
N=17*2
picture1=np.empty((N,N))
picture1[:,:]=np.nan
picture2=np.empty((N,N))
picture2[:,:]=np.nan
for row in data:
    ix=sample(row[2])
    iy=sample(row[3])
    picture1[ix,iy]=row[4]
    picture2[ix,iy]=row[5]
x = data[:,0]

plt.imshow(picture2,cmap=mpl.cm.gray)#, interpolation='nearest')
plt.show()
plt.savefig("dots,2,0.png")
