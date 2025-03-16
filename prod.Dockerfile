FROM scratch
ARG APP

COPY ${APP} /app

EXPOSE 8080
WORKDIR /app
ENV ROCKET_CONFIG=/app/Rocket.toml
ENV ROCKET_PROFILE=release
CMD [ "/app/app" ]

