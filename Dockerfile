# Use Alpine as a base image, because it's small.
FROM alpine:latest

# Run our webserver out of /app.
WORKDIR /app

# Configure our Rocket webserver.
ADD falconerid/Rocket.toml .

# Build target.
ARG MODE=debug

# Copy static executables into container.
ADD bin/${MODE}/falconerid bin/${MODE}/falconeri-worker /usr/local/bin/
