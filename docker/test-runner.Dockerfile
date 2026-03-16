FROM rust:1.86-bookworm

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        file \
        git \
        libasound2-dev \
        libgl1-mesa-dev \
        libglib2.0-dev \
        libgtk-3-dev \
        libwayland-dev \
        libx11-dev \
        libxcb-render0-dev \
        libxcb-shape0-dev \
        libxcb-xfixes0-dev \
        libxcb1-dev \
        libxcursor-dev \
        libxi-dev \
        libxinerama-dev \
        libxkbcommon-dev \
        libxrandr-dev \
        pkg-config \
        tar \
        xz-utils \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace
