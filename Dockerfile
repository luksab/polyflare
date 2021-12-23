FROM nvidia/opengl:1.2-glvnd-runtime

WORKDIR /usr/src/wgpu_test
COPY . .

RUN apt update -y
RUN apt install libxcursor-dev libxrandr-dev libx11-dev libxxf86vm-dev libxi-dev -y
RUN apt install libvulkan1 libvulkan-dev mesa-vulkan-drivers -y
RUN apt install vulkan-utils cloc
# RUN cloc --read-lang-def=cloc_wgsl.txt gpu polynomial_optics

# enable intel GPU pass through
# still need to run "xhost +local:root" on the host before
# and "xhost -local:root" after using the container
# call container with
# docker run --volume=/tmp/.X11-unix:/tmp/.X11-unix --device=/dev/dri:/dev/dri --env="DISPLAY=$DISPLAY" -it wgpu_test /bin/bash
# RUN apt install libgl1-mesa-glx libgl1-mesa-dri -y
# RUN rm -rf /var/lib/apt/lists/*
# For some reasson, this is still broken on my machine

# enable NVidia GPU pass though
# docker run --volume=/tmp/.X11-unix:/tmp/.X11-unix --device=/dev/dri:/dev/dri  --runtime=nvidia -e NVIDIA_DRIVER_CAPABILITIES=compute,utility --env="DISPLAY=$DISPLAY" -it wgpu_test /bin/bash
#docker run --volume=/tmp/.X11-unix:/tmp/.X11-unix --runtime=nvidia -e NVIDIA_DRIVER_CAPABILITIES=all -e XAUTHORITY --env="DISPLAY=$DISPLAY" -it wgpu_test /bin/bash
# nvidia-container-runtime
ENV NVIDIA_VISIBLE_DEVICES \
    ${NVIDIA_VISIBLE_DEVICES:-all}
ENV NVIDIA_DRIVER_CAPABILITIES \
    ${NVIDIA_DRIVER_CAPABILITIES:+$NVIDIA_DRIVER_CAPABILITIES,}graphics

# Get Rust
# Get Ubuntu packages
RUN apt update -y
RUN apt install -y curl build-essential
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

RUN echo 'source $HOME/.cargo/env' >> $HOME/.bashrc

# ENV RUSTUP_HOME=/usr/local/rustup \
#     CARGO_HOME=/usr/local/cargo \
#     PATH=/usr/local/cargo/bin:$PATH
# ENV PATH="$HOME/.cargo/bin:$PATH"

RUN PATH="$HOME/.cargo/bin:$PATH" cargo build
# build using "docker build -t wgpu_test ."