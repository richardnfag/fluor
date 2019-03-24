# Template Format

```dockerfile
FROM image:latest AS builder

WORKDIR /usr/src/function/
ADD source.tar.gz /usr/src/function/

RUN ... # compile and optimize the binary or bytecode 


FROM scratch AS runtime # scratch or minimal runtime

COPY --from=builder /usr/src/function/function /usr/local/bin/

CMD [ "/usr/local/bin/function" ]

```