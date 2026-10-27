Engineering Challenge

Your role at Voluntas will primarily be building and maintaining data collectors for various kinds of financial data (trades, quotes, etc.) and alternative data (weather data, blockchain data, etc.). This challenge will have you build a service that captures one of those types of data, from a single exchange.

Background
One of the most important types of financial data is order book data, which consists of snapshots and price level updates. This data is immensely useful for backtesting, since it can give you a view into the available liquidity for an instrument at any given point in time.

Animated ETH/BTC order-book depth chart: green bids on the left and red asks on the right change as available liquidity moves over time.
Pause animation
However, in order for this data to be useful, it is extremely important that it is complete. If we miss data, our backtests may show that a particular price level still exists, when in reality, a price update was sent to delete it, but our collector missed it. The timing of when these price level update messages arrived is also extremely important for high frequency strategies, where the difference of a few milliseconds can be the difference between a taker algorithm making money, or missing the trade. As such, it is highly important for our order book data to be both complete, and time-accurate.

Challenge
Your job is to connect to Kalshi, subscribe to all live symbols, and maintain an orderbook for each symbol. MVEs are not included in this challenge -- just regular markets. You should also continuously monitor for new symbols and subscribe to them, as well as unsubscribe from symbols that are no longer trading.

This service should write orderbook deltas to stdout, along with a full snapshot every minute. To write this output, you should use the code in the challenge repo. It contains the various helpers you'll need to make this seamless. You must use these helpers, so that the output is in a standard format. Upon receiving a SIGINT, you should close your websockets, as well as the file you are currently writing to.

Notes:

Connecting to Kalshi for orderbook data may require an API key. If you cannot create a Kalshi account, you may build your challenge based on Polymarket.
Subscribing to all symbols on a single websocket will likely not work. You should load balance the symbols across multiple websockets.
Extensions
This challenge is meant to be a toy example, and based purely on the spec above, it would not be enough for a production trading environment. I've listed a few extensions to the challenge here, where you can improve your solution to be better! I recommend you read through the extensions even if you don't end up doing all of them.

Extension 1: Redundant Websockets
This is the first and most important extension. Exchanges can encounter the following problems:

Sockets can disconnect at any time, without warning. While we are reconnecting, we are missing data.
Subscriptions can go quiet without any notification from the exchange. This means we are still connected to the socket, and at the application layer, we are still "subscribed" to the symbol.
Exchanges may rate limit our IP and/or connection to a certain number of messages per second, or they may even just be unintentionally buffering messages to the same connection, causing the time we receive the message to be greatly delayed.
We may encounter an error in the process, causing us to go down unexpectedly
By managing multiple redundant websockets to the exchange, you can reduce your dataloss.

Some notes:

When you are switching your "source" websocket to be a new one, you will need to emit a snapshot message, since the two websockets have potentially different orderbook representations in memory, due to dropped / delayed messages.
An exchange feeds sockets A and B. When A stalls, switch to healthy socket B and emit a fresh snapshot before continuing with deltas.
Extension 2: Subscription monitoring
Exchanges won't always be so nice as to tell us we are exceeding their rate limits, and sometimes their code simply can't keep up.

We also need to build a system that monitors the number of data points per second (or per minute, for low activity markets). If activity for a specific symbol dips too low, it should be unsubscribed and resubscribed.

Additionally, it is a good practice periodically fully disconnect a full websocket, and then bring it back online with a new connection, resubscribing to all the symbols on that socket. This is to mitigate cumulative bugs on both the exchange's side, such as memory leaks. Remember: You cannot expect the exchange to write perfect code, and we should anticipate these types of issues!

A symbol's update rate falls below a threshold. Unsubscribe and resubscribe, then keep monitoring. Periodically reconnect whole sockets as well.
Extension 3: Generic encode/decode per-exchange
Many elements of the base challenge, as well as extensions 1 & 2 are required regardless of the exchange we are connecting to. Try making your service generic, so the only thing we need to change is the encoding/decoding on the websocket layer (don't worry about exchanges that don't use websockets)!

Kalshi and Polymarket each use an exchange-specific encoder and decoder. Both connect to the same collector for subscriptions, order books, and common snapshot and delta output.
Extension 4: Latency improvements
There are a lot of ways to get faster connections from exchanges. For example, being the first to connect after their schedule maintenance, or spamming connections until you find a fast one, or changing your source port on TCP! While the correctness/completeness of the data is the #1 priority for this project, it would be interesting to see experimentation into how to do this well on various exchanges like Polymarket or Kalshi. Please write your discoveries in a markdown file!

The same exchange update reaches connection A after 18 milliseconds and connection B after 7 milliseconds. Compare receive times across connections. These are illustrative timings.