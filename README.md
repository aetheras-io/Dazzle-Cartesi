# Dazzle Dapp

## Cartesi local environment setup
1. Prepare wallet
2. Launch cartesi rollup in local (host-mode or prod-mode)

### Wallet Prepartion
- Before testing, you'll need to setup Metamask chrome extension and make sure your wallets and balance are ready.
- Wallets for local environment
    - If you are going to test Cartesi on your local environment, there is no need to generate your own wallets, `hardhat` framework already provided some test wallets that have sufficient balance. Just use the test mnemonic words `"test test test test test test test test test test test junk"` to import the test wallets into your Metamask.
    - Make sure you switch network to `localhost:8545`, and change the chainId to `31337`

### Testing Cartesi in development phase (Host-mode on local)
- Start up Rust dapp on host
```
cd ./server
make debug
```
- Open another terminal window, launch Cartesi Rollups in host-mode
```
cd ./cartesi-dapp
make host-mode
```
- Wait for the Cartesi Rollups to settle down and stop pop up error message, eventually the container logs will repeatedly show the following, then you can start testing
```
[2022-09-01T09:05:50Z INFO  host_server_manager::controller] fetch request timed out; setting state to idle
[2022-09-01T09:05:50Z INFO  actix_web::middleware::logger] 172.30.0.1 "POST /finish HTTP/1.1" 202 27 "-" "-" 9.979536
[2022-09-01T09:05:50Z INFO  host_server_manager::controller] received finish request; changing state to fetch request
[2022-09-01T09:05:54Z INFO  host_server_manager::grpc::server_manager] received get_session_status with id=default_rollups_id
[2022-09-01T09:05:54Z INFO  host_server_manager::grpc::server_manager] received get_epoch_status with id=default_rollups_id and epoch_index=0
[2022-09-01T09:05:57Z INFO  host_server_manager::grpc::server_manager] received get_session_status with id=default_rollups_id
[2022-09-01T09:05:57Z INFO  host_server_manager::grpc::server_manager] received get_epoch_status with id=default_rollups_id and epoch_index=0
```
- If you want to shutdown Cartesi Rollup
```
cd ./cartesi-dapp
make shutdown
```

### Testing Cartesi in deployment phase (Prod-mode on local)
- Build Rust binary for Cartesi environment
```
cd ./server
make build-cartesi
```

- Build docker images for Cartesi Rollups
```
cd ./cartesi-dapp
make images
make prod-mode
```
- Wait for the Cartesi Rollups to settle down and stop pop up error message, eventually the container logs will repeatedly show the following, then you can start testing
```
[2022-09-01T09:05:50Z INFO  host_server_manager::controller] fetch request timed out; setting state to idle
[2022-09-01T09:05:50Z INFO  actix_web::middleware::logger] 172.30.0.1 "POST /finish HTTP/1.1" 202 27 "-" "-" 9.979536
[2022-09-01T09:05:50Z INFO  host_server_manager::controller] received finish request; changing state to fetch request
[2022-09-01T09:05:54Z INFO  host_server_manager::grpc::server_manager] received get_session_status with id=default_rollups_id
[2022-09-01T09:05:54Z INFO  host_server_manager::grpc::server_manager] received get_epoch_status with id=default_rollups_id and epoch_index=0
[2022-09-01T09:05:57Z INFO  host_server_manager::grpc::server_manager] received get_session_status with id=default_rollups_id
[2022-09-01T09:05:57Z INFO  host_server_manager::grpc::server_manager] received get_epoch_status with id=default_rollups_id and epoch_index=0
```

- If you want to shutdown Cartesi Rollup
```
cd ./cartesi-dapp
make shutdown
```
```
