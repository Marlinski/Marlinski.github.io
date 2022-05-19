<script>
    import { web3, connected, selectedAccount } from "svelte-web3";
    import LoginWeb3 from "./LoginWeb3.svelte";
    import {
        tokenPrice,
        buildMintTx,
    } from "../utils/ContractApi.js";

    let txHash, txReceipt, txError;
    $: status = 0;

    const onMint = async () => {
        status = 1;
        buildMintTx($selectedAccount)
            .then((tx) => {
                $web3.eth
                    .sendTransaction(tx)
                    .once("transactionHash", (hash) => {
                        txHash = hash;
                        status = 2;
                    })
                    .once("receipt", (receipt) => {
                        txReceipt = receipt;
                        status = 3;
                    })
                    .on(
                        "confirmation",
                        (confNumber, receipt, latestBlockHash) => {
                            confirmedBlock = confNumber;
                        }
                    );
            })
            .then((receipt) => {
                console.log(txReceipt);
                mintedTokens = receipt.logs
                    .map((entry) =>
                        $web3.eth.abi.decodeLog(
                            [
                                {
                                    indexed: true,
                                    name: "from",
                                    type: "address",
                                },
                                {
                                    indexed: true,
                                    name: "to",
                                    type: "address",
                                },
                                {
                                    indexed: true,
                                    name: "tokenId",
                                    type: "uint256",
                                },
                            ],
                            entry.data,
                            entry.topics.slice(1)
                        )
                    )
                    .map((obj) => obj.tokenId);
                status = 4;
            })
            .catch((reason) => {
                txError = reason;
            });
    };
</script>

<div id="claim">
    {#if $connected && $selectedAccount}
        {#if status == 0 || status == 4}
            <div id="mintButton" on:click={onMint}>
                Acheter un token pour {$tokenPrice} ΞTH
            </div>
        {/if}
        <div id="status">
            {#if status == 1}
                <div>waiting for tx hash...</div>
            {:else if status == 2}
                <div>
                    transaction sent: <a href="https://rinkeby.etherscan.io/tx/{txHash}"
                        >{txHash}</a
                    >
                </div>
            {:else if status == 3}
                <div>
                    waiting for confirmation: <a
                        href="https://rinkeby.etherscan.io/tx/{txHash}">{txHash}</a
                    >
                </div>
            {:else if status == 4}
                <div>
                    transaction confirmed! <a
                        href="https://rinkeby.etherscan.io/tx/{txHash}">{txHash}</a
                    >
                </div>
                <div>
                    Congratulation! you successfully minted a token!
                </div>
            {:else if status == 5}
                <div>
                    tx failed :-(
                    {#if { txHash }}
                        <a href="https://etherscan.io/tx/{txHash}">{txHash}</a>
                    {/if}
                </div>
            {/if}
        </div>
    {:else}
        <LoginWeb3 />
    {/if}
</div>

<style>
    #claim {
        margin: 20px;
    }

    #mintButton {
        background-color: white;
        border: 1px solid grey;
        border-radius: 8px;
        font-weight: 400;
        cursor: pointer;
        width: fit-content;
        width: -moz-fit-content;
        margin: 0 auto;
        transition: 0.3s;
        padding: 20px;
        margin-bottom: 20px;
        border-color: rgb(32, 129, 226);
        color: rgb(32, 129, 226);
    }

    #mintButton:hover {
        background-color:rgb(32, 129, 226);
        color: white;
        transition: 0.3s;
        border: 1px solid white;
    }

    #status {
        margin: 0 auto;
        text-align: center;
    }
</style>
