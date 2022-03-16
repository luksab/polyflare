import numpy as np
from itertools import product
import matplotlib.pyplot as plt

class Poly:
    def __init__(self,coef):
        if not isinstance(coef,np.ndarray):
            coef=np.array(coef)
        self.coef=coef

    def __call__(self, x):
        if isinstance(x,np.ndarray):
            return np.sum(self.coef*x[:,np.newaxis]**np.arange(len(self.coef)),axis=1)
        return np.sum(self.coef*x**np.arange(len(self.coef)))

    def antiDerive(self):
        return Poly(np.insert(self.coef/np.arange(1, len(self.coef)+1), 0, 0))
    
    def __len__(self):
        return len(self.coef)

    def __iter__(self):
        return iter(self.coef)

    def __add__(self, b):
        res=np.zeros(max(len(self),len(b)))
        res[:len(self)]+=self.coef
        res[:len(b)]+=b.coef
        return Poly(res)

    def __sub__(self, b):
        res=np.zeros(max(len(self),len(b)))
        res[:len(self)]+=self.coef
        res[:len(b)]-=b.coef
        return Poly(res)

    def __mul__(self, b):
        if isinstance(b, Poly):
            res=np.zeros(len(self)+len(b)-1)
            for (i,a),(j,b) in product(enumerate(self),enumerate(b)):
                res[i+j]+=a*b
            return Poly(res)
        else:
            return Poly(self.coef*b)

    def __and__(self,b): # scalar product
        f = (self*b).antiDerive()
        return (f(1)-f(-1))

    def grid(self,b, num_samples): # scalar product
        sum = 0
        for i in range(num_samples):
            # range from -1 to 1 inclusive
            x = i/(num_samples-1)*2-1
            # print(i, x)
            # x = 2*i/num_samples - 1
            sum += self(x)*b(x)
        return sum/num_samples*2

    # def __repr__(self):
    #    return "Poly({})".format(self.coef)
    def __repr__(self):
        return str(self)

    def __str__(self):
        string = []
        for i,c in enumerate(self.coef):
            if c!=0:
                if i == 0:
                    string.append("{}".format(c))
                elif i == 1:
                    string.append("{}x".format(c))
                else:
                    string.append("{}x^{}".format(c,i))
        return " + ".join(string)

def gramSchmidt(basis):
    for i in range(len(basis)):
        for j in range(i):
            basis[i] -= basis[j]*(basis[j]&basis[i])
        basis[i] *= 1/np.sqrt(basis[i]&basis[i])

def gramSchmidtGrid(basis, num_samples):
    for i in range(len(basis)):
        for j in range(i):
            integrate = basis[j].grid(basis[i], num_samples)
            basis[i] -= basis[j]*(integrate)
        integrate = basis[i].grid(basis[i], num_samples)
        basis[i] *= 1/np.sqrt(integrate)

N=5
LGbasis=[Poly([0]*i+[1]+[0]*(N-i-1)) for i in range(N)]
print(LGbasis)
gramSchmidt(LGbasis)
print(LGbasis)
for i in range(N):
    print([round(LGbasis[i]&LGbasis[j], 4) for j in range(N)])

X=np.linspace(-1,1,1000)
for i in range(N):
    plt.plot(X, LGbasis[i](X))

plt.title(f'legendre polynomials')
plt.savefig(f'basis/legendre.svg')
plt.clf()

samples = [5, 10, 100, 1000]
for num_samples in samples:
    basis=[Poly([0]*i+[1]+[0]*(N-i-1)) for i in range(N)]
    print(basis)
    gramSchmidtGrid(basis, num_samples)
    print(basis)
    
    for i in range(N):
        print(f"basis {num_samples}",[round(basis[i].grid(basis[j], num_samples), 4) for j in range(N)])

    X=np.linspace(-1,1,1000)
    for i in range(N):
        plt.plot(X, basis[i](X))

    plt.title(f'{num_samples} samples')
    plt.savefig(f'basis/grid{num_samples}.svg')
    plt.clf()

errors = []
samples = [100, 200, 250, 400, 500, 700, 1000, 2000, 3500, 5000, 7000, 10000]
for num_samples in samples:
    error = 0
    for i in range(N):
        for j in range(N):
            if i == j:
                error += abs(LGbasis[i].grid(LGbasis[j], num_samples) - 1)
            else:
                error += abs(LGbasis[i].grid(LGbasis[j], num_samples))
        print(f"basis {num_samples}",[round(basis[i]&basis[j], 4) for j in range(N)])
    errors.append(error)
    print(f"error {num_samples}", error)

# plt.yscale('log')
# plt.xscale('log')
plt.xlabel("N")
plt.ylabel("Error")
plt.title("Linear axis")
plt.plot(samples, errors)
plt.savefig(f'basis/legendreError.png')
plt.savefig(f'basis/legendreError.svg')

plt.yscale('log')
plt.xscale('log')
plt.title("Log-log axis")
plt.plot(samples, errors)
plt.savefig(f'basis/legendreErrorLog.png')
plt.savefig(f'basis/legendreErrorLog.svg')

#types: x: 4d input; i,j,k,l: indices of the basis polynomials
# total poly: coefficents onto i,j,k,l
def poly4d(x,i,j,k,l):
    return LGbasis[i](x[0]) * LGbasis[j](x[1]) * LGbasis[k](x[2]) * LGbasis[l](x[3])
# to fit i,j,k,l: scalar product of basis[i,j,k,l] with the function to fit (numerically)
