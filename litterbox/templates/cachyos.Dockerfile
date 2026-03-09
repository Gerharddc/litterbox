# syntax=docker/dockerfile:1.4
# adjust according to your CPU architecture level
FROM docker.io/cachyos/cachyos-v3:latest

# Setup base system (install essential packages)
RUN pacman -Syu --noconfirm && \
    pacman -S --noconfirm sudo wayland mesa vulkan-tools vulkan-radeon vulkan-intel openssh git iputils curl iproute2 rsync

# Install the fish shell for a nicer experience (ADAPT TO YOUR OWN NEEDS)
RUN pacman -S --noconfirm fish

# Install development toolchain and additional package managers (ADAPT TO YOUR OWN NEEDS)
RUN pacman -S --noconfirm base-devel paru mise

# We put these args later to avoid excessive rebuilding
ARG USER
ARG PASSWORD

# Setup non-root user with a password for added security
RUN useradd -m $USER && \
    echo "${USER}:${PASSWORD}" | chpasswd && \
    echo "${USER} ALL=(ALL) ALL" >> /etc/sudoers
WORKDIR /home/$USER

# We do not install things directly into $HOME here as they will get nuked
# once the home directory gets mounted. Instead we use a script that runs
# at start-up to construct the home directory the first time.
#
# A benefit of not installing things directly into home means that they do
# need to be re-installed when the container gets rebuilt.
RUN <<'EOF'
# Create the script using a nested heredoc
cat <<'EOT' > /prep-home.sh
#!/usr/bin/env sh

# -------------------------------------
# ADAPT THIS EXAMPLE TO YOUR OWN NEEDS
# -------------------------------------
# curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
EOT

chmod +x /prep-home.sh
chown $USER /prep-home.sh
EOF

# Set LANG to enable UTF-8 support
ENV LANG=en_US.UTF-8

# Enter the fish shell by default
ENV SHELL=fish
RUN chsh -s /usr/bin/fish $USER
