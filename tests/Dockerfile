FROM --platform=linux ubuntu:22.04
ARG BUILDARCH

ENV GO_VERSION=1.18.2
ENV IGNITE_VERSION=0.25.1
ENV NODE_VERSION=18.x

ENV LOCAL=/usr/local
ENV GOROOT=$LOCAL/go
ENV HOME=/root
ENV GOPATH=$HOME/go
ENV PATH=$GOROOT/bin:$GOPATH/bin:$PATH

RUN mkdir -p $GOPATH/bin

ENV PACKAGES curl gcc jq git make
RUN apt-get update
RUN apt-get install -y $PACKAGES

RUN curl -L https://go.dev/dl/go${GO_VERSION}.linux-$BUILDARCH.tar.gz | tar -C $LOCAL -xzf -
RUN curl -L https://get.ignite.com/cli@v${IGNITE_VERSION}! | bash
RUN curl -fsSL https://deb.nodesource.com/setup_${NODE_VERSION} | bash -
RUN apt-get install -y nodejs

RUN git clone https://github.com/Team-Kujira/core.git /code
WORKDIR /code

RUN rm config.yml
COPY config.yml config.yml

EXPOSE 1317 26657
