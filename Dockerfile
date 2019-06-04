# Use Alpine as a base image, because it's small. We need `edge` to get
# `aws-cli`.
FROM alpine:edge

# Install `gsutil`. Taken from
# https://github.com/GoogleCloudPlatform/cloud-sdk-docker/blob/master/alpine/Dockerfile.
ARG CLOUD_SDK_VERSION=248.0.0
ENV CLOUD_SDK_VERSION=$CLOUD_SDK_VERSION
ENV PATH /google-cloud-sdk/bin:$PATH
RUN apk --no-cache --update add \
        curl \
        python \
        py-crcmod \
        bash \
        libc6-compat \
        openssh-client \
        git \
        gnupg \
    && curl -O https://dl.google.com/dl/cloudsdk/channels/rapid/downloads/google-cloud-sdk-${CLOUD_SDK_VERSION}-linux-x86_64.tar.gz && \
    tar xzf google-cloud-sdk-${CLOUD_SDK_VERSION}-linux-x86_64.tar.gz && \
    rm google-cloud-sdk-${CLOUD_SDK_VERSION}-linux-x86_64.tar.gz && \
    ln -s /lib /lib64 && \
    gcloud config set core/disable_usage_reporting true && \
    gcloud config set component_manager/disable_update_check true && \
    gcloud config set metrics/environment github_docker_image && \
    gcloud --version
VOLUME ["/root/.config"]

# Install `awscli`.
RUN echo http://dl-cdn.alpinelinux.org/alpine/edge/testing/ >> /etc/apk/repositories && \
    apk --no-cache --update add aws-cli

# Install `kubectl`.
ADD https://storage.googleapis.com/kubernetes-release/release/v1.13.4/bin/linux/amd64/kubectl /usr/local/bin/kubectl
RUN chmod +x /usr/local/bin/kubectl

# Run our webserver out of /app.
WORKDIR /app

# Configure our Rocket webserver.
ADD falconerid/Rocket.toml .

# Build target.
ARG MODE=debug

# Copy static executables into container.
ADD bin/${MODE}/falconerid bin/${MODE}/falconeri-worker /usr/local/bin/
