FROM envoyproxy/envoy:v1.14.1

ENV WAIT_VERSION 2.7.2
ADD https://github.com/ufoscout/docker-compose-wait/releases/download/$WAIT_VERSION/wait /wait
RUN chmod +x /wait

COPY ./envoy.yaml /etc/envoy/envoy.yaml
CMD /wait && /usr/local/bin/envoy -c /etc/envoy/envoy.yaml
