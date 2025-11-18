FROM ubuntu:latest

# Setup base system (we install weston to easily get all the Wayland deps)
RUN apt-get update && \
    apt-get install -y sudo weston mesa-vulkan-drivers openssh-client git iputils-ping vulkan-tools curl

# Install the fish shell for a nicer experience
RUN apt-get install -y fish

# Install development tools (ADAPT TO YOUR OWN NEEDS)
RUN apt-get install -y rustup clang cmake ninja-build g++ gh

ARG USER
ARG PASSWORD

# Setup non-root user with a password for added security
RUN usermod -l $USER ubuntu -m -d /home/$USER && \
    echo "${USER}:${PASSWORD}" | chpasswd && \
    echo "${USER} ALL=(ALL) ALL" >> /etc/sudoers
WORKDIR /home/$USER

# We do not install things that go into $HOME here as they will get nuked
# once the home directory gets mounted. There are ways to work around this
# but it does not seem worth it for this use-case.
#
# A benefit of not installing such things here is also that they don't
# need to be re-installed when the container gets rebuilt.

# Enter the fish shell by default
ENV SHELL=fish
RUN chsh -s /usr/bin/fish $USER
CMD ["fish"]
