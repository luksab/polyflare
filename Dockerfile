FROM rust:1.55

WORKDIR /usr/src/wgpu_test
COPY . .

RUN apt update -y
RUN apt install libxcursor-dev libxrandr-dev libx11-dev libxxf86vm-dev libxi-dev -y
RUN apt install libvulkan-dev -y

# enable intel GPU pass through
# still need to run "xhost +local:root" on the host before
# and "xhost -local:root" after using the container
# call container with
# docker run --volume=/tmp/.X11-unix:/tmp/.X11-unix --device=/dev/dri:/dev/dri --env="DISPLAY=$DISPLAY" -it wgpu_test /bin/bash
RUN apt install libgl1-mesa-glx libgl1-mesa-dri -y
RUN rm -rf /var/lib/apt/lists/*
# For some reasson, this is still broken on my machine

RUN cargo build
# build using "docker build -t wgpu_test ."