# Algo Technical Challenge - Q1 2020 - (L2)

## The Solution

The task has been completed in full. 
I allowed myself the luxury of testing the server from tests without writing a rust grpc-client, but connected from ipython, the server returns everything perfectly.

## The Challenge
Using  Rust, code a mini-project that:

1. Connects to two exchanges' websocket feeds at the same time
2. Pulls order books, using these streaming connections, for a given traded pair of currencies (configurable), from each exchange
3. Merges and sorts the order books to create a combined order book
4. From the combined book, publishes the spread, top ten bids, and top ten asks, as a stream, through a gRPC server

## Background

### Markets
A market is generally a pair of currencies and an exchange where they are traded. For example, ETH (Ethereum) and BTC(Bitcoin) are a pair that together from a traded ‘symbol’, - ETHBTC. This means you can buy or sell ETH using BTC as the ‘pricing’ currency.

### Order books
Orders at which people are prepared to buy and sell are send to an exchange, such as Binance. The exchange will usually match the buy and sell orders that approach a market ‘mid-price’. The difference between the best ask price and the best bid price is called the spread.

The final, merged order book should have the best deals first. That means, if I am selling currency and I want to be the first one to sell, I should be at the best position for this which means am selling the largest amount at the lowest price. Think about this when sorting each side of the order book.

### gRPC
[GRPC](https://grpc.io/) is relatively modern Remote Procedure Call protocol. If you have used anything like GraphQL or Thrift, it should be fairly familiar. If not, it is not difficult to learn! If you are new to RPC you may even find you prefer gRPC’s structured and typed protocol over HTTP 

For this challenge you can use this schema to create your gRPC:
```proto
syntax = "proto3";
package orderbook;
service OrderbookAggregator {
    rpc BookSummary(Empty) returns (stream Summary);
}
message Empty {}
message Summary {
    double spread = 1;
    repeated Level bids = 2;
    repeated Level asks = 3;
}
message Level {
    string exchange = 1;
    double price = 2;
    double amount = 3;
}
```
