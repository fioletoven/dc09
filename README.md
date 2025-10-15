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
| `--scenarios`      | Configuration file specifying defined scenarios for the run   | `None`        | --scenarios examples/scenarios.json |

#### Example commands

Send a custom message to a specific receiver with 3 repetitions:

```sh
./dialler 192.168.1.100 --port 9000 --token "SIA-DCS" --message "NRR|Atest" --repeat 3
```

Send an encrypted message with a custom account and sequence number:

```sh
./dialler --account 5678 --sequence 100 --key "my16bytekey1234567890abcdef"
```

## Receiver simulator

### Overview

The DC-09 Receiver Simulator is a command-line test server that handles DC-09 dialler connections. This application listens on a specified IP address and port, decrypts incoming DC-09 messages using a user-provided key, and responds with `ACK` or `NAK` messages.

### Features

- Listens for incoming DC09 dialler connections (TCP and UDP).
- Optional encryption with a user-provided key (16, 24, or 32 bytes).
- Optional `NAK` response for received messages.
- Assign distinct keys to different account names using scenario files.

### Usage

#### Command-Line Arguments

The application uses the following arguments, configurable via the command line:

| Argument       | Description                                             | Default Value | Example                             |
|:---------------|:--------------------------------------------------------|:--------------|:------------------------------------|
| _\[ADDRESS\]_  | IP address to listen on                                 | 127.0.0.1     | 192.168.1.100                       |
| `--port`, `-p` | Port number to listen on                                | 8080          | --port 9000                         |
| `--key`, `-k`  | Key to decrypt DC09 messages (16, 24, or 32 bytes long) | None          | --key "my16bytekey1234567890abcdef" |
| `--nak`        | Send `NAK` instead of `ACK` for received messages       | false         | --nak                               |
| `--scenarios`  | Configuration file specifying keys for the diallers     | None          | --scenarios examples/scenarios.json |

#### Example commands

Spin up a test server that tries to use encrypted communication with the `my16bytekey1234567890abcdef` key and sends `NAK` to all received messages.

```sh
./receiver 192.168.1.100 --port 9000 --key "my16bytekey1234567890abcdef" --nak
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

[MIT](./LICENSE)
