<script>
    import {
        defaultChainStore,
        web3,
        selectedAccount
    } from "svelte-web3";
    import { updatedBalance } from "../utils/ContractApi.js";

    $: balance = $updatedBalance;

    const disconnect = () => defaultChainStore.close();
</script>

<div>
    {#if $selectedAccount}
        <div id="connected" on:click={disconnect}>
            <span>{parseFloat($web3.utils.fromWei(balance, "ether")).toFixed(3)} ΞTH</span>
            <div id="address">
                {$selectedAccount.slice(0, 6)}...{$selectedAccount.slice(
                    $selectedAccount.length - 4,
                    $selectedAccount.length
                ) || "not defined"}
            </div>
        </div>
    {/if}
</div>

<style>
    #connected,
    #address {
        display: flex;
        align-items: center;
        background-color: white;
        border: 1px solid grey;
        border-radius: 8px;
        font-weight: 400;
        cursor: pointer;
        transition: 0.3s;
    }

    #connected {
        background-color: #cfcfcf;
        border: 1px solid white;
        color: white;
        padding-left: 5px;
    }

    #address {
        margin-left: 5px;
        padding: 0px 5px;
        background-color: white;
        color: black;
    }

    #address:hover {
        background-color: slategrey;
        font-weight: 400;
        color: white;
        transition: 0.3s;
        border: 1px solid white;
    }
</style>
