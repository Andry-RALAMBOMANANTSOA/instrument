# **Instrument: Agricultural Futures Matching Engine**

## Overview

**Instrument** is a Rust-based matching engine designed for processing orders in agricultural futures markets. The project provides a REST API with WebSocket support using the Actix framework, allowing real-time interaction with market data and order processing. The system is highly configurable, enabling multiple instances of the matching engine to be run with different market configurations.

This repository is intended to work alongside the **shared_struct** repository, which contains the global data structures used within the Instrument. These shared data structures ensure consistent data handling across different components of the matching engine.

## Ownership and Broker Integration

The Instrument (ex: soja_juillet_2025) is owned by the exchange, and all traders must be subscribed through a broker. Brokers are responsible for forwarding traders' orders to the exchange's Instrument after conducting their own risk management checks.

- **HMAC Key Validation**: Each broker must have a unique HMAC key to communicate securely with the exchange's Instrument. This key is used to validate and authenticate all orders sent to the exchange.
- **Clearing and Trader Information**: Brokers can fetch clearing data and trader information directly from the central MongoDB database generated by the Instrument. This database contains the necessary trading and market data required for brokers to manage their traders effectively.
- **MessagePack Serialization**: Communication between brokers and the exchange must use the MessagePack serialization format. This ensures efficient, compact, and fast data transfer between the broker's systems and the exchange's Instrument.

## Features

- **REST API Endpoints**: Provides endpoints for managing orders and retrieving market data.
- **WebSocket Support**: Real-time data streaming for market events to connected clients.
- **MongoDB Integration**: Stores trading information and market data for later analysis and retrieval.
- **Configurable Market Instances**: Allows multiple instruments to run in parallel using different market configurations defined in JSON files.
- **Scalable Architecture**: Can be deployed on multiple servers with unified server names using Nginx.
- **Broker-Based Order Processing**: Traders' orders are processed through brokers, who must pass risk management checks before submitting orders to the exchange.

## Configuration

### `marketcg.json`

To run different market instances, configure the `marketcg.json` file at the root of the project. This file defines the market parameters, including exchange name, contract specifications, and tick sizes.

Example configuration:
```json
{
  "exchange": "Your_exchange_name",
  "market_name": "your_instrument_name",
  "contract": 100,
  "tick_size": 10,
  "tick_value": 1000,
  "quotation": "MGA"
}
```
### Environment Variables

The server URL and other necessary configurations can be defined in the `.env` file. Each instrument will have a different server URL, but they can be unified under a single server name using Nginx.

## Project Structure

### REST API Endpoints

The following routes are available for order management and market data retrieval. All routes are dynamically built using the `market_name` specified in the `marketcg.json` file.

#### Order Management

- `/order/limit_order/{market_name}`: Handle limit orders
- `/order/iceberg_order/{market_name}`: Handle iceberg orders
- `/order/market_order/{market_name}`: Handle market orders
- `/order/stop_order/{market_name}`: Handle stop orders
- `/order/stoplimit_order/{market_name}`: Handle stop-limit orders
- `/order/modify_order/{market_name}`: Modify existing orders
- `/order/modify_iceberg_order/{market_name}`: Modify iceberg orders
- `/order/delete_order/{market_name}`: Delete orders
- `/order/delete_iceberg_order/{market_name}`: Delete iceberg orders
- `/order/save/{market_name}`: Save orders to the database

#### Historical Data

- `/history_last/{market_name}`: Retrieve last traded prices
- `/history_bbo/{market_name}`: Retrieve best bid and offer
- `/history_tns/{market_name}`: Retrieve trade and sales data
- `/history_mbpevent/{market_name}`: Retrieve market-by-price events
- `/history_volume/{market_name}`: Retrieve volume data
- `/full_ob/{market_name}`: Retrieve the full order book
- `/full_interest/{market_name}`: Retrieve full interest data
- `/history_interestevent/{market_name}`: Retrieve historical interest events

### WebSocket Routes

WebSocket routes allow real-time data streaming for connected clients:

- `/ws/last_rt/{market_name}`: Real-time last traded prices
- `/ws/mbp_event_rt/{market_name}`: Real-time market-by-price events
- `/ws/best_bid_offer_rt/{market_name}`: Real-time best bid and offer
- `/ws/volume_rt/{market_name}`: Real-time volume updates
- `/ws/time_sale_rt/{market_name}`: Real-time time and sales data
- `/ws/interest_event_rt/{market_name}`: Real-time interest events

## Usage

### Starting the Server

Ensure you have the required environment variables and market configurations set up in the `.env` file and `marketcg.json` file before running the server.

```bash
cargo run --release
```

### Example Nginx Configuration

To unify multiple instrument instances under a single server name, you can use an Nginx configuration like this:

```nginx
server {
    listen 80;
    server_name example.com;

    location /instrument1_or_katsaka_mai_2025 {
        proxy_pass http://127.0.0.1:8081;
    }

    location /instrument2_or_voanjo_avril_2025 {
        proxy_pass http://127.0.0.1:8082;
    }
}
```
## Dependencies

This project uses the following main dependencies:

- **Actix-Web**: For building the REST API and WebSocket server.
- **MongoDB**: For database operations to store market data and trading information.
- **shared_struct**: For global data structures shared across the application.
- **MessagePack**: For efficient serialization and deserialization of data between brokers and the exchange.

## Related Repositories

- [**shared_struct**](https://github.com/Andry-RALAMBOMANANTSOA/shared_structs): Contains global structures that are utilized by the Instrument matching engine.

## Contributing

Contributions are welcome! Please open an issue or submit a pull request for any bugs, feature requests, or improvements.

## License

This project is licensed under the MIT License. See the LICENSE file for more details.

## Contact

For any questions or support, please open an issue on this repository or send me email to ralambomanantsoaandry@gmail.com.
