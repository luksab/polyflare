# polyflare
wgpu implentation of polynomial optics for physically based lens flare rendering.

This uses the [polynomial_optics](https://github.com/luksab/polyflare/tree/master/polynomial_optics) library for the CPU implementation of dense and sparse polynomials.
Polynomial_optics also contains the CPU code for raytracing though the lens system and fitting a dense polynomial to it and selecting a sparse one from that.

All the GPU code is in the [gpu crate](https://github.com/luksab/polyflare/tree/master/gpu).

## Example output
![example](./images/screenshot_correct.png)
