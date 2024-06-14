# this script is used to generate test vectors for bi-kzg

r = 21888242871839275222246405745257275088548364400416034343698204186575808495617
omega0 = 0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000000
omega1 = 0x30644e72e131a029048b6e193fd841045cea24f6fd736bec231204708f703636
n = 2
m = 4

print(omega0^2%r)
print(omega1^4%r)

P.<x, y> = PolynomialRing(Zmod(r))
f = P(1 + 2*x + 3*y + 4*x*y + 5*y^2 + 6*x*y^2 + 7*y^3 + 8*x*y^3)

omega0_powers = [1, omega0 ]
omega1_powers = [1, omega1, omega1^2 % r, omega1^3 % r]

f_lag = []
for o1 in omega1_powers:
    for o0 in omega0_powers:
        print(o0, o1)
        f_lag.append(f(o0, o1))

poly_lag_coeff = [0x0000000000000000000000000000000000000000000000000000000000000024, 0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593effffffd, 0x00000000000000059e26bcea0d48bac65a4e1a8be2302529067f891b047e4e50, 0x0000000000000000000000000000000000000000000000000000000000000000, 0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593effffff9, 0x0000000000000000000000000000000000000000000000000000000000000000, 0x30644e72e131a0241a2988cc74389d96cde5cdbc97894b683d626c78eb81b1a1, 0x0000000000000000000000000000000000000000000000000000000000000000]

t0 = []
for i in range(n):
    tmp = P(1)
    for k in range(n):
        if i!=k:
            tmp *= P((omega0_powers[k]-x)/(omega0_powers[k]-omega0_powers[i]))
    t0.append(tmp)  
print("omega0")
for t in t0:
    print(t(5, 7))

t1 = []
for i in range(m):
    tmp = P(1)
    for k in range(m):
        if i!=k:
            tmp *= P((omega1_powers[k]-y)/(omega1_powers[k]-omega1_powers[i]))
    t1.append(tmp)  

print("omega1")
for t in t1:
    print(t(5, 7))
    
bases = []

for t10 in t1:
    for t00 in t0:
        t = t00*t10
        bases.append(t)

res = P(0)
for i in range(n*m):
    res += bases[i] * poly_lag_coeff[i]

print(res - f)
print()
for t in bases:
    print(t(5, 7))