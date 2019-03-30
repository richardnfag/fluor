# Fluor Functions


Fluor Functions is a serverless platform written in Rust.

## Architecture

![Architecture Diagram](docs/images/architecture.png)

## Requirements

- [Docker CE](https://docs.docker.com/install/)

## Examples

[Hello World example in Rust](examples/rust)


```sh
# run Fluor Service
cargo run

cd examples/rust

# create new function (hello-rust)
bash hello.sh new

# invoke function
bash hello.sh run
```

[Show all examples](examples)

# TODO
- [ ] Test all
- [ ] Document
- [ ] Optimizations
- [ ] Create CLI
- [ ] Create Web UI
- [ ] Templates for more programming languages
- [ ] Support for Windows
- [ ] Support others runtimes (LXC, NVIDIA-Docker,...)

## Contributions
Contributions in the form of bug reports, feature requests, or pull requests are welcome. 

## License

Fluor Functions is licensed under the [MIT License](LICENSE)
