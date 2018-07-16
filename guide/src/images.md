# Docker image requirements

If your pipeline JSON contained the following input:

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

Your Docker image must contain both `gsutil` (assuming you're using Google Cloud Storage) and `falconeri-worker` somewhere in your `$PATH`. You can install `gsutil` on an Ubuntu image as follows:

```Dockerfile
RUN export CLOUD_SDK_REPO="cloud-sdk-$(lsb_release -c -s)" && \
    echo "deb http://packages.cloud.google.com/apt $CLOUD_SDK_REPO main" | \
        tee -a /etc/apt/sources.list.d/google-cloud-sdk.list && \
    curl https://packages.cloud.google.com/apt/doc/apt-key.gpg | \
        apt-key add - && \
    apt-get update && apt-get install -y google-cloud-sdk && \
    apt-get clean && rm -rf /var/lib/apt/lists/*
```

You can install `falconeri-worker` by downloading the latest [release][] and copying `falconeri-worker` to `/usr/local/bin`, or another directory in your `$PATH`. This is a statically-linked Linux binary, so it should work on any reasonably modern Linux distro.

TODO: Add example of installing `falconeri-worker`.

[release]: https://github.com/faradayio/falconeri/
