# GCS Integration Test

End-to-end test for Falconeri with Google Cloud Storage.

## Prerequisites

- A GCS bucket with read/write permissions
- A Google Cloud service account JSON key file
- `minikube`, `gsutil`, `kubectl`, `cargo`, `docker` in your PATH

## Setup

Set required environment variables:

```bash
export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
export GCS_BUCKET=your-bucket-name  # or gs://your-bucket-name
```

The test script will automatically:
- Start minikube if not already running (and delete it when done)
- Configure kubectl context
- Set up Docker environment variables

## Running the test

```bash
just run
# or
python3 test_gcs.py
```

## What the test does

1. **Setup**: Uploads `sample.txt` to `gs://$GCS_BUCKET/falconeri-test/input/`
2. **Build**: Builds falconeri-worker and the test Docker image
3. **Run**: Starts falconeri proxy and runs a job that processes the input file
4. **Verify**: Checks that output file was created with word frequencies
5. **Cleanup**: Deletes test files from GCS and removes k8s secret

The test reuses the `word-frequencies.sh` script from the sibling example directory, which counts word frequencies in text files.

## Expected output

On success, you should see:

```
✓ GCS integration test PASSED
```

The test will create a word frequency count file at `gs://$GCS_BUCKET/falconeri-test/output/sample.txt`.

