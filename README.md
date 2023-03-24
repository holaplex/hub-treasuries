# Hub Treasuries
The Holaplex Hub manages project and customer treasuries and wallets. Additionally, Hub treasuries are responsible for signing and submitting transactions for custodial wallets.


## Getting Started
The service requires credentials to Fireblocks.

```
# .env.local
FIREBLOCKS_API_KEY=
```

The private key of the Fireblocks API user.

```
# fireblocks_secret.key
-----BEGIN PRIVATE KEY-----
...
-----END PRIVATE KEY-----
```

```
docker compose up
cargo run --bin holaplex-hub-treasuries
```

Visit [http://localhost:3007/playground](http://localhost:3007/playground) to access GraphQL playground.
