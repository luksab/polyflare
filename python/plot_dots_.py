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
N=316
def sample(x):
    return int(np.floor((x+1)/2*N))

# data = np.genfromtxt("dots,2,0.csv", delimiter=",")
def plot_dot(name, picture = 0):
    data = np.loadtxt(f"dots/dots,{name}.csv", delimiter=",")

    picture1=np.empty((N,N))
    picture1[:,:]=np.nan
    picture2=np.empty((N,N))
    picture2[:,:]=np.nan
    for row in data:
        ix=sample(row[0])
        iy=sample(row[1])
        picture1[ix,iy]=row[2]
        picture2[ix,iy]=row[3]

    if picture == 0:
        plt.imshow(picture1, cmap=mpl.cm.gray)#, interpolation='nearest')
    else:
        plt.imshow(picture2, cmap=mpl.cm.gray)
    plt.savefig(f"dots/dots,{name},{picture}.png")
    plt.clf()

def plot_poly(name, num = 0):
    data = np.loadtxt(f"dots/poly,{name},{num}.csv", delimiter=",")

    picture=np.empty((N,N))
    picture[:,:]=np.nan
    for row in data:
        ix=sample(row[0])
        iy=sample(row[1])
        picture[ix,iy]=row[2]

    plt.imshow(picture, cmap=mpl.cm.gray)
    plt.savefig(f"dots/poly,{name},{num}.png")
    plt.clf()

def plot_de_poly(name, num = 0):
    data = np.loadtxt(f"dots/depoly,{name},{num}.csv", delimiter=",")

    picture=np.empty((N,N))
    picture[:,:]=np.nan
    for row in data:
        ix=sample(row[0])
        iy=sample(row[1])
        picture[ix,iy]=row[2]

    plt.imshow(picture, cmap=mpl.cm.gray)
    plt.savefig(f"dots/depoly,{name},{num}.png")
    plt.clf()

for i in range(1,6):
    plot_dot(i, 0)
    plot_dot(i, 1)
    plot_poly(i, 0)
    plot_poly(i, 1)
    plot_de_poly(i, 0)
    plot_de_poly(i, 1)
