FROM ubuntu:20.04

RUN apt-get update && \
    apt-get install -y \
        ca-certificates \
        python3-pip \
        && \
    apt-get clean && rm -rf /var/lib/apt/lists/* && \
    pip3 install awscli --upgrade

ADD falconeri-worker word-frequencies.sh /usr/local/bin/
