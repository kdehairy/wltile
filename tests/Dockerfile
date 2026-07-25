FROM archlinux:latest

RUN pacman -Syu --noconfirm \
    && pacman -S --noconfirm \
        base-devel \
        libcap \
        rust \
        sway \
				swaybg \
    && pacman -Scc --noconfirm

# sway refuses to run as root; create a dedicated test user.
RUN useradd -m -u 1000 testuser

# XDG_RUNTIME_DIR must exist, be owned by the user, and be mode 0700.
ENV XDG_RUNTIME_DIR=/run/user/1000
RUN mkdir -p /run/user/1000 \
    && chown testuser:testuser /run/user/1000 \
    && chmod 700 /run/user/1000

WORKDIR /home/testuser/app
COPY --chown=testuser:testuser . .
RUN chown testuser:testuser /home/testuser/app

USER testuser

# Pre-fetch dependencies so repeated runs are faster when only source changes.
RUN cargo fetch

CMD ["timeout", "--kill-after=10s", "5m", "cargo", "test", "--test", "integration"]
