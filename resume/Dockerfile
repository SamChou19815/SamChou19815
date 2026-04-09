FROM texlive/texlive:latest

RUN apt-get update && apt-get install -y \
    openssh-server \
    git \
    make \
    && rm -rf /var/lib/apt/lists/*

RUN mkdir -p /var/run/sshd

# Allow root login with key-based auth only
RUN sed -i 's/#PermitRootLogin prohibit-password/PermitRootLogin prohibit-password/' /etc/ssh/sshd_config \
    && sed -i 's/#PasswordAuthentication yes/PasswordAuthentication no/' /etc/ssh/sshd_config

# Copy host's SSH public key for passwordless auth
RUN mkdir -p /root/.ssh && chmod 700 /root/.ssh
COPY authorized_keys /root/.ssh/authorized_keys
RUN chmod 600 /root/.ssh/authorized_keys

WORKDIR /workspace

EXPOSE 22

CMD ["/usr/sbin/sshd", "-D"]
