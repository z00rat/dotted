FROM archlinux:latest

# Update system packages and install dependencies
RUN pacman -Syu --noconfirm && pacman -S --noconfirm base-devel git sudo curl unzip fish flatpak trash-cli eza zoxide bat micro xdg-user-dirs gitui

# Create a non-root user 'user1' with fish as default shell and passwordless sudo permissions
RUN useradd -m -s /usr/bin/fish user1 && echo "user1 ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/user1

# Add Flathub remote for flatpak testing
RUN flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo

USER user1
ENV HOME=/home/user1
ENV LANG=en_US.UTF-8
WORKDIR /home/user1


# Configure fish aliases
RUN mkdir -p /home/user1/.config/fish && \
    printf "%s\n" \
    "alias l 'eza --color=auto -lA --icons=auto --smart-group --time-style=relative'" \
    "alias ll 'eza --color=auto -la --icons=auto --smart-group --time-style=relative'" \
    "alias ls 'eza --color=auto -l --icons=auto --smart-group --time-style=relative'" \
    "alias mkdir 'mkdir -pv'" \
    "alias mv 'mv -iv'" \
    "alias cp 'cp -v'" \
    "alias diff 'diff --color=auto'" \
    "alias fgrep 'fgrep --color=auto'" \
    >> /home/user1/.config/fish/config.fish

RUN xdg-user-dirs-update

# Copy test environment metadata to standard ~/.local/share/dotted
RUN mkdir -p /home/user1/.local/share/dotted
COPY --chown=user1:user1 test-dotted/ /home/user1/.local/share/dotted/

# Copy test-home inside the user1's home dir
COPY --chown=user1:user1 test-home/ /home/user1/

USER root
# Copy pre-built dotted binary
COPY dotted /usr/local/bin/dotted
RUN chmod +x /usr/local/bin/dotted

USER user1
# Generate fish completions for dotted
RUN mkdir -p /home/user1/.config/fish/completions && /usr/local/bin/dotted shell completions fish > /home/user1/.config/fish/completions/dotted.fish

ENV SHELL=/usr/bin/fish
CMD ["/usr/bin/fish"]
