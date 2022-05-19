<script>
    import { totalMinted, maxSupply } from "../utils/ContractApi.js";

    $: remainingToMint = $maxSupply - $totalMinted;
</script>

<div id="minted">
    {#if remainingToMint == 0}
        La vente est terminé !
    {:else}
        La vente est ouverte, plus que {remainingToMint} tokens de disponible !
    {/if}
</div>

<style>
    #minted {
        font-size: 18px;
        margin-bottom: 20px;
    }
</style>
