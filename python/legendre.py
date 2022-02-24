import numpy as np
from itertools import product

class Poly:
    def __init__(self,coef):
        if not isinstance(coef,np.ndarray):
            coef=np.array(coef)
        self.coef=coef

    def __call__(self, x):
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
        return 0.5*(f(1)-f(-1))

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

N=5
basis=[Poly([0]*i+[1]+[0]*(N-i-1)) for i in range(N)]
print(basis)

gramSchmidt(basis)
print(basis)
for i in range(N):
    print([round(basis[i]&basis[j], 4) for j in range(N)])

#types: x: 4d input; i,j,k,l: indices of the basis polynomials
# total poly: coefficents onto i,j,k,l
def poly4d(x,i,j,k,l):
    return basis[i](x[0]) * basis[j](x[1]) * basis[k](x[2]) * basis[l](x[3])
# to fit i,j,k,l: scalar product of basis[i,j,k,l] with the function to fit (numerically)
