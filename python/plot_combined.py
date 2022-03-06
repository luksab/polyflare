import numpy as np
import matplotlib.pyplot as plt
import matplotlib as mpl

N=316
def sample(x):
    return int(np.floor((x+1)/2*N))

def plot_dot(ax, name, picture = 0):
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

    ax.set_title("data")
    if picture == 0:
        ax.imshow(picture1, cmap=mpl.cm.gray)#, interpolation='nearest')
    else:
        ax.imshow(picture2, cmap=mpl.cm.gray)

def plot_poly(ax, name, num = 0):
    data = np.loadtxt(f"dots/poly,{name},{num}.csv", delimiter=",")

    picture=np.empty((N,N))
    picture[:,:]=np.nan
    for row in data:
        ix=sample(row[0])
        iy=sample(row[1])
        picture[ix,iy]=row[2]

    ax.set_title("sparse poly")
    ax.imshow(picture, cmap=mpl.cm.gray)

def plot_de_poly(ax, name, num = 0):
    data = np.loadtxt(f"dots/depoly,{name},{num}.csv", delimiter=",")

    picture=np.empty((N,N))
    picture[:,:]=np.nan
    for row in data:
        ix=sample(row[0])
        iy=sample(row[1])
        picture[ix,iy]=row[2]

    ax.set_title("dense poly")
    ax.imshow(picture, cmap=mpl.cm.gray)

def plot_combined(name, num = 0):
    fig, axs = plt.subplots(2,2)
    fig.suptitle(f"ghost {name} axis {num}")
    plot_dot(axs[0, 0], name, num)
    plot_poly(axs[0, 1], name, num)
    plot_de_poly(axs[1,0], name, num)
    plt.tight_layout()
    fig.savefig(f"dots/combinded,{name},{num}.png")

for i in range(1,6):
    print(f"plotting ghost {i}")
    plot_combined(name = i, num = 0)
    plot_combined(name = i, num = 1)

print("combining images")
import os
os.system('convert dots/combinded,*,0.png +append result-sprite0.png')
os.system('convert dots/combinded,*,1.png +append result-sprite1.png')
os.system('convert result-sprite0.png result-sprite1.png -append result-sprite.png')
