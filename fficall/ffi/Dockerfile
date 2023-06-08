FROM golang
RUN wget https://www.musl-libc.org/releases/musl-1.2.4.tar.gz && \
   tar -xvf musl-1.2.4.tar.gz && \
   cd musl-1.2.4 && \
   ./configure --enable-static --disable-shared && \
   make && make install

# GOOS=linux GOARCH=amd64 CGO_ENABLED=1 CC=musl-gcc GOOS=linux GOARCH=amd64 go build -tags lib --ldflags '-s -w -linkmode external -extldflags -static'  -buildmode=c-archive -o libfficall.a