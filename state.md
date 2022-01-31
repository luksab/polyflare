# Ray tracing
- [x] naive ray tracing
  - [x] GPU implementation
- [x] sparse ray tracing
  - [x] GPU implementation
    - [x] ray tracing itself
    - [x] triangle area calculation
- [x] autodiff CPU version

# Rendering
- [x] naive
- [x] sparse
- [ ] (autodiff 1/gradient as strength) - use polynomials here
  - [ ] maybe as comparison to polynomials

# Polynomials
- [ ] make Dense degree correct
  - [ ] implement exponent generation function
  - [ ] use generated exponents

# Fitting Polynomials
- [x] proper ray generation 
- [x] Dense polynomials - this is currently broken
  - [x] Sparse from polynomials
    - [x] fix used_monomials ~~hashMap~~

# Plugin
- [ ] OpenFX
  - [ ] Rust wrapper
  - [ ] CPU only implementation
  - [ ] GPU implementation by forcing OpenGL
