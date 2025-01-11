# Camera Hook!

[![Docker](https://github.com/gckopper/camera-hook/actions/workflows/docker-publish.yml/badge.svg)](https://github.com/gckopper/camera-hook/actions/workflows/docker-publish.yml)

- Works for me!
- Uses distroless as container base
- No openssl

## The idea

In my house I have a wireless doorbell and so I loaded an ESP8266 with Tasmota and connected it to an RF module to intercept the communications between the indoor unit of my doorbell and the outdoor one. When someone rings the doorbell, the microcontroller posts a message to an MQTT topic (over wifi) and this project listens to it, pulls a picture from a security camera that is pointing towards the doorbell and sends that to me using a Discord webhook.

![Diagram as described above](https://github.com/user-attachments/assets/4ce1b859-6adc-4b8f-a648-0fcb5d04bf85#gh-light-mode-only)
![Diagram as described above](https://github.com/user-attachments/assets/3a30318b-777c-41d5-bc15-dc372f4e1cfd#gh-dark-mode-only)

## Build (Manual)

Requires Rust. Binaries are stored in the target folder!

### Debug
`cargo build`

### Release
`cargo build --release`

## Build (Docker)

`docker build -t camera-hook .`

## Usage:

To deploy this with docker compose (or nerdctl) remember to copy the `.env.example` file, rename it to `.env` and populate it according to your needs.
