FROM debian:bookworm-20240926@sha256:27586f4609433f2f49a9157405b473c62c3cb28a581c413393975b4e8496d0ab

RUN apt-get update \
 && apt-get install -y \
    sshpass \
 && rm -rf /var/lib/apt/lists/*