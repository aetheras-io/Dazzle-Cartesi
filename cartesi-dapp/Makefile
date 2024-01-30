# Copyright 2022 Cartesi Pte. Ltd.
#
# SPDX-License-Identifier: Apache-2.0
# Licensed under the Apache License, Version 2.0 (the "License"); you may not use
# this file except in compliance with the License. You may obtain a copy of the
# License at http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software distributed
# under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
# CONDITIONS OF ANY KIND, either express or implied. See the License for the
# specific language governing permissions and limitations under the License.

DAPP_NAME := cartesi-dapp
INSPECT_PORT := 5005
GRAPHQL_PORT := 4000
NETWORK := sepolia

images:
	docker buildx bake -f docker-bake.hcl -f docker-bake.override.hcl --load

prod-mode:
	DAPP_NAME=${DAPP_NAME} INSPECT_PORT=${INSPECT_PORT} GRAPHQL_PORT=${GRAPHQL_PORT} docker compose -f ./docker-compose.yml -f ./docker-compose.override.yml up

host-mode:
	DAPP_NAME=${DAPP_NAME} INSPECT_PORT=${INSPECT_PORT} GRAPHQL_PORT=${GRAPHQL_PORT} docker compose -f ./docker-compose.yml -f ./docker-compose.override.yml -f ./docker-compose-host.yml up

shutdown:
	DAPP_NAME=${DAPP_NAME} INSPECT_PORT=${INSPECT_PORT} GRAPHQL_PORT=${GRAPHQL_PORT} docker-compose -f ./docker-compose.yml -f ./docker-compose.override.yml down -v

#NOTES: This is useful when try to test Vouchers
test-advancing-time:
	curl --data '{"id":1337,"jsonrpc":"2.0","method":"evm_increaseTime","params":[864010]}' http://localhost:8545

## For Testnet
machine:
	docker buildx bake -f docker-bake.hcl -f docker-bake.override.hcl machine --load --set *.args.NETWORK=${NETWORK}

server:
	docker buildx bake -f docker-bake.hcl -f docker-bake.override.hcl server --load --set *.args.NETWORK=${NETWORK}

#Need to setup env vars beforehand: NETWORK=polygon_mumbai, MNEMONIC=<user sequence of twelve words>, RPC_URL=https://polygon-mumbai.g.alchemy.com/v2/<USER_KEY>
deploy-dapp-testnet:
	. ./scripts/.env_prod ;\
	. ./scripts/.env.${NETWORK} ;\
	DAPP_NAME=${DAPP_NAME} docker compose -f ./deploy-testnet.yml up

#Need to setup env vars beforehand: NETWORK=polygon_mumbai, MNEMONIC=<user sequence of twelve words>, RPC_URL=https://polygon-mumbai.g.alchemy.com/v2/<USER_KEY>
deploy-dapp-testnet-cartesi:
	. ./scripts/.env_prod ;\
	. ./scripts/.env.${NETWORK} ;\
	DAPP_NAME=${DAPP_NAME} docker compose -f ./deploy-testnet-cartesi.yml up

post-deploy-dapp-testnet:
	. ./scripts/.env_prod ;\
	. ./scripts/.env.${NETWORK} ;\
	DAPP_NAME=${DAPP_NAME} docker compose -f ./deploy-testnet.yml down -v

#Need to setup env vars beforehand: WSS_URL=wss://polygon-mumbai.g.alchemy.com/v2/<USER_KEY>, CHAIN_ID=80001
start-cartesi-validator:
	. ./scripts/.env_prod ;\
	. ./scripts/.env.${NETWORK} ;\
	DAPP_NAME=${DAPP_NAME} docker compose -f ./docker-compose-testnet.yml -f ./docker-compose.override.yml up

# #Need to setup env vars beforehand: WSS_URL=wss://polygon-mumbai.g.alchemy.com/v2/<USER_KEY>, CHAIN_ID=80001
# start-cartesi-validator-host-mode:
# 	DAPP_NAME=${DAPP_NAME} docker compose -f ./docker-compose-testnet.yml -f ./docker-compose.override.yml -f ./docker-compose-host.yml up