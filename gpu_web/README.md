# Build for the web

Add the `wasm32-unknown-unknown` target if you haven't already:
```
rustup target add wasm32-unknown-unknown
```

Then build the `wasm32-unknown-unknown` target:

```
RUSTFLAGS=--cfg=web_sys_unstable_apis cargo build  --no-default-features --target wasm32-unknown-unknown -p gpu_web 
```

And finally, run wasm-bindgen:
```
wasm-bindgen --out-dir target/generated --web target/wasm32-unknown-unknown/debug/gpu_web.wasm
```

Then you can open the `index.html` file in a [supported](https://caniuse.com/webgpu) browser.
