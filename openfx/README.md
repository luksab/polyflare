# Install by
## Linux
```
cargo build
sudo mkdir -p /usr/OFX/Plugins/ofx_polyflare.ofx.bundle/Contents/Linux-x86-64/
sudo cp target/debug/libopenfx.so /usr/OFX/Plugins/ofx_polyflare.ofx.bundle/Contents/Linux-x86-64/ofx_polyflare.ofx
```

# Fix frequent crashes
Because of a problem in the OFX Library, you'll need to disable multithreading in your video editing software.
