#!/usr/bin/env python3
"""
End-to-end test for Falconeri with Google Cloud Storage.

Required environment variables:
- GOOGLE_APPLICATION_CREDENTIALS: Path to service account JSON file
- GCS_BUCKET: Name of existing GCS bucket (without gs:// prefix)

Usage:
    python3 test_gcs.py              # Reuse existing minikube profile
    python3 test_gcs.py --clean      # Delete and recreate minikube profile

Build Strategy:
- All cluster binaries (falconerid, falconeri-worker) are built inside minikube
  using 'minikube image build' to ensure architecture compatibility
- Only the CLI tools (falconeri proxy, job commands) run locally
- No cargo build commands are run on the host machine for cluster components
"""

import argparse
import json
import os
import secrets
import subprocess
import sys
import time
import tempfile
import urllib.request
import urllib.error
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent
PROJECT_ROOT = SCRIPT_DIR.parent.parent
MINIKUBE_PROFILE = "falconeri-gcs-test"


def run(cmd, check=True, **kwargs):
    """Run a command and print it."""
    print(f"$ {' '.join(cmd) if isinstance(cmd, list) else cmd}")
    return subprocess.run(cmd, check=check, **kwargs)


def setup_minikube():
    """Create a dedicated minikube profile for this test."""
    print(f"\n=== Setting up minikube profile: {MINIKUBE_PROFILE} ===")

    # Check if this profile already exists
    result = run(
        ["minikube", "profile", "list", "-o", "json"],
        capture_output=True,
        text=True,
        check=False,
    )

    profile_exists = False
    if result.returncode == 0:
        try:
            profiles = json.loads(result.stdout)
            if "valid" in profiles:
                for profile in profiles["valid"]:
                    if profile.get("Name") == MINIKUBE_PROFILE:
                        profile_exists = True
                        print(f"✓ Profile {MINIKUBE_PROFILE} already exists")
                        break
        except (json.JSONDecodeError, KeyError):
            pass

    if not profile_exists:
        print(f"Creating minikube profile {MINIKUBE_PROFILE}...")
        run(["minikube", "start", "-p", MINIKUBE_PROFILE])
        print(f"✓ minikube profile {MINIKUBE_PROFILE} started")
    else:
        # Check if it's running
        status_result = run(
            ["minikube", "status", "-p", MINIKUBE_PROFILE, "-o", "json"],
            capture_output=True,
            text=True,
            check=False,
        )

        is_running = False
        if status_result.returncode == 0:
            try:
                status = json.loads(status_result.stdout)
                is_running = status.get("Host") == "Running"
            except (json.JSONDecodeError, KeyError):
                pass

        if not is_running:
            print(f"Starting {MINIKUBE_PROFILE}...")
            run(["minikube", "start", "-p", MINIKUBE_PROFILE])
        else:
            print(f"✓ {MINIKUBE_PROFILE} is already running")

    # Switch kubectl context to our profile
    print("Setting kubectl context to test profile...")
    run(["kubectl", "config", "use-context", MINIKUBE_PROFILE])

    print("✓ minikube setup complete")


def validate_minikube():
    """Ensure we're not accidentally running against production k8s."""
    print("\n=== Validating kubectl context ===")

    # Check current kubectl context
    result = run(
        ["kubectl", "config", "current-context"],
        capture_output=True,
        text=True,
        check=False,
    )

    if result.returncode != 0:
        print("No kubectl context set - will be configured during minikube setup")
        return

    context = result.stdout.strip()

    # Warn if context looks like production (doesn't contain minikube or our test profile)
    if "minikube" not in context.lower() and MINIKUBE_PROFILE not in context:
        print(f"⚠️  WARNING: Current kubectl context is '{context}'")
        print(
            f"This test will create a new minikube profile '{MINIKUBE_PROFILE}' and switch to it."
        )
        response = input("Continue? (y/N): ")
        if response.lower() != "y":
            sys.exit(1)
    else:
        print(f"Current context: {context}")


def ensure_correct_context():
    """Ensure kubectl is using the correct test context."""
    result = run(
        ["kubectl", "config", "current-context"],
        capture_output=True,
        text=True,
        check=False,
    )

    if result.returncode != 0:
        print(f"⚠️  No kubectl context set! Setting to {MINIKUBE_PROFILE}")
        run(["kubectl", "config", "use-context", MINIKUBE_PROFILE])
        return

    current_context = result.stdout.strip()
    if current_context != MINIKUBE_PROFILE:
        print(f"⚠️  Context is '{current_context}', switching to '{MINIKUBE_PROFILE}'")
        run(["kubectl", "config", "use-context", MINIKUBE_PROFILE])
    else:
        print(f"✓ Using correct context: {MINIKUBE_PROFILE}")


def validate_env():
    """Validate required environment variables."""
    creds = os.getenv("GOOGLE_APPLICATION_CREDENTIALS")
    bucket = os.getenv("GCS_BUCKET")

    if not creds:
        print("Error: GOOGLE_APPLICATION_CREDENTIALS environment variable not set")
        sys.exit(1)

    if not bucket:
        print("Error: GCS_BUCKET environment variable not set")
        sys.exit(1)

    if not Path(creds).exists():
        print(f"Error: Service account file not found: {creds}")
        sys.exit(1)

    # Strip gs:// prefix if provided
    bucket = bucket.removeprefix("gs://")

    return creds, bucket


def upload_test_data(bucket, gcs_prefix):
    """Upload sample.txt to GCS."""
    sample_file = SCRIPT_DIR / "sample.txt"
    gcs_uri = f"gs://{bucket}/{gcs_prefix}/input/sample.txt"

    print(f"\n=== Uploading test data to {gcs_uri} ===")
    run(["gsutil", "cp", str(sample_file), gcs_uri])


def kill_kubectl_port_forwards():
    """Kill any kubectl port-forward processes."""
    print("Killing kubectl port-forward processes...")
    result = run(
        ["pgrep", "-f", "kubectl port-forward"],
        capture_output=True,
        text=True,
        check=False,
    )

    if result.stdout.strip():
        pids = result.stdout.strip().split("\n")
        for pid in pids:
            if pid:
                print(f"  Killing kubectl port-forward process {pid}")
                run(["kill", pid], check=False)
        time.sleep(1)
    else:
        print("  No kubectl port-forward processes found")


def cleanup_previous_run():
    """Clean up resources from any previous test run."""
    print("\n=== Cleaning up resources from previous runs ===")
    ensure_correct_context()

    kill_kubectl_port_forwards()

    # Delete all gcs-test jobs (pods will be cleaned up automatically)
    print("Deleting old gcs-test jobs...")
    result = run(
        ["kubectl", "get", "jobs", "-o", "name"],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.stdout:
        job_names = [
            line.strip()
            for line in result.stdout.strip().split("\n")
            if line and "gcs-test" in line
        ]
        if job_names:
            run(["kubectl", "delete"] + job_names + ["--wait=false"], check=False)
            print(f"  Deleted {len(job_names)} old gcs-test jobs")
        else:
            print("  No old gcs-test jobs found")
    else:
        print("  No old gcs-test jobs found")

    # Delete falconerid deployment
    print("Deleting old falconerid deployment...")
    result = run(
        ["kubectl", "delete", "deployment", "falconerid", "--wait=false"],
        check=False,
        capture_output=True,
    )
    if result.returncode == 0:
        print("  Deleted falconerid deployment")
    else:
        print("  No falconerid deployment found")

    # Delete old replica sets
    print("Deleting old replica sets...")
    run(
        ["kubectl", "delete", "replicaset", "-l", "app=falconerid", "--wait=false"],
        check=False,
        capture_output=True,
    )

    # Delete old secrets (they'll be recreated)
    print("Deleting old secrets...")
    run(
        ["kubectl", "delete", "secret", "gcs", "--wait=false"],
        check=False,
        capture_output=True,
    )
    run(
        ["kubectl", "delete", "secret", "gcs-credentials", "--wait=false"],
        check=False,
        capture_output=True,
    )

    # Wait a moment for deletions to start
    print("Waiting for resources to be deleted...")
    time.sleep(3)

    print("✓ Cleanup complete")


def build_falconeri_image():
    """Build the falconeri Docker image directly in minikube."""
    print("\n=== Building falconeri Docker image ===")

    # Use a local image name to avoid conflicts with Docker Hub
    image_name = "falconeri-local:test"

    # Build Docker image directly in minikube (Dockerfile will build binaries inside Docker)
    run(
        [
            "minikube",
            "image",
            "build",
            "-t",
            image_name,
            "-f",
            "Dockerfile",
            ".",
            "-p",
            MINIKUBE_PROFILE,
        ],
        cwd=PROJECT_ROOT,
    )

    print(f"✓ Built {image_name} in minikube")

    return image_name


def deploy_falconeri(image_name):
    """Deploy falconeri infrastructure to the cluster."""
    print("\n=== Deploying falconeri infrastructure ===")
    ensure_correct_context()

    run(
        ["cargo", "run", "-p", "falconeri", "--", "deploy"],
        cwd=PROJECT_ROOT,
    )

    # Patch the deployment to use our custom local image, set imagePullPolicy, and scale to 1 replica
    print(
        f"Updating deployment to use {image_name} with imagePullPolicy: Never and 1 replica..."
    )
    run(
        [
            "kubectl",
            "patch",
            "deployment/falconerid",
            "--type",
            "json",
            "-p",
            f'[{{"op": "replace", "path": "/spec/template/spec/containers/0/image", "value": "{image_name}"}}, {{"op": "replace", "path": "/spec/template/spec/containers/0/imagePullPolicy", "value": "Never"}}, {{"op": "replace", "path": "/spec/replicas", "value": 1}}]',
        ]
    )

    # Wait a moment for the rollout to start
    print("Waiting for rollout to start...")
    time.sleep(2)

    # Delete old replica sets to force clean rollout
    print("Cleaning up old replica sets...")
    result = run(
        ["kubectl", "get", "replicaset", "-l", "app=falconerid", "-o", "name"],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.stdout:
        for rs in result.stdout.strip().split("\n"):
            if rs:
                # Get the deployment revision to keep only the latest
                result = run(
                    ["kubectl", "get", rs, "-o", "jsonpath='{.spec.replicas}'"],
                    capture_output=True,
                    text=True,
                    check=False,
                )
                # Delete replica sets with 0 desired replicas
                replicas = result.stdout.strip().strip("'")
                if replicas == "0":
                    print(f"  Deleting old replica set: {rs}")
                    run(["kubectl", "delete", rs], check=False)
                else:
                    print(f"  Keeping active replica set: {rs} (replicas={replicas})")

    print("Waiting for falconeri deployments to be ready...")
    run(
        [
            "kubectl",
            "wait",
            "--for=condition=available",
            "deployment/falconerid",
            "--timeout=300s",
        ]
    )
    run(
        [
            "kubectl",
            "wait",
            "--for=condition=available",
            "deployment/falconeri-postgres",
            "--timeout=300s",
        ]
    )
    print("✓ Falconeri infrastructure is ready")


def create_k8s_secret(creds_path):
    """Create Kubernetes secret from service account JSON."""
    print("\n=== Creating Kubernetes secret ===")
    ensure_correct_context()

    # Read the service account JSON
    with open(creds_path) as f:
        service_account_json = f.read()

    # Create secret with GCLOUD_SERVICE_ACCOUNT_KEY
    # Use gcs-credentials to match what falconerid deployment expects
    run(
        [
            "kubectl",
            "create",
            "secret",
            "generic",
            "gcs-credentials",
            f"--from-literal=GCLOUD_SERVICE_ACCOUNT_KEY={service_account_json}",
        ]
    )


def build_worker():
    """No-op: worker is now built inside Docker."""
    pass


def build_docker_image(bucket):
    """Build the test Docker image directly in minikube."""
    print("\n=== Building test Docker image (with Rust compilation) ===")

    # Copy word-frequencies.sh to project root temporarily for Docker context
    script_src = SCRIPT_DIR.parent / "word-frequencies" / "word-frequencies.sh"
    script_dst = PROJECT_ROOT / "word-frequencies.sh"
    run(["cp", str(script_src), str(script_dst)])

    # Build Docker image from project root using Dockerfile in gcs-test directory
    # Use relative path from PROJECT_ROOT to the Dockerfile
    dockerfile_rel_path = "examples/gcs-test/Dockerfile"
    run(
        [
            "minikube",
            "image",
            "build",
            "-f",
            dockerfile_rel_path,
            "-t",
            "gcs-test",
            ".",
            "-p",
            MINIKUBE_PROFILE,
        ],
        cwd=PROJECT_ROOT,
    )

    # Clean up copied file
    script_dst.unlink()


def verify_images():
    """Verify that images contain expected binaries in correct format."""
    print("\n=== Verifying Docker images ===")

    def check_file_in_image(image_name, file_path, expected_type):
        """Check that a file exists in the image and is of the expected type."""
        print(f"Checking {file_path} in {image_name}...")

        result = run(
            [
                "minikube",
                "-p",
                MINIKUBE_PROFILE,
                "ssh",
                "--",
                f"docker run --rm {image_name} file {file_path}",
            ],
            capture_output=True,
            text=True,
            check=False,
        )

        if result.returncode != 0:
            print(f"  ❌ Could not check {file_path}")
            return False

        output = result.stdout.strip()
        if expected_type.lower() in output.lower():
            print(f"  ✓ {file_path} is a {expected_type}")
            return True
        else:
            print(f"  ❌ Expected {expected_type}, got: {output}")
            return False

    # Check falconeri-local:test image
    print("\nVerifying falconeri-local:test:")
    falconeri_checks = [
        check_file_in_image("falconeri-local:test", "/usr/local/bin/falconerid", "ELF"),
        check_file_in_image(
            "falconeri-local:test", "/usr/local/bin/falconeri-worker", "ELF"
        ),
        check_file_in_image("falconeri-local:test", "/usr/local/bin/kubectl", "ELF"),
    ]

    # Check gcs-test image
    print("\nVerifying gcs-test:")
    gcs_test_checks = [
        check_file_in_image("gcs-test", "/usr/local/bin/falconeri-worker", "ELF"),
        check_file_in_image(
            "gcs-test", "/usr/local/bin/word-frequencies.sh", "shell script"
        ),
    ]

    if all(falconeri_checks + gcs_test_checks):
        print("\n✓ All image verifications passed")
    else:
        print("\n⚠️  Some image verifications failed, but continuing...")
        print("    (This may be expected if architecture differs)")


def generate_pipeline_spec(bucket, gcs_prefix):
    """Generate the pipeline spec JSON."""
    spec = {
        "pipeline": {"name": "gcs_test"},
        "transform": {
            "image": "gcs-test",
            "image_pull_policy": "Never",
            "cmd": ["word-frequencies.sh"],
            "env": {"RUST_LOG": "falconeri_common=debug,falconeri_worker=debug,info"},
            "secrets": [
                {
                    "name": "gcs-credentials",
                    "key": "GCLOUD_SERVICE_ACCOUNT_KEY",
                    "env_var": "GCLOUD_SERVICE_ACCOUNT_KEY",
                }
            ],
        },
        "parallelism_spec": {"constant": 2},
        "resource_requests": {"memory": "128Mi", "cpu": 0.1},
        "datum_tries": 3,
        "input": {
            "atom": {
                "repo": "input",
                "URI": f"gs://{bucket}/{gcs_prefix}/input/",
                "glob": "/*",
            }
        },
        "egress": {"URI": f"gs://{bucket}/{gcs_prefix}/output/"},
    }

    spec_path = SCRIPT_DIR / "gcs-test.json"
    with open(spec_path, "w") as f:
        json.dump(spec, f, indent=2)

    print(f"\n=== Generated pipeline spec at {spec_path} ===")
    return spec_path


def start_proxy():
    """Start falconeri proxy in background."""
    print("\n=== Starting falconeri proxy ===")
    print("$ cargo run -p falconeri -- proxy")
    proxy_proc = subprocess.Popen(
        ["cargo", "run", "-p", "falconeri", "--", "proxy"],
        cwd=PROJECT_ROOT,
    )

    # Wait for proxy to be ready by polling the version endpoint
    print("Waiting for proxy to be ready...")

    for i in range(60):
        time.sleep(1)
        try:
            urllib.request.urlopen("http://localhost:8089/version", timeout=1)
            print("✓ Proxy is ready")
            return proxy_proc
        except Exception:
            if proxy_proc.poll() is not None:
                raise Exception(f"Proxy exited with code {proxy_proc.returncode}")
            continue

    raise Exception("Proxy did not become ready within 60 seconds")


def run_job(spec_path):
    """Run the falconeri job and return job ID."""
    print("\n=== Running falconeri job ===")
    ensure_correct_context()

    result = run(
        ["cargo", "run", "-p", "falconeri", "--", "job", "run", str(spec_path)],
        cwd=PROJECT_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )

    # Print stdout and stderr
    if result.stdout:
        print(result.stdout)
    if result.stderr:
        print(result.stderr, file=sys.stderr)

    # Check if command failed
    if result.returncode != 0:
        raise subprocess.CalledProcessError(
            result.returncode, result.args, result.stdout, result.stderr
        )

    # Look for job name in output (it's just the job name on a single line)
    for line in result.stdout.split("\n"):
        line = line.strip()
        # Job names are alphanumeric with dashes, matching pattern: ^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$
        if (
            line
            and not line.startswith("Handling")
            and not line.startswith("Finished")
            and not line.startswith("Running")
        ):
            # This should be the job name
            if line and all(c.isalnum() or c == "-" for c in line):
                return line

    print("Warning: Could not extract job ID from output")
    return None


def wait_for_job(job_id, timeout=300):
    """Wait for job to complete, polling status."""
    if not job_id:
        print("No job ID provided, skipping wait")
        return False

    print(f"\n=== Waiting for job {job_id} to complete ===")
    ensure_correct_context()
    start_time = time.time()

    while time.time() - start_time < timeout:
        result = run(
            ["cargo", "run", "-p", "falconeri", "--", "job", "describe", job_id],
            cwd=PROJECT_ROOT,
            capture_output=True,
            text=True,
            check=False,
        )

        # Check for completion indicators
        if "Status: done" in result.stdout:
            print(f"Job {job_id} completed successfully!")
            return True
        elif "error" in result.stdout.lower():
            print(f"Job {job_id} failed!")
            if result.stdout:
                print(result.stdout)
            if result.stderr:
                print(result.stderr, file=sys.stderr)
            return False

        print(f"Job still running... ({int(time.time() - start_time)}s elapsed)")
        time.sleep(10)

    print(f"Job did not complete within {timeout}s")
    return False


def verify_output(bucket, gcs_prefix):
    """Verify that output file was created and contains expected data."""
    print("\n=== Verifying output ===")

    output_uri = f"gs://{bucket}/{gcs_prefix}/output/sample.txt"

    # Check if file exists
    result = run(
        ["gsutil", "ls", output_uri], check=False, capture_output=True, text=True
    )

    if result.returncode != 0:
        print(f"Error: Output file not found at {output_uri}")
        if result.stderr:
            print(result.stderr, file=sys.stderr)
        return False

    print(f"Output file exists at {output_uri}")

    # Download and check contents
    with tempfile.NamedTemporaryFile(mode="w+", delete=False) as tmp:
        tmp_path = tmp.name

    # Download to temp file
    run(["gsutil", "cp", output_uri, tmp_path])

    # Read contents
    with open(tmp_path) as f:
        contents = f.read()

    # Clean up temp file
    os.unlink(tmp_path)

    print("\nOutput contents:")
    print(contents[:500])  # Print first 500 chars

    # Basic validation - should contain word frequency counts
    if not contents.strip():
        print("Error: Output file is empty")
        return False

    # Should have lines with counts and words
    lines = contents.strip().split("\n")
    if len(lines) < 5:
        print("Error: Output has too few lines")
        return False

    print(f"\nSuccess! Output contains {len(lines)} lines of word frequencies")
    return True


def cleanup(bucket, gcs_prefix, proxy_proc):
    """Clean up test resources."""
    print("\n=== Cleaning up ===")

    # Stop proxy
    if proxy_proc:
        print("Stopping proxy...")
        proxy_proc.terminate()
        try:
            proxy_proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proxy_proc.kill()

    # Kill any kubectl port-forward processes (in case proxy didn't clean them up)
    kill_kubectl_port_forwards()

    # Delete GCS test files
    print("Deleting GCS test files...")
    run(["gsutil", "-m", "rm", "-r", f"gs://{bucket}/{gcs_prefix}/"], check=False)

    # Delete k8s secrets
    print("Deleting k8s secrets...")
    ensure_correct_context()
    run(["kubectl", "delete", "secret", "gcs"], check=False)
    run(["kubectl", "delete", "secret", "gcs-credentials"], check=False)

    # Delete generated spec
    spec_path = SCRIPT_DIR / "gcs-test.json"
    if spec_path.exists():
        spec_path.unlink()


def main():
    """Main test orchestration."""
    parser = argparse.ArgumentParser(
        description="End-to-end test for Falconeri with Google Cloud Storage"
    )
    parser.add_argument(
        "--clean",
        action="store_true",
        help="Delete the minikube profile before starting for a clean slate",
    )
    args = parser.parse_args()

    print("=== Falconeri GCS Integration Test ===\n")

    # Validate environment first
    creds_path, bucket = validate_env()
    print(f"Using GCS bucket: {bucket}")
    print(f"Using credentials: {creds_path}")

    # Generate random GCS prefix for this test run
    gcs_prefix = f"falconeri-test-{secrets.token_hex(6)}"
    print(f"Using GCS prefix: {gcs_prefix}")

    # SAFETY: Validate current kubectl context
    validate_minikube()

    # Delete minikube profile if --clean flag is set
    if args.clean:
        print(f"\n🧹 Deleting minikube profile {MINIKUBE_PROFILE} for clean slate...")
        run(["minikube", "delete", "-p", MINIKUBE_PROFILE], check=False)

    # Setup dedicated minikube profile
    setup_minikube()
    print(f"\n⚠️  Using dedicated minikube profile '{MINIKUBE_PROFILE}'")

    # Clean up any resources from previous runs
    cleanup_previous_run()

    proxy_proc = None
    success = False

    try:
        # Setup phase
        upload_test_data(bucket, gcs_prefix)
        falconeri_image = build_falconeri_image()
        create_k8s_secret(creds_path)  # Create secret BEFORE deploying
        deploy_falconeri(falconeri_image)

        # Build phase
        build_worker()
        build_docker_image(bucket)

        # Verify images contain correct binaries
        verify_images()

        # Run phase
        spec_path = generate_pipeline_spec(bucket, gcs_prefix)
        proxy_proc = start_proxy()
        job_id = run_job(spec_path)

        # Wait for completion
        job_success = wait_for_job(job_id)

        # Verify phase
        if job_success:
            success = verify_output(bucket, gcs_prefix)
    finally:
        # Always cleanup
        cleanup(bucket, gcs_prefix, proxy_proc)

    # Report results
    print("\n" + "=" * 50)
    if success:
        print("✓ GCS integration test PASSED")
        sys.exit(0)
    else:
        print("✗ GCS integration test FAILED")
        sys.exit(1)


if __name__ == "__main__":
    main()
