# syntax=docker/dockerfile:1.4
FROM ubuntu:latest

# Setup base system (we install weston to easily get all the Wayland deps)
RUN apt-get update && \
    apt-get install -y sudo weston mesa-vulkan-drivers openssh-client git iputils-ping vulkan-tools curl iproute2 rsync

# Install the fish shell for a nicer experience (ADAPT TO YOUR OWN NEEDS)
RUN apt-get install -y fish

# Install development tools (ADAPT TO YOUR OWN NEEDS)
RUN apt-get install -y clang cmake ninja-build g++

# We put these args later to avoid excessive rebuilding
ARG USER
ARG PASSWORD

# Setup non-root user with a password for added security
RUN usermod -l $USER ubuntu -m -d /home/$USER && \
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
