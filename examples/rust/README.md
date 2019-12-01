
# Rust function example
```sh
# Running the Fluor Server
cargo run --release

cd examples/rust

# Creating a compressed archive from project directory
# -- "hello/" is a project directory
tar -czvf source.tar.gz -C hello/ .

# Creating a new function (hello-rust)

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


# Invoking the function

curl -X GET "http://localhost:8000/hello-rust/"

# Deleting the function

curl -X DELETE -H "Content-Type:application/json" -d "$JSON" "http://localhost:8000/function/"

```