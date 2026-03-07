# SIA DC-09 simulators

This repository contains `Dialler` and `Receiver` simulators for the `SIA DC-09` protocol.

## Dialler simulator

### Overview

The DC-09 Dialler Simulator is a command-line application designed to send DC-09 protocol messages to a specified receiver. The simulator supports creating multiple diallers and repeating messages for testing scenarios.

### Features

- Send DC09 messages to a specified IP address and port (TCP and UDP).
- Configure message content, account number, and ID token.
- Support for message repetition and sequence number customization.
- Optional encryption with a user-provided key (16, 24, or 32 bytes).
- Support for scenario files.

### Usage

#### Command-Line Arguments

The application uses the following arguments, configurable via the command line:

| Argument           | Description                                                   | Default Value | Example                             |
|:-------------------|:--------------------------------------------------------------|:--------------|:------------------------------------|
| _\[ADDRESS\]_      | IP address of the receiver                                    | 127.0.0.1     | 192.168.1.100                       |
| `--port`, `-p`     | Port number of the receiver                                   | 8080          | --port 9000                         |
| `--token`, `-t`    | ID token for the DC09 message                                 | NULL          | --token ADM-CID                     |
| `--message`, `-m`  | Message content to send                                       | `None`        | --message "NRR\|AStart of dialler"  |
| `--account`, `-a`  | Dialler account number (automatically incremented if possible)| 1234          | --account 5678                      |
| `--line`, `-l`     | Receiver line number (account prefix)                         | `None`        | --line L01                          |
| `--receiver`, `-r` | Receiver number                                               | `None`        | --receiver R01                      |
| `--fixed`, `-f`    | Ensure that the account number is fixed across all diallers   | false         | --fixed                             |
| `--sequence`, `-s` | Message sequence start number                                 | 1             | --sequence 100                      |
| `--diallers`, `-d` | Number of diallers to create                                  | 1             | --diallers 20                       |
| `--repeat`, `-c`   | Number of times to repeat the message per dialler             | 1             | --repeat 5                          |
| `--key`, `-k`      | Encryption key for DC09 messages (16, 24, or 32 bytes)        | `None`        | --key "my16bytekey1234567890abcdef" |
| `--udp`, `-u`      | Use a UDP connection instead of a TCP one                     | false         | --udp                               |
| `--show`           | Display mode for sent messages (target, plain or both)        | target        | --show both                         |
| `--scenarios`      | Configuration file specifying defined scenarios for the run   | `None`        | --scenarios examples/scenarios.json |
| `--timeout`        | Timeout for waiting for a response, in seconds                | 1             | --timeout 10                        |

#### Example commands

Send a custom message to a specific receiver with 3 repetitions:

```sh
./dialler 192.168.1.100 --port 9000 --token "SIA-DCS" --message "NRR|Atest" --repeat 3
```

Send an encrypted message with a custom account and sequence number:

```sh
./dialler --account 5678 --sequence 100 --key "my16bytekey1234567890abcdef"
```

Send a NULL message and wait indefinitely for a response:

```sh
./dialler --account 1234 --line L02 --receiver R001 --timeout 0
```

## Receiver simulator

### Overview

The DC-09 Receiver Simulator is a command-line test server that handles DC-09 dialler connections. This application listens on a specified IP address and port, decrypts incoming DC-09 messages using a user-provided key, and responds with `ACK`, `NAK` or `DUH` messages.

### Features

- Listens for DC-09 connections over **TCP** and **UDP**
- Optional AES encryption/decryption with user-provided key (16, 24, or 32 bytes)
- Per-account key support via scenario configuration file
- Configurable static response mode: always `ACK`, `NAK` or `DUH`
- Dynamic response mode switching via HTTP API (override command-line setting)
- Prometheus metrics

### Usage

#### Command-Line Arguments

| Argument          | Description                                                                 | Default       | Example                                    |
|:------------------|:----------------------------------------------------------------------------|:--------------|:-------------------------------------------|
| _[ADDRESS]_       | IP address to listen on                                                     | 127.0.0.1     | 192.168.1.100                              |
| `--port`, `-p`    | Port number to listen on (DC-09 traffic)                                    | 8080          | `--port 9000`                              |
| `--key`, `-k`     | Default decryption key (16, 24 or 32 bytes)                                 | None          | `--key "my16bytekey1234567890abcdef"`      |
| `--metrics`, `-m` | Port number for metrics server (Prometheus metrics)                         | 9090          | `--metrics 5000`                           |
| `--nak`           | Always send `NAK` instead of `ACK`                                          | false         | `--nak`                                    |
| `--duh`           | Always send `DUH` instead of `ACK`                                          | false         | `--duh`                                    |
| `--show`          | Display received messages: `target`, `plain` or `both`                      | `target`      | `--show both`                              |
| `--scenarios`     | JSON file with per-account keys and settings                                | None          | `--scenarios examples/scenarios.json`      |

**Note:** `--nak` and `--duh` are mutually exclusive. If neither is set, the default is `ACK`. The HTTP API can override this behaviour at runtime.

#### Example commands

Basic encrypted receiver that always NAKs:

```bash
./receiver 192.168.1.100 --port 9000 --key "my16bytekey1234567890abcdef" --nak
```

Run with per-account keys and show encrypted and decrypted DC-09 messages:

```bash
./receiver --port 5140 --scenarios ./test-accounts.json --show both
```

### Prometheus Metrics

Exposed at: `http://<address>:<port>/metrics`

Main metrics:

| Metric name                              | Type      | Labels                  | Description                                       |
|:-----------------------------------------|:----------|:------------------------|:--------------------------------------------------|
| `dc09_heartbeat_received_total`          | Counter   | `account`               | Heartbeat / null messages received                |
| `dc09_messages_received_total`           | Counter   | `token`, `account`      | Total DC-09 messages received                     |
| `dc09_messages_failed_total`             | Counter   | `transport`, `reason`   | Messages that failed parsing / processing         |
| `dc09_connections_total`                 | Counter   | `transport`             | Total connections accepted (tcp/udp)              |
| `dc09_active_connections`                | Gauge     | -                       | Currently active client connections               |
| `dc09_last_message_timestamp_seconds`    | Gauge     | `account`               | Unix timestamp of most recent message per account |
| `dc09_message_size_bytes`                | Histogram | `transport`             | Size distribution of received messages (bytes)    |

Example Grafana dashboard: [grafana-dashboard.json](./examples/grafana-dashboard.json).

### HTTP Control API

A lightweight HTTP server runs on the **same port** as Prometheus metrics (to keep firewall/NAT rules simple).

| Method | Endpoint               | Description                          |
|--------|------------------------|--------------------------------------|
| `GET`  | `/mode`                | Get response modes for all types     |
| `GET`  | `/mode/{type}`         | Get response mode for a single type  |
| `PUT`  | `/mode/{type}/{mode}`  | Set response mode for a single type  |

| Parameter | Values                         |
|-----------|--------------------------------|
| `{type}`  | `message`, `heartbeat`         |
| `{mode}`  | `ack`, `nak`, `duh`, `none`    |

> Currently it is possible to set separate response modes for messages and heartbeats only via HTTP API.

#### Examples

```bash
# Get all modes
curl http://192.168.1.100:9090/mode
{"message":"ack","heartbeat":"ack"}

# Get mode for heartbeats
curl http://192.168.1.100:9090/mode/heartbeat
{"heartbeat":"ack"}

# Set message response to NAK (overrides command-line setting)
curl -X PUT http://192.168.1.100:9090/mode/message/nak
{"message":"nak"}

# Stop responding to heartbeats (useful for timeout/retransmission testing)
curl -X PUT http://192.168.1.100:9090/mode/heartbeat/none
{"heartbeat":"none"}
```

## Scenario files

It is possible to provide a JSON scenario file to the `Dialler` and `Receiver` simulators (using `--scenario` argument).

The JSON file defines a collection of diallers and their associated test scenarios, including sequences of signals to be sent during testing.  
The `Receiver` simulator uses only **diallers** array to get `name` and `key` properties for configured diallers.

Example scenarios file: [scenarios.json](./examples/scenarios.json).

### JSON Structure

#### Root Object

The root object contains two main arrays: `diallers` and `scenarios`.

- **diallers**: An array of dialler configurations.
- **scenarios**: An array of test scenarios, each with a unique identifier and a sequence of signals.

#### Diallers

Each entry in the `diallers` array represents a dialler configuration with the following properties:

| Property   | Type     | Description                                            | Required |
|------------|----------|--------------------------------------------------------|----------|
| `name`     | String   | Unique identifier for the dialler (e.g., "1234").      | Yes      |
| `count`    | Integer  | Number of diallers to create with this configuration.  | No       |
| `key`      | String   | Encryption key (16, 24, or 32 bytes) or `null`.        | No       |
| `receiver` | String   | Receiver identifier (e.g., "R001").                    | No       |
| `prefix`   | String   | Prefix identifier (e.g., "L001").                      | No       |
| `scenarios`| Array    | List of scenario IDs to be executed (e.g., `[1, 2]`).  | No       |
| `sequence` | Integer  | Sequence number start for the messages.                | No       |
| `udp`      | Boolean  | Indicates if UDP protocol is used (`true` or `false`). | No       |

> If `scenarios` is not specified, the dialler will use the token and message provided on the command line.

#### Scenarios

Each entry in the `scenarios` array defines a test scenario with the following properties:

| Property   | Type     | Description                                         | Required |
|------------|----------|-----------------------------------------------------|----------|
| `id`       | Integer  | Unique identifier for the scenario (e.g., 1).       | Yes      |
| `sequence` | Array    | Ordered list of signals to be sent in the scenario. | Yes      |

###### Sequence Array

Each entry in the `sequence` array represents a signal with the following properties:

| Property   | Type     | Description                                                      | Required |
|------------|----------|------------------------------------------------------------------|----------|
| `token`    | String   | ID token of the signal (e.g., "NULL", "SIA-DCS", "ADM-CID").     | Yes      |
| `message`  | String   | Message content for the signal (e.g., "NRR\|AStart of dialler"). | No       |
| `delay`    | Integer  | Delay in milliseconds before sending the signal (e.g., 5000).    | No       |
| `repeat`   | Integer  | Number of times to repeat the signal (e.g., 100).                | No       |

> If `delay` is not specified, the signal is sent immediately.

## License

This project is licensed under the **MIT License**.  
See the [LICENSE](./LICENSE) file for the full text.

### Third-party dependencies

All third-party Rust crates and their licenses are listed in [THIRD_PARTY_LICENSES.md](./THIRD_PARTY_LICENSES.md).
