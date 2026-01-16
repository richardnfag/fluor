.PHONY: build clean

WIT_DIR = ../../wit
WORLD = function
APP = app
OUT = echo_python.wasm

build:
	componentize-py -d $(WIT_DIR) -w $(WORLD) componentize $(APP) -o $(OUT)

clean:
	rm -f $(OUT)
