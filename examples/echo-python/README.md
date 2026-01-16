# Python Wasm Example (Component Model)

This example demonstrates a Python function compatible with Fluor's Wasm Component Model interface.

## Source Code (`app.py`)

```python
class Function:
    def handle(self, input: str) -> str:
        return f"Python Echo Component: {input}"
```

## Building

To run this on Fluor, you must compile it into a Wasm Component. This requires [componentize-py](https://github.com/bytecodealliance/componentize-py).

1.  **Install Tool**: `pip install componentize-py`
2.  **Build**:
    ```bash
    componentize-py -d ../../wit -w function componentize app -o echo_python.wasm
    ```
3.  **Deploy**: Place `echo_python.wasm` in the target directory (e.g., `examples/echo-python/`).

## API Integration

The Fluor API is pre-configured to look for `examples/echo-python/echo_python.wasm` if you create a function named `python-echo`.
