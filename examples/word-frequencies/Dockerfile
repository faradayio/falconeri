FROM ubuntu

RUN apt-get update && \
    apt-get install -y \
        ca-certificates \
        python-pip \
        && \
    apt-get clean && rm -rf /var/lib/apt/lists/* && \
    pip install awscli --upgrade

ADD falconeri-worker word-frequencies.sh /usr/local/bin/
