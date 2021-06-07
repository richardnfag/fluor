<div align="center">
  <h1>Fluor Functions</h1>
  <p>
    <strong>Fluor Functions is a experimental serverless platform written in Rust.</strong>
  </p>
  <p>

[![CI](https://github.com/richardnas/fluor/workflows/CI/badge.svg)](https://github.com/richardnas/fluor/actions?query=workflow%3ACI)
[![MIT licensed](https://img.shields.io/github/license/richardnas/fluor?color=%23000c0c)](LICENSE)

  </p>
</div>


## Setup

### Requirements

- [Docker CE](https://docs.docker.com/install/)


## Running

### Follow the examples
```sh
# Run the Fluor Server
cargo run --release > /dev/null 2>&1 &

# Go to example directory
cd examples/rust
```

```sh
# Create a compressed archive from project directory
# -- "hello/" is a project directory
tar -czvf source.tar.gz -C hello/ .
```
```sh
# Create a new function (hello-rust)

JSON=$(cat <<EOF
{
    "name": "hello-rust",
    "language": "rust",
    "source": "$(base64 -w 0 source.tar.gz)",
    "method": "GET",
    "path": "/hello-rust/",
    "cpu": "2",
    "memory": "512m",
    "uptime": "30"
}
EOF
)

curl -X POST -H "Content-Type:application/json" -d "$JSON" "http://localhost:8000/function/"
```

```sh
# Invoke the function
curl -X GET "http://localhost:8000/hello-rust/"
```


```sh
# Delete the function
curl -X DELETE -H "Content-Type:application/json" -d "$JSON" "http://localhost:8000/function/"
```

```sh
# Stop the Fluor Server
killall -9 fluor
```


- [Show all examples](examples)



## Contributions
Contributions in the form of bug reports, feature requests, or pull requests are welcome. 

## License

Fluor Functions is licensed under the [MIT License](LICENSE)
