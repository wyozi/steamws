FROM rust:1.44 as builder

WORKDIR /usr/src/steamws
COPY ./steam_sdk ./steam_sdk
RUN STEAM_SDK_LOCATION=steam_sdk cargo install steamws

FROM steamcmd:root
RUN apt-get update && apt-get install -y extra-runtime-dependencies
COPY --from=builder /usr/local/cargo/bin/steamws /usr/local/bin/steamws
CMD ["gma"]