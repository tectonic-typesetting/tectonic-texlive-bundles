FROM ubuntu:20.04

# Do NOT include this, iso.img is a file.
# this directive makes a directory.
#VOLUME /iso.img
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


COPY entrypoint.sh /

ENTRYPOINT [ "bash", "/entrypoint.sh" ]
CMD [ "help" ]