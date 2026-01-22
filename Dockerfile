# Single stage build for testing
# Note: No platform specified - uses native architecture of build environment (minikube VM)
FROM ubuntu:25.10

# Install Rust, build dependencies, and runtime utilities
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    libssl-dev \
    pkg-config \
    libpq-dev \
    bash \
    openssh-client \
    git \
    gnupg \
    ca-certificates \
    unzip \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    && curl "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip" -o "awscliv2.zip" \
    && unzip awscliv2.zip \
    && ./aws/install \
    && rm -rf awscliv2.zip aws \
    && rm -rf /var/lib/apt/lists/*

# Add Rust to PATH
ENV PATH="/root/.cargo/bin:${PATH}"

# Install kubectl
ARG KUBERNETES_VERSION=1.13.4
ARG TARGETARCH=amd64
ENV KUBERNETES_VERSION=$KUBERNETES_VERSION
ADD https://storage.googleapis.com/kubernetes-release/release/v${KUBERNETES_VERSION}/bin/linux/${TARGETARCH}/kubectl /usr/local/bin/kubectl
RUN chmod +x /usr/local/bin/kubectl

# Set working directory
WORKDIR /build

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY falconeri ./falconeri
COPY falconerid ./falconerid
COPY falconeri-worker ./falconeri-worker
COPY falconeri_common ./falconeri_common

# Build target
ARG MODE=debug

# Build the binaries
RUN if [ "$MODE" = "release" ]; then \
        cargo build --release --bin falconerid --bin falconeri-worker && \
        cp target/release/falconerid target/release/falconeri-worker /usr/local/bin/; \
    else \
        cargo build --bin falconerid --bin falconeri-worker && \
        cp target/debug/falconerid target/debug/falconeri-worker /usr/local/bin/; \
    fi

# Run our webserver out of /app
WORKDIR /app

# Configure our Rocket webserver
ADD falconerid/Rocket.toml .
