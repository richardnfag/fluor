#!/bin/bash

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

if [ "$1" == "new" ]; then
	curl -X POST -H "Content-Type:application/json" -d "$JSON" "http://localhost:8000/function/"

elif [ "$1" == "del" ]; then
    curl -X DELETE -H "Content-Type:application/json" -d "$JSON" "http://localhost:8000/function/"

elif [ "$1" == "run" ]; then
    curl -X GET "http://localhost:8000/hello-rust/"

else
	echo "Usage: $0 [new | del | run]"
fi