# syntax=docker/dockerfile:1.4
# adjust according to your CPU architecture level
FROM docker.io/cachyos/cachyos-v3:latest

# Setup base system (install essential packages)
RUN pacman -Syu --noconfirm && \
    pacman -S --noconfirm sudo wayland mesa vulkan-tools vulkan-radeon vulkan-intel openssh git iputils curl iproute2 rsync

# Install the fish shell for a nicer experience
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
