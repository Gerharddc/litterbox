FROM registry.opensuse.org/opensuse/tumbleweed:latest

# Setup base system (we install weston to easily get all the Wayland deps)
RUN zypper refresh && \
    zypper in -y sudo weston libvulkan_radeon libvulkan_intel openssh git iputils vulkan-tools

# Install development toolchain (ADAPT TO YOUR OWN NEEDS)
RUN zypper in -y rustup clang cmake ninja gcc-c++

ARG USER
ARG PASSWORD

# Setup non-root user with a password for added security
RUN useradd -m $USER && \
    echo "${USER}:${PASSWORD}" | chpasswd && \
    echo "${USER} ALL=(ALL) ALL" >> /etc/sudoers
WORKDIR /home/$USER

# We do not install things that go into $HOME here as they will get nuked
# once the home directory gets mounted. There are ways to work around this
# but it does not seem worth it for this use-case.
#
# A benefit of not installing such things here is also that they don't
# need to be re-installed when the container gets rebuilt.
