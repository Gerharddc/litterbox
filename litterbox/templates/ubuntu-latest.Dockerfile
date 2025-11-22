# syntax=docker/dockerfile:1.4
FROM ubuntu:latest

ARG USER
ARG PASSWORD

# Setup base system (we install weston to easily get all the Wayland deps)
RUN apt-get update && \
    apt-get install -y sudo weston mesa-vulkan-drivers openssh-client git iputils-ping vulkan-tools curl

# Install the fish shell for a nicer experience
RUN apt-get install -y fish

# Install development tools (ADAPT TO YOUR OWN NEEDS)
RUN apt-get install -y clang cmake ninja-build g++ gh

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
#!/usr/bin/env fish

set MARKER "$HOME/.home-built"

# If the marker file already exists, exit early
if test -f "$MARKER"
    echo "Home already built; skipping."
    exec $SHELL -l
end

echo "Building home for the first time..."

# ------------------------------
# ADAPT THIS TO YOUR OWN NEEDS
# ------------------------------
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
curl -f https://zed.dev/install.sh | sh
fish_add_path -U "$HOME/.local/bin"

# Create the marker file to prevent re-running
touch "$MARKER"
echo "Done."

# Return to the normal shell
exec $SHELL -l
EOT

chmod +x /prep-home.sh
chown $USER /prep-home.sh
EOF

# Enter the fish shell by default
ENV SHELL=fish
RUN chsh -s /usr/bin/fish $USER
CMD ["fish", "/prep-home.sh"]
