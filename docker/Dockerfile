FROM ubuntu:20.04

VOLUME /iso/
VOLUME /bundle/
VOLUME /install/
VOLUME /output/

ENV DEBIAN_FRONTEND noninteractive
RUN apt-get update && \
	apt-get install -yq --no-install-recommends \
		libdigest-perl-md5-perl \
		libfontconfig1 \
		git-core \
		python3 \
		wget \
	&& \
	rm -rf /var/lib/apt/lists/*


COPY --chmod=770 entrypoint.sh /
COPY scripts /scripts

ENTRYPOINT ["/entrypoint.sh"]
CMD ["help"]
