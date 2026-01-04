# Creating Docker images

In order to transform your data, you will need to create a Docker image.

## Inputs and outputs

If your pipeline JSON contains the following input section:

```json
"input": {
  "atom": {
    "URI": "gs://example-bucket/books/",
    "repo": "books",
    "glob": "/*"
  }
}
```

...you will find one or more input files from your bucket in the directory `/pfs/books`. You should place your input files in `/pfs/out`, using output names that are unique across all workers.

## Required executables

Your Docker image must contain `falconeri-worker` somewhere in your `$PATH`. You can install `falconeri-worker` by downloading the latest [release][] and copying `falconeri-worker` to `/usr/local/bin`, or another directory in your `$PATH`. This is a statically-linked Linux binary, so it should work on any reasonably modern Linux distro.

Note: `falconeri-worker` now uses the native Google Cloud Storage SDK for Rust, so there is no need to install `gsutil` or the Google Cloud SDK. Authentication is handled via the `GCLOUD_SERVICE_ACCOUNT_KEY` environment variable or Application Default Credentials.

TODO: Add example of installing `falconeri-worker`.

[release]: https://github.com/faradayio/falconeri/
