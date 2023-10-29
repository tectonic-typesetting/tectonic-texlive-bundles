FROM ubuntu:20.04

VOLUME /iso.img
VOLUME /bundle/
VOLUME /install/

ENV DEBIAN_FRONTEND noninteractive
RUN apt-get update && \
	apt-get install -yq --no-install-recommends \
		libdigest-perl-md5-perl \
		libfontconfig1 \
		wget grep faketime \
	&& \
	rm -rf /var/lib/apt/lists/*


COPY --chmod=770 entrypoint.sh /

ENTRYPOINT [ "/entrypoint.sh" ]
CMD [ "help" ]