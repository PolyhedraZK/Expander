# Generate test vectors for the field implementations using SageMath
# Usage: sage test_vectors.sage

# BabyBear Field
p = 2**31 - 2**27 + 1
F = GF(p)
R.<x> = F[]

# Degree 3 extension
K.<a> = F.extension(x^3 - 2)
b = 1 + 2*a + 3*a^2
c = 4 + 5*a + 6*a^2
print("BabyBear Degree 3 Extension")
print(f"b = {b}")
print(f"c = {c}")
print(f"b*c = {b*c}")
print(f"b^(-1) = {b^(-1)}")
print(f"b^(11) = {b^(11)}")

# Degree 4 extension
K.<a> = F.extension(x^4 - 11)
b = 1 + 2*a + 3*a^2 + 4*a^3
c = 5 + 6*a + 7*a^2 + 8*a^3
print("BabyBear Degree 4 Extension")
print(f"b = {b}")
print(f"c = {c}")
print(f"b*c = {b*c}")
print(f"b^(-1) = {b^(-1)}")
print(f"b^(11) = {b^(11)}")
