# Build Verification for GCS Test

## Build Strategy

All cluster components (falconerid and falconeri-worker) are built **inside minikube** to ensure architecture compatibility with the Kubernetes cluster. Nothing is built on the host machine except the CLI tools that need to run locally.

### What is Built in Minikube

1. **falconeri-local:test** image
   - Contains: falconerid, falconeri-worker, kubectl
   - Built with: `minikube image build -t falconeri-local:test`
   - Dockerfile: `/Dockerfile`

2. **gcs-test** image
   - Contains: falconeri-worker, word-frequencies.sh
   - Built with: `minikube image build -t gcs-test`
   - Dockerfile: `examples/gcs-test/Dockerfile`

### What Runs Locally

Only CLI tools that coordinate the test:
- `cargo run -p falconeri -- deploy` - Deploy infrastructure
- `cargo run -p falconeri -- proxy` - Run local proxy for API access
- `cargo run -p falconeri -- job run` - Submit job to cluster
- `cargo run -p falconeri -- job describe` - Query job status

## Architecture Compatibility

The Dockerfiles **do not specify a platform**, allowing Docker to build for the native architecture of the build environment (the minikube VM). This ensures:
- No cross-compilation issues
- Binaries match the cluster architecture
- No dependency on host machine architecture

## Verification

The test script now includes automatic verification that checks each built image contains the expected files in the correct format:

### falconeri-local:test
- `/usr/local/bin/falconerid` → ELF executable
- `/usr/local/bin/falconeri-worker` → ELF executable
- `/usr/local/bin/kubectl` → ELF executable

### gcs-test
- `/usr/local/bin/falconeri-worker` → ELF executable
- `/usr/local/bin/word-frequencies.sh` → shell script

The verification runs after building images and before running the job, using:
```bash
minikube -p falconeri-gcs-test ssh -- docker run --rm <image> file <path>
```

This ensures binaries are correctly formatted and executable within the minikube cluster environment.

