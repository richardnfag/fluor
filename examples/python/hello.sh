#!/bin/bash

JSON=$(cat <<EOF
{
    "name": "hello-python",
    "language": "python",
    "source": "$(base64 -w 0 source.tar.gz)",
    "method": "GET",
    "path": "/hello-python/",
    "cpu": "2",
    "memory": "1024m",
    "uptime": "30"
}
EOF
)

if [ "$1" == "new" ]; then
	curl -X POST -H "Content-Type:application/json" -d "$JSON" "http://localhost:8000/function/"

elif [ "$1" == "del" ]; then
    curl -X DELETE -H "Content-Type:application/json" -d "$JSON" "http://localhost:8000/function/"

elif [ "$1" == "run" ]; then
    curl -X GET "http://localhost:8000/hello-python/"
    
else
	echo "Usage: $0 [new | del | run]"
fi