# Use Alpine as a base image, because it's small.
FROM alpine:latest

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
