# Python function example
```sh
# Running the Fluor Server
cargo run --release

cd examples/python

# Creating a compressed archive from project directory
# -- "main.py" is a file function
tar -czvf source.tar.gz main.py

# Creating a new function (hello-python)

JSON=$(cat <<EOF
{
    "name": "hello-python",
    "language": "python",
    "source": "$(base64 -w 0 source.tar.gz)",
    "method": "GET",
    "path": "/hello-python/",
    "cpu": "2",
    "memory": "512m",
    "uptime": "30"
}
EOF
)

curl -X POST -H "Content-Type:application/json" -d "$JSON" "http://localhost:8000/function/"


# Invoking the function

curl -X GET "http://localhost:8000/hello-python/"

# Deleting the function

curl -X DELETE -H "Content-Type:application/json" -d "$JSON" "http://localhost:8000/function/"

```
