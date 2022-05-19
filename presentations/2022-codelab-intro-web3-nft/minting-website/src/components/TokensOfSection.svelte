<script>
    import { balanceOfConnectedAccount, nftAddress } from "../utils/ContractApi.js";
</script>

{#if $balanceOfConnectedAccount > 0}
<div>
    <div id="tokensOfSection">
        Vous possédez actuellement {$balanceOfConnectedAccount} token
    </div>
    <div id="etherscan">
        voir le contrat sur <a href="https://rinkeby.etherscan.io/address/{nftAddress}">etherscan</a>
    </div>
</div>
{/if}

<style>
    #tokensOfSection {
        border: 1px solid #ddd;
        border-radius: 5px;
        margin: 0 auto;
        width: fit-content;
        padding: 10px;
    }

    #etherscan {
        padding-top: 20px;
        margin: 0 auto;
        width: fit-content;
    }
</style>
