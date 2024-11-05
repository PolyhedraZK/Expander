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



#P.<x, y> = PolynomialRing(ZZ)
f = P((x+1)*(y^3+1))

f_lag = []
for o1 in omega1_powers:
    for o0 in omega0_powers:
        print(o0, o1)
        f_lag.append(f(o0, o1))
print(f)
print(f_lag)

g = f/P(y+1)
g_lag = []
for o1 in omega1_powers:
    for o0 in omega0_powers:
        print(o0, o1)
        g_lag.append(g(o0, o1))
print(g)
print(g_lag)

t = P(y+1)
t_lag = []
for o1 in omega1_powers:
    for o0 in omega0_powers:
        print(o0, o1)
        t_lag.append(t(o0, o1))
print(t)
print(t_lag)

for i in range(8):
    print(f_lag[i] - g_lag[i] *t_lag[i])

g_rec = P(0)
for i in range(8):
    g_rec += bases[i] * g_lag[i]
print(g_rec)

f_rec = P(0)
for i in range(8):
    f_rec += bases[i] * f_lag[i]
print(f_rec)

t_rec = P(0)
for i in range(8):
    t_rec += bases[i] * t_lag[i]
print(t_rec)


#poly: BivariatePolynomial { coefficients: [0x22f570bef606bed65c7096c6a0c817e609afcfe713ba31147f8bd51b251cabeb, 0x0507ca135e47bffb64abd5ae3354a7bfdb3153434c930089dd662b20932fc330, 0x21fa0dd9f736b1f3a1a3c04aac2531ac09644d652db07c85eaeb64fa7d6e19dd, 0x23e83443e10a4ff4ab587f214216873c3fc8c6ed9f0d3bc6765a3e33825c4a27, 0x0620576f0f43be5090f4454fa5a86123628037e211520624fff8f4e060146af4, 0x2ce9baa1967d600ca9df7cca0b75f47318fc3ed448d3948930d148cef30a837f, 0x03c948203ef5a50473a68eb3f20d5ed12b1866c106d62aef7c8eced2275ef406, 0x2b19eea023ee966c87bfbf6490c791577e9c015e3a07afb209000e7190f4afee], degree_0: 4, degree_1: 2 }
#point: (0x287b052408f6442905491ad99116c3d6e15673ef933dc3ee05bd02b22cf1a598, 0x0e495ae92321aac79e9db2666cc509872a1b9890585320b2c11c257b701ae079)
#f_x_b: [0x0e892e919ac142923a9c14b04fe1977064b57df066909bb9f504329f07d2ce95, 0x15aa88702ff4eeffdb2918b1398ed54ac7a82021a3af554afbd21e40c277c745, 0x206ae4ddcfd8b46f3d6eac253f3be7c872a46a64c262ef6fa80d87420722cf8c, 0x06f1006603a0f604cdcecc6a96cf33764cb551649fd1dc571a9c504c7f645bad]
#q_0_x_b: [0x1a7b6dc7ce5e3ccf739e2fff825616f7cc2098a6b46b9bc5936695e27ed31fc0, 0x2852fb8eed333e5fcdcae12f91da1192728f21178f9dba779f576e74ccf47365, 0x06f1006603a0f604cdcecc6a96cf33764cb551649fd1dc571a9c504c7f645bad, 0x0000000000000000000000000000000000000000000000000000000000000000]


p = P(0x22f570bef606bed65c7096c6a0c817e609afcfe713ba31147f8bd51b251cabeb + 0x0507ca135e47bffb64abd5ae3354a7bfdb3153434c930089dd662b20932fc330*x + 0x21fa0dd9f736b1f3a1a3c04aac2531ac09644d652db07c85eaeb64fa7d6e19dd*x^2 + 0x23e83443e10a4ff4ab587f214216873c3fc8c6ed9f0d3bc6765a3e33825c4a27*x^3 + 0x0620576f0f43be5090f4454fa5a86123628037e211520624fff8f4e060146af4 * y + 0x2ce9baa1967d600ca9df7cca0b75f47318fc3ed448d3948930d148cef30a837f*x*y+ 0x03c948203ef5a50473a68eb3f20d5ed12b1866c106d62aef7c8eced2275ef406*x^2*y + 0x2b19eea023ee966c87bfbf6490c791577e9c015e3a07afb209000e7190f4afee *x^3*y)

px = p(x, 0x0e495ae92321aac79e9db2666cc509872a1b9890585320b2c11c257b701ae079) 

f = P(0x0e892e919ac142923a9c14b04fe1977064b57df066909bb9f504329f07d2ce95 + 0x15aa88702ff4eeffdb2918b1398ed54ac7a82021a3af554afbd21e40c277c745*x + 0x206ae4ddcfd8b46f3d6eac253f3be7c872a46a64c262ef6fa80d87420722cf8c*x^2 + 0x06f1006603a0f604cdcecc6a96cf33764cb551649fd1dc571a9c504c7f645bad*x^3)