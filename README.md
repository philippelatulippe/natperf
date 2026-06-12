How many simultaneous connections does your NAT or CGNAT allow?

Find out with this server and client pair.

### Usage examples

Run the server on a remote host with direct IPv4 access:

    cargo run server 1234
    
Start a client that will open 1024 connections:

    cargo run client 1.2.3.4 1234 1024 > ports.csv

The output will be a single line of comma-separated ports (or network errors)

The connections will stay open 5 seconds to make sure they all overlap.

If there are no network errors, try with a higher number of connections.
