# Contract V2 PoC

Minimal proof of concept for a typed Fluor function contract aimed at validating C# support with `componentize-dotnet`.

## Layout

- `wit/`: shared WIT contract
- `csharp-function/`: example C# component using the official `componentize-dotnet` flow
- `host/`: minimal Rust/Wasmtime host that will invoke the component directly

## Goal

Validate a contract shaped like an HTTP request/response interface:

- exported interface instead of a direct top-level function
- typed request/response records
- typed errors with `result`

## Contract

The PoC contract is defined in [wit/world.wit](/home/richard/Documents/Projects/fluor/poc/contract-v2/wit/world.wit:1):

```wit
package fluor:fun@0.2.0;

interface handler {
    record header {
        name: string,
        value: string,
    }

    record request {
        method: string,
        path: string,
        query: option<string>,
        headers: list<header>,
        body: string,
    }

    record response {
        status: u16,
        headers: list<header>,
        body: string,
    }

    enum error-code {
        bad-request,
        unauthorized,
        forbidden,
        not-found,
        conflict,
        internal,
    }

    record error {
        code: error-code,
        message: string,
    }

    handle: func(request: request) -> result<response, error>;
}

world function {
    export handler;
}
```

## Run

```bash
cd poc/contract-v2
just demo
```

This will:

- build the C# component with `componentize-dotnet`
- run the minimal Rust host
- invoke `POST /hello` with a small JSON body

Expected output shape:

```text
Ok(
    Response {
        status: 200,
        headers: [...],
        body: "{ ... }",
    },
)
```

## Notes

- The C# component uses the official `componentize-dotnet` flow and exports an interface named `handler`.
- The Rust host uses `wasmtime::component::bindgen!` directly and does not depend on Fluor's API/server layers.
- `CC=/usr/bin/gcc` is forced in the host command because this environment exposes a read-only `ccache` wrapper on `PATH`.
