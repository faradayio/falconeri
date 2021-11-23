# `falconeri`: Run batch data-processing jobs on Kubernetes

Falconeri runs on a pre-existing Kubernetes cluster, and it allows you to use Docker images to transform large data files stored in cloud buckets.

For detailed instructions, see the [Falconeri guide][guide].

Setup is simple:

```sh
falconeri deploy
falconeri proxy
falconeri migrate
```

Running is similarly simple:

```sh
falconeri job run my-job.json
```

[guide]: https://github.com/faradayio/falconeri/blob/master/guide/src/SUMMARY.md

## Contributing to `falconeri`

First, you'll need to set up some development tools:

```sh
cargo install just
cargo install cargo-deny
cargo install cargo-edit
```

Next, check out the available tasks in the `justfile`:

```sh
just --list
```

For local development, you'll want to install [`minikube`](https://minikube.sigs.k8s.io/docs/start/). Start it as follows, and point your local Docker at it:

```sh
minikube start
eval $(minikube docker-env)
```

Then build an image. **You must have `docker-env` set up as above** if you want to test this image.

```sh
just image
```

Now you can deploy a development version of `falconeri` to `minikube`:

```sh
cargo run -p falconeri -- deploy --development
```

Check to see if your cluster comes up:

```sh
kubectl get all

# Or if you have `watch`, try:
watch -n 5 kubectl get all
```

### Running the example program

Running the example program is necessary to make sure `falconeri` works. First, run:

```sh
cd examples/word-frequencies
```

Next, you'll need to set up an S3 bucket. If you're **at Faraday,** run:

```sh
# Faraday only!
just secret
```

If you're **not a Faraday**, create an S3 bucket, and place a `*.txt` file in `$MY_BUCKET/texts/`. Then, set up an AWS access key with read/write access to the bucket, and save the key pair in files named `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY`. Then run:

```sh
# Not for Faraday!
kubectl create secret generic s3 \
    --from-file=AWS_ACCESS_KEY_ID \
    --from-file=AWS_SECRET_ACCESS_KEY
```

Then edit `word-frequencies.json` to point at your bucket.

Now you can build the worker image using:

```sh
# This assumes you previously ran `just image` in the top-level directory.
just image
```

In another terminal, start a `falconeri proxy` command:

```sh
just proxy
```

In the original terminal, start the job:

```sh
just run
```

From here, you can use `falconeri job describe $ID` and `kubectl` normally. See the [guide][] for more details.

### Releasing a new `falconeri`

For now, this process should only be done by Eric, because there are some semver issues that we haven't fully thought out yet.

First, edit the `CHANGELOG.md` file to describe the release. Next, bump the version:

```sh
just set-version $MY_NEW_VERSION
```

Commit your changes with a subject like:

```sh
$MY_NEW_VERSION: Short description
```

You should be able to make a release by running:

```sh
just MODE=release release
```

### REST API

Note that `falconerid` has a complete REST API, and you don't actually need to use the `falconeri` command-line tool during normal operations. This _is_ used internally at Faraday.
