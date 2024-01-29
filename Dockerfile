FROM ubuntu:latest

COPY ./target/release/ninja  /bin/ninja

ENV LANG=C.UTF-8 DEBIAN_FRONTEND=noninteractive LANG=zh_CN.UTF-8 LANGUAGE=zh_CN.UTF-8 LC_ALL=C
ENTRYPOINT ["/bin/ninja"]
