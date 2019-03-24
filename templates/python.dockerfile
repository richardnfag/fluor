FROM python:3-alpine AS builder

WORKDIR /usr/src/function/
ADD source.tar.gz /usr/src/function/
RUN python3 -m compileall -f -b /usr/src/function/


FROM python:3-alpine AS runtime

COPY --from=builder /usr/src/function/*.pyc /usr/local/bin/function/
RUN chmod +x /usr/local/bin/function/*.pyc
ENTRYPOINT [ "python", "/usr/local/bin/function/main.pyc" ]