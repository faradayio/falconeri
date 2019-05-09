# Local testing with minikube

```sh
minikube start
kubectl config use-context minikube
eval (minikube docker-env)
just set-version 0.x.y-alpha.z
just image
cargo run -p falconeri -- deploy --development`
``
