import numpy as np
import matplotlib.pyplot as plt
import matplotlib as mpl

N=316
N=190
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
        img = ax.imshow(picture1, cmap=mpl.cm.gray)#, interpolation='nearest')
        plt.colorbar(img, ax=ax)
    else:
        img = ax.imshow(picture2, cmap=mpl.cm.gray)
        plt.colorbar(img, ax=ax)

def plot_poly_error(ax, name, picture = 0):
    data = np.loadtxt(f"dots/dots,{name}.csv", delimiter=",")
    data_poly = np.loadtxt(f"dots/poly,{name},{picture}.csv", delimiter=",")
    # data = data - data_poly

    picture1=np.empty((N,N))
    picture1[:,:]=np.nan
    picture2=np.empty((N,N))
    picture2[:,:]=np.nan
    for (row, poly) in zip(data, data_poly):
        ix=sample(row[0])
        iy=sample(row[1])
        picture1[ix,iy]=row[2] - poly[2]
        picture2[ix,iy]=row[3] - poly[2]

    ax.set_title("sparse error")
    if picture == 0:
        img = ax.imshow(picture1, cmap=mpl.cm.gray)#, interpolation='nearest')
        plt.colorbar(img, ax=ax)
    else:
        img = ax.imshow(picture2, cmap=mpl.cm.gray)
        plt.colorbar(img, ax=ax)

def plot_poly(ax, name, num = 0):
    data = np.loadtxt(f"dots/poly,{name},{num}.csv", delimiter=",")

    picture=np.empty((N,N))
    picture[:,:]=np.nan
    for row in data:
        ix=sample(row[0])
        iy=sample(row[1])
        picture[ix,iy]=row[2]

    ax.set_title("sparse poly")
    img = ax.imshow(picture, cmap=mpl.cm.gray)
    plt.colorbar(img, ax=ax)

def plot_de_poly(ax, name, num = 0):
    data = np.loadtxt(f"dots/depoly,{name},{num}.csv", delimiter=",")

    picture=np.empty((N,N))
    picture[:,:]=np.nan
    for row in data:
        ix=sample(row[0])
        iy=sample(row[1])
        picture[ix,iy]=row[2]

    ax.set_title("dense poly")
    img = ax.imshow(picture, cmap=mpl.cm.gray)
    plt.colorbar(img, ax=ax)

def plot_combined(name, num = 0):
    fig, axs = plt.subplots(2,2)
    fig.suptitle(f"ghost {name} axis {num}")
    plot_dot(axs[0, 0], name, num)
    plot_poly(axs[0, 1], name, num)
    plot_de_poly(axs[1,0], name, num)
    plot_poly_error(axs[1,1], name, num)
    plt.tight_layout()
    # plt.show()
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
