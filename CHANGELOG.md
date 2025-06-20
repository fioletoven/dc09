# Change Log

## 0.1.4 - 2025-06-07

### Features

- allow defining signal sequences in scenario files for the dialler simulator

## 0.1.3 - 2025-05-25

### Features

- add support (initial) for scenario files in the dialler and receiver applications
- allow distinct keys for different account names

## 0.1.2 - 2025-05-19

### Features

- add UDP support to the dialler and receiver apps

## 0.1.1 - 2025-05-10

### Features

- allow creating multiple diallers in the diallers app

## 0.1.0 - 2025-05-04

First release of DC-09 dialler and receiver simulators.

### Features

#### Dialler

- send DC09 messages to a specified IP address and port
- configure message content, account number, and ID token
- support for message repetition and sequence number customization
- optional encryption with a user-provided key (16, 24, or 32 bytes)

#### Receiver

- listens for incoming DC09 dialler connections
- optional encryption with a user-provided key (16, 24, or 32 bytes)
- optional `NAK` response for received messages
