# rs-bitswap

## Example

### 1. Connect to IPFS Node
Paste in the addr of your local go-ipfs node

```cargo run --example ipfs "/ip4/127.0.0.1/tcp/4001"```

The output should look like:

```
Dialed "/ip4/127.0.0.1/tcp/4001"
Listening on "/ip4/127.0.0.1/tcp/58665"
P1: Recived Block from QmShkjkBoYXLezS4wZMvvE2zGVeSAC1WrALMgotg9aUtsT
P1: Cid bafybeifx7yeb55armcsxwwitkymga5xf53dxiarykms3ygqic223w5sk3m
P2: "\n&\u{8}\u{2}\u{12} Hello from IPFS Gateway Checker\n\u{18} "
```

### 2. Connect to rust peer

Start up the first peer

```cargo run --example peer1```

It will print the listening addr. Copy the listening addr 

```Listening on "/ip4/127.0.0.1/tcp/51120"```

On a new terminal start the second peer and paste in peer1 addr

```cargo run --example peer2 "/ip4/127.0.0.1/tcp/51120"```

It should output to the console:

```
P1: Recived Want from 12D3KooWPA8jwtzNBd97R7AxxJrvpPRPQcQWjQKRbfHdZYzk4dX7
P1: Sending Block to peer 12D3KooWPA8jwtzNBd97R7AxxJrvpPRPQcQWjQKRbfHdZYzk4dX7
P1: Recived Cancel bafkreicl7q5riggjyd5o7ns7lao7mkecswrd67wvin4o5tfrj3o5i77jxe from 12D3KooWPA8jwtzNBd97R7AxxJrvpPRPQcQWjQKRbfHdZYzk4dX7

P1: Recived Block from 12D3KooWD3H24ZyfvappEcX1go83VJXRGGY21YJ6t4jNdyC8ekcn
P1: Cid bafkreicl7q5riggjyd5o7ns7lao7mkecswrd67wvin4o5tfrj3o5i77jxe
P2: "Hey bro"
```

