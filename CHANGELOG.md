# Change Log

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
