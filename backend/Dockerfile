# PocketBase with this repo's migrations and hooks baked in (ADR 0012).
# Production: mount a persistent volume on /pb_data (SQLite DB); configure through
# pb_settings.json and env vars (see .env.example).
# Dev (docker compose) bind-mounts pb_migrations, pb_hooks and the settings files over the copies.
# Test instances: WWC_SEED=1 seeds demo data on every boot (ADR 0023).
FROM alpine:3.22

# PocketBase is pre-1.0: upgrades are deliberate, bump this in its own change.
ARG PB_VERSION=0.40.4
ARG TARGETARCH=amd64

RUN apk add --no-cache ca-certificates \
 && apk add --no-cache --virtual .fetch unzip wget \
 && cd /tmp \
 && base="https://github.com/pocketbase/pocketbase/releases/download/v${PB_VERSION}" \
 && zip="pocketbase_${PB_VERSION}_linux_${TARGETARCH}.zip" \
 && wget -q "$base/$zip" "$base/checksums.txt" \
 && grep " $zip\$" checksums.txt | sha256sum -c - \
 && unzip -q "$zip" pocketbase -d /usr/local/bin \
 && rm -f "$zip" checksums.txt \
 && apk del .fetch \
 && adduser -D -H -u 10001 pocketbase \
 && mkdir /pb_data \
 && chown pocketbase:pocketbase /pb_data

COPY --chmod=755 entrypoint.sh /entrypoint.sh
COPY pb_migrations /pb_migrations
COPY pb_hooks /pb_hooks
COPY pb_settings.json pb_settings.dev.json /

# Test instances only: the seed script's tools. On Render a service's env vars are also build args,
# so WWC_SEED=1 there installs them too. Release and dev builds don't set it.
ARG WWC_SEED
RUN if [ "$WWC_SEED" = "1" ]; then apk add --no-cache bash curl jq; fi
COPY --chmod=755 seed/seed.sh /seed/seed.sh

USER pocketbase
VOLUME /pb_data
EXPOSE 8090
HEALTHCHECK --interval=10s --timeout=3s --retries=5 \
  CMD wget -qO- http://127.0.0.1:8090/api/health >/dev/null || exit 1
ENTRYPOINT ["/entrypoint.sh"]
