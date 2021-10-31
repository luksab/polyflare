# polyflare
wgpu implentation of polynomial optics for physically based lens flare rendering.

This uses the [polynomial_optics](https://github.com/luksab/polynomial_optics) library for the CPU implementation of dense and sparse polynomials.
Polynomial_optics also contains the code for raytracing though the lens system and fitting a dense polynomial to it and selecting a sparse one from that.
