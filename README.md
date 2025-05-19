# SIA DC-09 simulators

This repository contains `Dialler` and `Receiver` simulators for the `SIA DC-09` protocol.

## Dialler simulator

### Overview

The DC-09 Dialler Simulator is a command-line application designed to send DC-09 protocol messages to a specified receiver. The simulator supports repeating messages and customizing sequence numbers for testing scenarios.

### Features

- Send DC09 messages to a specified IP address and port.
- Configure message content, account number, and ID token.
- Support for message repetition and sequence number customization.
- Optional encryption with a user-provided key (16, 24, or 32 bytes).

### Usage

#### Command-Line Arguments

The application uses the following arguments, configurable via the command line:

| Argument           | Description                                                   | Default Value                 | Example                             |
|:-------------------|:--------------------------------------------------------------|:------------------------------|:------------------------------------|
| `--address`        | IP address of the receiver                                    | 127.0.0.1                     | --address 192.168.1.100             |
| `--port`, `-p`     | Port number of the receiver                                   | 8080                          | --port 9000                         |
| `--token`, `-t`    | ID token for the DC09 message                                 | SIA-DCS                       | --token ADM-CID                     |
| `--message`, `-m`  | Message content to send                                       | #1234\|NRR\|AStart of dialler | --message "#5678\|NRR\|Atest"       |
| `--account`, `-a`  | Dialler account number (automatically incremented if possible)| 1234                          | --account 5678                      |
| `--fixed`, `-f`    | Ensure that the account number is fixed across all diallers   | false                         | --fixed                             |
| `--sequence`, `-s` | Message sequence start number                                 | 1                             | --sequence 100                      |
| `--diallers`, `-d` | Number of diallers to create                                  | 1                             | --diallers 20                       |
| `--repeat`, `-r`   | Number of times to repeat the message per dialler             | 1                             | --repeat 5                          |
| `--key`, `-k`      | Encryption key for DC09 messages (16, 24, or 32 bytes)        | None                          | --key "my16bytekey1234567890abcdef" |
| `--udp`, `-u`      | Use a UDP connection instead of a TCP one                     | false                         | --udp                               |

#### Example commands

Send a custom message to a specific receiver with 3 repetitions:

```sh
./dialler --address 192.168.1.100 --port 9000 --message "#5678|NRR|Atest" --repeat 3
```

Send an encrypted message with a custom account and sequence number:

```sh
./dialler --account 5678 --sequence 100 --key "my16bytekey1234567890abcdef"
```

## Receiver simulator

### Overview

The DC-09 Receiver Simulator is a command-line test server that handles DC-09 dialler connections. This application listens on a specified IP address and port, decrypts incoming DC-09 messages using a user-provided key, and responds with `ACK` or `NAK` messages.

### Features

- Listens for incoming DC09 dialler connections.
- Optional encryption with a user-provided key (16, 24, or 32 bytes).
- Optional `NAK` response for received messages.

### Usage

#### Command-Line Arguments

The application uses the following arguments, configurable via the command line:

| Argument       | Description                                             | Default Value | Example                             |
|:---------------|:--------------------------------------------------------|:--------------|:------------------------------------|
| `--address`    | IP address to listen on                                 | 127.0.0.1     | --address 192.168.1.100             |
| `--port`, `-p` | Port number to listen on                                | 8080          | --port 9000                         |
| `--key`, `-k`  | Key to decrypt DC09 messages (16, 24, or 32 bytes long) | None          | --key "my16bytekey1234567890abcdef" |
| `--nak`        | Send `NAK` instead of `ACK` for received messages       | false         | --nak                               |

#### Example commands

Spin up a test server that tries to use encrypted communication with the `my16bytekey1234567890abcdef` key and sends `NAK` to all received messages.

```sh
./receiver --address 192.168.1.100 --port 9000 --key "my16bytekey1234567890abcdef" --nak
```

## License

[MIT](./LICENSE)
